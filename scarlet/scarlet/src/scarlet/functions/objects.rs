// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use bytemuck::{Pod, Zeroable};
use witch_common::engine::ebex::{ObjectInfo, ObjectInfoRegistry};
use witch_common::engine::r#unsafe::map::LuminousStaticMap;
use witch_common::engine::r#unsafe::mutex::LuminousGameMutex;
use witch_common::engine::{LuminousDynamicArray, LuminousPointer, LuminousString};
use witch_common::memory::windows_local_mem::Win32LocalMemoryReader;
use witch_common::memory::{MemoryCursor, MemoryReader};

use super::find_ebex_function;
use crate::scarlet::functions::{EbexFunc, EbexGetName, EbexGetType};

#[derive(Default)]
pub struct ScarletObjects {
	pub get_entity_manager_module: EbexFunc<LuminousPointer<()>>,
	pub get_entity_manager: EbexFunc<LuminousPointer<()>>,

	pub _activate_gameobj_impl: EbexFunc<bool>,
	pub _deactivate_gameobj_impl: EbexFunc<bool>,
	pub _is_active_gameobj_impl: EbexFunc<bool>,

	inheritance_chain: HashMap<u32, ScarletObjectType>,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct ScarletGameComponentDto {
	pub map: LuminousPointer<LuminousStaticMap<u32, LuminousPointer<()>>>,
	pub components: LuminousPointer<LuminousDynamicArray<()>>,
	pub owner: LuminousPointer<()>,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct ScarletGameObjectDto {
	pub vtable: LuminousPointer<()>,
	pub flags: u32,
	pub class_flags: u32,
	pub object_flags: u64,
	pub components: LuminousPointer<ScarletGameComponentDto>,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ScarletBaseObject {
	pub address: LuminousPointer<()>,
	pub object_type: ObjectInfo,
	pub name: Option<String>,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ScarletGameObject {
	pub inner: ScarletBaseObject,
	pub flags: u32,
	pub class_flags: u32,
	pub object_flags: u64,
	pub components: Option<Vec<ScarletObject>>,
}

impl ScarletGameObject {
	pub fn _activate_gameobj(&self, objects: ScarletObjects, gameobj: LuminousPointer<()>) -> bool {
		objects._activate_gameobj_impl.call(Some(gameobj), None)
	}

	pub fn _deactivate_gameobj(&self, objects: ScarletObjects, gameobj: LuminousPointer<()>) -> bool {
		objects._deactivate_gameobj_impl.call(Some(gameobj), None)
	}

	pub fn _gameobject_is_active(&self, objects: ScarletObjects, gameobj: LuminousPointer<()>) -> bool {
		objects._is_active_gameobj_impl.call(Some(gameobj), None)
	}
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ScarletEntityGroup {
	pub inner: ScarletGameObject,
	pub name: Option<String>,
	pub source_name: Option<String>,
	pub entities: Vec<ScarletObject>,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ScarletEntityPackage {
	pub inner: ScarletEntityGroup,
	pub objects: Vec<ScarletObject>,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum ScarletObject {
	Package(ScarletEntityPackage),
	Group(ScarletEntityGroup),
	Object(ScarletGameObject),
	BaseObject(ScarletBaseObject),
}

#[derive(Debug, Copy, Clone)]
pub enum ScarletObjectType {
	Package,
	Group,
	Object,
	BaseObject,
}

impl ScarletObjects {
	pub fn new(base: LuminousPointer<()>, ebex: &ObjectInfoRegistry) -> Option<Self> {
		Some(ScarletObjects {
			get_entity_manager_module: find_ebex_function(base, ebex, "Luminous.EntitySystem.EntityManagerModule", "GetInsntance")?, /* NOTE: this is typo'd in the game, not my fault! */
			get_entity_manager: find_ebex_function(base, ebex, "Luminous.EntitySystem.EntityManagerModule", "GetEntityManager")?,
			_activate_gameobj_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.GameObject", "Activate")?,
			_deactivate_gameobj_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.GameObject", "Inactivate")?,
			_is_active_gameobj_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.GameObject", "IsActive")?,
			..Default::default()
		})
	}

	pub fn get_packages(&mut self, ebex: &ObjectInfoRegistry) -> anyhow::Result<Vec<ScarletObject>> {
		let entity_manager_module = self.get_entity_manager_module.call(None, None);
		let entity_manager = self.get_entity_manager.call(Some(entity_manager_module), None);

		let mut reader = MemoryCursor::new(MemoryReader::Process(Win32LocalMemoryReader::new(true)));
		let mut result = Vec::new();

		self.load_packages(entity_manager.cast() + 0x8, &mut reader, &mut result, ebex)?;
		self.load_packages(entity_manager.cast() + 0x48, &mut reader, &mut result, ebex)?;

		Ok(result)
	}

	fn load_packages(
		&mut self,
		base: LuminousPointer<()>,
		reader: &mut MemoryCursor,
		result: &mut Vec<ScarletObject>,
		ebex: &ObjectInfoRegistry,
	) -> anyhow::Result<()> {
		let mut mutex = LuminousGameMutex::new(base.cast());
		mutex.try_lock()?;
		let load_result = self.load_packages_inner(base.cast() + 0x30, reader, result, ebex);
		mutex.unlock();
		load_result
	}

	fn load_packages_inner(
		&mut self,
		base: LuminousPointer<LuminousDynamicArray<()>>,
		reader: &mut MemoryCursor,
		result: &mut Vec<ScarletObject>,
		ebex: &ObjectInfoRegistry,
	) -> anyhow::Result<()> {
		let array = base.read(&mut reader.inner)?.read_ptrs(&mut reader.inner)?;

		for ptr in array {
			if !ptr.is_valid() {
				continue;
			}

			result.push(self.read_object(ptr, reader, ebex)?);
		}

		Ok(())
	}

	fn read_objects(
		&mut self,
		base: LuminousPointer<LuminousDynamicArray<()>>,
		reader: &mut MemoryCursor,
		ebex: &ObjectInfoRegistry,
	) -> anyhow::Result<Vec<ScarletObject>> {
		let mut result = Vec::new();

		for ptr in base.read(&mut reader.inner)?.read_ptrs(&mut reader.inner)? {
			if !ptr.is_valid() {
				continue;
			}

			result.push(self.read_object(ptr, reader, ebex)?);
		}

		Ok(result)
	}

	fn read_object(
		&mut self,
		base: LuminousPointer<()>,
		reader: &mut MemoryCursor,
		ebex: &ObjectInfoRegistry,
	) -> anyhow::Result<ScarletObject> {
		let type_info = Self::get_object_type(base, reader)?;
		let id = type_info.this_id;
		let mut type_name = type_info.name.clone();
		let mut base_type = type_info.base_type;

		let inheritance_type = match self.inheritance_chain.entry(id) {
			Entry::Occupied(e) => *e.get(),
			Entry::Vacant(e) => {
				let object_type: ScarletObjectType;
				loop {
					if type_name.eq("Luminous.EntitySystem.EntityPackage") {
						object_type = ScarletObjectType::Package;
						break;
					}

					if type_name.eq("Luminous.EntitySystem.EntityGroup") {
						object_type = ScarletObjectType::Group;
						break;
					}

					if type_name.eq("Luminous.GameFramework.GameObject") {
						object_type = ScarletObjectType::Object;
						break;
					}

					match base_type {
						None => {
							object_type = ScarletObjectType::BaseObject;
							break;
						}
						Some(base) => match ebex.elements.get(&base) {
							None => {
								object_type = ScarletObjectType::BaseObject;
								break;
							}
							Some(base) => {
								type_name = base.name.clone();
								base_type = base.base_type.clone();
							}
						},
					}
				}

				e.insert(object_type);
				object_type
			}
		};

		Ok(match inheritance_type {
			ScarletObjectType::Package => ScarletObject::Package(self.read_entity_package(base, reader, ebex)?),
			ScarletObjectType::Group => ScarletObject::Group(self.read_entity_group(base, reader, ebex)?),
			ScarletObjectType::Object => ScarletObject::Object(self.read_game_object(base, reader, ebex)?),
			ScarletObjectType::BaseObject => ScarletObject::BaseObject(self.read_base_object(base, reader)?),
		})
	}

	fn read_base_object(&mut self, base: LuminousPointer<()>, reader: &mut MemoryCursor) -> anyhow::Result<ScarletBaseObject> {
		Ok(ScarletBaseObject {
			address: base,
			name: Self::get_object_name(base, reader).ok(),
			object_type: Self::get_object_type(base, reader)?,
		})
	}

	fn read_game_object(
		&mut self,
		base: LuminousPointer<()>,
		reader: &mut MemoryCursor,
		ebex: &ObjectInfoRegistry,
	) -> anyhow::Result<ScarletGameObject> {
		let dto = base.cast::<ScarletGameObjectDto>().read(&mut reader.inner)?;

		Ok(ScarletGameObject {
			inner: self.read_base_object(base, reader)?,
			flags: dto.flags,
			class_flags: dto.class_flags,
			object_flags: dto.object_flags,
			components: self.read_objects(dto.components.read(&mut reader.inner)?.components, reader, ebex).ok(),
		})
	}

	fn read_entity_group(
		&mut self,
		base: LuminousPointer<()>,
		reader: &mut MemoryCursor,
		ebex: &ObjectInfoRegistry,
	) -> anyhow::Result<ScarletEntityGroup> {
		let inner = self.read_game_object(base, reader, ebex)?;

		let entities: LuminousPointer<LuminousDynamicArray<()>> = base.cast() + 0x70;
		// let transform_component: LuminousPointer<()> = base + 0xa0
		// let has_transform: LuminousPointer<bool> = base.cast() + 0xa8;
		// let scale: LuminousPointer<f32> = base.cast() + 0xb0;
		let source_name: LuminousPointer<LuminousString> = base.cast() + 0xe0;
		let name: LuminousPointer<LuminousString> = base.cast() + 0x118;

		let source_name = source_name.read(&mut reader.inner)?.as_some_string(&mut reader.inner);
		let name = name.read(&mut reader.inner)?.as_some_string(&mut reader.inner);
		let entities = self.read_objects(entities, reader, ebex)?;

		Ok(ScarletEntityGroup {
			inner,
			name,
			source_name,
			entities,
		})
	}

	fn read_entity_package(
		&mut self,
		base: LuminousPointer<()>,
		reader: &mut MemoryCursor,
		ebex: &ObjectInfoRegistry,
	) -> anyhow::Result<ScarletEntityPackage> {
		let inner = self.read_entity_group(base, reader, ebex)?;

		let objects: LuminousPointer<LuminousDynamicArray<()>> = base.cast() + 0x148;
		let objects = self.read_objects(objects, reader, ebex)?;

		Ok(ScarletEntityPackage {
			inner,
			objects,
		})
	}

	pub fn get_object_type(entity: LuminousPointer<()>, reader: &mut MemoryCursor) -> anyhow::Result<ObjectInfo> {
		let vtable = entity.cast::<LuminousPointer<()>>().read(&mut reader.inner)?; // void* -> void** (ptr to vtable)
		let get_type_info: LuminousPointer<()> = vtable + 8; // vtable entry 2
		let call = unsafe { std::mem::transmute::<u64, EbexGetType>(get_type_info.inner) };
		let type_info_ptr = unsafe { call(entity.cast().unsafe_ptr()) };
		let type_info = type_info_ptr.read(&mut reader.inner)?;
		ObjectInfo::new(reader, type_info)
	}

	pub fn get_object_name(entity: LuminousPointer<()>, reader: &mut MemoryCursor) -> anyhow::Result<String> {
		let vtable = entity.cast::<LuminousPointer<()>>().read(&mut reader.inner)?; // void* -> void** (ptr to vtable)
		let get_type_info: LuminousPointer<()> = vtable + 0x48; // vtable entry 10
		let call = unsafe { std::mem::transmute::<u64, EbexGetName>(get_type_info.inner) };
		let name_ptr = unsafe { call(entity.cast().unsafe_ptr()) };
		Ok(name_ptr.read(reader).unwrap_or("(no name)".to_string()))
	}
}
