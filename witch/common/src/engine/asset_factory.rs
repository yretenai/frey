// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use anyhow::Result;
use log::debug;

use crate::asset_id::AssetId;
use crate::engine::r#unsafe::asset_factory::{AssetFactoryContainer, AssetFactoryStatisticsHolder};
use crate::engine::{LuminousGame, LuminousPointer};
use crate::memory::MemoryCursor;

#[derive(Debug, Copy, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AssetFactoryFunctions {
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::CompactPfx>"))]
	pub load: u64,
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::CompactPfx>"))]
	pub alloc: u64,
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::CompactPfx>"))]
	pub init: u64,
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::CompactPfx>"))]
	pub from_fs: u64,
}

impl AssetFactoryFunctions {
	pub fn new(reader: &mut MemoryCursor, dto: crate::engine::r#unsafe::asset_factory::AssetFactory) -> Self {
		let base = reader.inner.get_base_address();

		AssetFactoryFunctions {
			load: dto.loader.cast::<LuminousPointer<()>>().read(&mut reader.inner).unwrap_or_default().debase(base),
			alloc: dto.alloc.cast::<LuminousPointer<()>>().read(&mut reader.inner).unwrap_or_default().debase(base),
			init: dto.init.cast::<LuminousPointer<()>>().read(&mut reader.inner).unwrap_or_default().debase(base),
			from_fs: dto.fs.cast::<LuminousPointer<()>>().read(&mut reader.inner).unwrap_or_default().debase(base),
		}
	}
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AssetFactoryStatistics {
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::CompactPfx>"))]
	pub current_size: u64,
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::CompactPfx>"))]
	pub peak_size: u64,
	pub current_count: usize,
	pub peak_count: usize,
}

impl AssetFactoryStatistics {
	pub fn new(dto: crate::engine::r#unsafe::asset_factory::AssetFactoryStatistics) -> Self {
		AssetFactoryStatistics {
			current_size: dto.current_size,
			peak_size: dto.peak_size,
			current_count: dto.current_count as usize,
			peak_count: dto.peak_count as usize,
		}
	}
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AssetFactoryHolder {
	pub functions: AssetFactoryFunctions,
	pub stats: AssetFactoryStatistics,
	pub name: String,
	pub asset_id: AssetId,
}

impl AssetFactoryHolder {
	pub fn new(reader: &mut MemoryCursor, dto: AssetFactoryStatisticsHolder, key: AssetId) -> Result<Self> {
		let functions_dto = dto.factory.read(&mut reader.inner)?;
		let functions = AssetFactoryFunctions::new(reader, functions_dto);
		let len = dto.name.iter().position(|&b| b == 0).unwrap_or(dto.name.len());
		let name = String::from_utf8_lossy(&dto.name[..len]).to_string();
		Ok(AssetFactoryHolder {
			functions,
			stats: AssetFactoryStatistics::new(dto.statistics),
			name,
			asset_id: key,
		})
	}
}

impl Display for AssetFactoryHolder {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.name)
	}
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AssetFactory {
	pub factories: HashMap<AssetId, AssetFactoryHolder>,
	pub default_factory: AssetFactoryFunctions,
}

impl AssetFactory {
	pub fn new(reader: &mut MemoryCursor) -> Result<Self> {
		let registry = match reader.inner.game_type() {
			LuminousGame::FORSPOKEN => {
				type Root = LuminousPointer<LuminousPointer<LuminousPointer<AssetFactoryContainer>>>;
				let ptr = Root::new(super::ASSET_FACTORY_CONTAINER_ADDR_FORSPOKEN) + reader.inner.get_base_address();
				debug!("asset factory registry address: {:?}", ptr);
				let ptr = ptr.read(&mut reader.inner)?; // Ptr<Ptr<Registry>>
				let ptr = ptr.read(&mut reader.inner)?; // Ptr<Registry>
				ptr + super::ASSET_FACTORY_CONTAINER_OFFS_FORSPOKEN
			}
			_ => {
				type Root = LuminousPointer<LuminousPointer<AssetFactoryContainer>>;
				let ptr = Root::new(super::ASSET_FACTORY_CONTAINER_ADDR_XV) + reader.inner.get_base_address();
				debug!("asset factory registry address: {:?}", ptr);
				let ptr = ptr.read(&mut reader.inner)?; // Ptr<Registry>
				ptr + super::ASSET_FACTORY_CONTAINER_OFFS_XV
			}
		}
		.read(&mut reader.inner)?;

		let elements = registry.factory_holder.read(&mut reader.inner)?;
		let mut factories = HashMap::new();

		for (k, v) in elements {
			factories.insert(k, AssetFactoryHolder::new(reader, v, k)?);
		}

		let default_factory = AssetFactoryFunctions::new(reader, registry.default_factory);

		factories.insert(
			Default::default(),
			AssetFactoryHolder {
				functions: default_factory,
				name: "default".to_string(),
				..Default::default()
			},
		);

		Ok(Self {
			factories,
			default_factory,
		})
	}
}
