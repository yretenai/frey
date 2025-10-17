// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt::{Display, Formatter};

use anyhow::{Result, bail};
use log::debug;

use crate::engine::r#unsafe::game_module::{GameModuleKey, GameModuleMap};
use crate::engine::r#unsafe::map::LuminousStaticMap;
use crate::engine::{LuminousCString, LuminousGame, LuminousPointer};
use crate::memory::{MemoryCursor, MemoryReader};

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct GameModuleVTable {
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::CompactPfx>"))]
	pub alloc: u64,
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::CompactPfx>"))]
	pub get_name: u64,
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::CompactPfx>"))]
	pub get_dependencies: u64,
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::CompactPfx>"))]
	pub init: u64,
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::CompactPfx>"))]
	pub tick: u64,
}

type PrivateVTable = crate::engine::r#unsafe::game_module::GameModuleVTable;

impl GameModuleVTable {
	pub fn new(reader: &mut MemoryReader, dto: PrivateVTable) -> Self {
		let base = reader.get_base_address();

		Self {
			alloc: dto.alloc.debase(base),
			get_name: dto.get_name.debase(base),
			get_dependencies: dto.get_dependencies.debase(base),
			init: dto.init.debase(base),
			tick: dto.tick.debase(base),
		}
	}
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct GameModuleInfo {
	pub vtable: GameModuleVTable,
	pub name: String,
	pub dependencies: Vec<String>,

	#[cfg_attr(feature = "serde", serde(skip_serializing))]
	pub stable_ptr: LuminousPointer<()>,
}

impl GameModuleInfo {
	pub fn new(reader: &mut MemoryCursor, instance: LuminousPointer<()>) -> Result<Self> {
		let vtable = instance.cast::<LuminousPointer<PrivateVTable>>().read(&mut reader.inner)?.read(&mut reader.inner)?;
		let name_func = vtable.get_name;
		let dependencies_func = vtable.get_dependencies;

		let mut buffer = vec![0u8; 0x100];
		reader.inner.read(name_func, &mut buffer)?;
		let name_addr = LuminousCString::new(Self::sim_lea_ret(vtable.get_name.inner, &buffer)?);
		buffer.fill(0);
		reader.inner.read(dependencies_func, &mut buffer)?;
		let mut deps_addr: LuminousPointer<LuminousCString> =
			Self::sim_lea_ret(vtable.get_dependencies.inner, &buffer).unwrap_or_default().cast();

		let name = name_addr.read(reader).unwrap_or_default();
		let mut deps = Vec::new();
		if deps_addr.is_valid() {
			loop {
				let outer = deps_addr.read(&mut reader.inner)?;
				if !outer.is_valid() {
					break;
				}

				deps.push(outer.read(reader).unwrap_or_default());
				deps_addr += size_of::<LuminousCString>();
			}
		}

		Ok(GameModuleInfo {
			vtable: GameModuleVTable::new(&mut reader.inner, vtable),
			name,
			dependencies: deps,
			stable_ptr: instance,
		})
	}

	fn sim_lea_ret(rip: u64, buffer: &[u8]) -> Result<LuminousPointer<()>> {
		use iced_x86::*;
		let mut decoder = Decoder::with_ip(64, buffer, rip, DecoderOptions::NONE);

		let mut rax: u64 = 0;
		let mut instruction = Instruction::default();
		while decoder.can_decode() {
			decoder.decode_out(&mut instruction);

			if instruction.code().mnemonic() == Mnemonic::Lea && instruction.op1_kind() == OpKind::Memory {
				rax = instruction.memory_displacement64();
			} else if instruction.code().mnemonic() == Mnemonic::Ret {
				break;
			}
		}

		if rax == 0 {
			bail!("cannot find rax value at return")
		}

		Ok(rax.into())
	}
}

impl Display for GameModuleInfo {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.name)
	}
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct GameModules {
	pub pending: HashMap<u32, GameModuleInfo>,
	pub modules: HashMap<u32, GameModuleInfo>,
}

impl GameModules {
	pub fn new(reader: &mut MemoryCursor) -> Result<Self> {
		if reader.inner.game_type() != LuminousGame::FORSPOKEN {
			bail!("only on forspoken");
		}

		let modules_addr: LuminousPointer<LuminousPointer<GameModuleMap>> =
			(reader.inner.get_base_address() + super::GAME_FRAMEWORK_MODULE_MAP_ADDR_FORSPOKEN).cast();
		debug!("game framework module registry address: {:?}", modules_addr);
		let module_map = modules_addr.read(&mut reader.inner)?.read(&mut reader.inner)?;

		let pending = Self::convert_map(reader, module_map.pending_initialize)?;
		let modules = Self::convert_map(reader, module_map.modules)?;

		Ok(GameModules {
			pending,
			modules,
		})
	}

	fn convert_map(
		reader: &mut MemoryCursor,
		map: LuminousStaticMap<GameModuleKey, LuminousPointer<()>>,
	) -> Result<HashMap<u32, GameModuleInfo>> {
		let map = map.read(&mut reader.inner)?;

		let mut hashmap = HashMap::new();

		for (k, v) in map {
			if let Entry::Vacant(e) = hashmap.entry(k.key) {
				e.insert(GameModuleInfo::new(reader, v)?);
			}
		}

		Ok(hashmap)
	}
}
