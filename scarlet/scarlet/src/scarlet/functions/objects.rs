// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use anyhow::bail;
use bytemuck::{Pod, Zeroable};
use witch_common::engine::ebex::{ObjectInfo, ObjectInfoRegistry};
use witch_common::engine::r#unsafe::ebex::ObjectType;
use witch_common::engine::r#unsafe::map::LuminousStaticMap;
use witch_common::engine::r#unsafe::mutex::LuminousGameMutex;
use witch_common::engine::{LuminousCString, LuminousDynamicArray, LuminousPointer, LuminousString};
use witch_common::memory::windows_local_mem::Win32LocalMemoryReader;
use witch_common::memory::{MemoryCursor, MemoryReader};

use super::{call_vtable_0, find_ebex_function};
use crate::scarlet::functions::EbexFunc;

#[derive(Default)]
pub struct ScarletObjects {
	pub get_entity_manager_module: EbexFunc<LuminousPointer<()>>,
	pub get_entity_manager: EbexFunc<LuminousPointer<()>>,

	pub _activate_gameobj_impl: EbexFunc<bool>,
	pub _deactivate_gameobj_impl: EbexFunc<bool>,
	pub _is_active_gameobj_impl: EbexFunc<bool>,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct ScarletGameComponentDto {
	pub map: LuminousStaticMap<u32, LuminousPointer<()>>,
	pub components: LuminousDynamicArray<()>,
	pub owner: LuminousPointer<()>,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct ScarletGameObjectDto {
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
	pub components: LuminousPointer<ScarletGameComponentDto>,
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
	pub entities: LuminousPointer<LuminousDynamicArray<()>>,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ScarletEntityPackage {
	pub inner: ScarletEntityGroup,
	pub objects: LuminousPointer<LuminousDynamicArray<()>>,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum ScarletObject {
	Package(ScarletEntityPackage),
	Group(ScarletEntityGroup),
	GameObject(ScarletGameObject),
	BaseObject(ScarletBaseObject),
}

#[allow(unused)]
impl ScarletObject {
	pub fn name(&self) -> Option<String> {
		match self {
			ScarletObject::Package(package) => package.inner.inner.inner.name.clone(),
			ScarletObject::Group(group) => group.inner.inner.name.clone(),
			ScarletObject::GameObject(game_object) => game_object.inner.name.clone(),
			ScarletObject::BaseObject(object) => object.name.clone(),
		}
	}

	pub fn type_info(&self) -> ObjectInfo {
		match self {
			ScarletObject::Package(package) => package.inner.inner.inner.object_type.clone(),
			ScarletObject::Group(group) => group.inner.inner.object_type.clone(),
			ScarletObject::GameObject(game_object) => game_object.inner.object_type.clone(),
			ScarletObject::BaseObject(object) => object.object_type.clone(),
		}
	}

	pub fn package(&self) -> Option<&ScarletEntityPackage> {
		match self {
			ScarletObject::Package(package) => Some(package),
			_ => None,
		}
	}

	pub fn group(&self) -> Option<&ScarletEntityGroup> {
		match self {
			ScarletObject::Package(package) => Some(&package.inner),
			ScarletObject::Group(group) => Some(group),
			_ => None,
		}
	}

	pub fn game_object(&self) -> Option<&ScarletGameObject> {
		match self {
			ScarletObject::Package(package) => Some(&package.inner.inner),
			ScarletObject::Group(group) => Some(&group.inner),
			ScarletObject::GameObject(game_object) => Some(game_object),
			_ => None,
		}
	}

	pub fn object(&self) -> Option<&ScarletBaseObject> {
		match self {
			ScarletObject::Package(package) => Some(&package.inner.inner.inner),
			ScarletObject::Group(group) => Some(&group.inner.inner),
			ScarletObject::GameObject(game_object) => Some(&game_object.inner),
			ScarletObject::BaseObject(object) => Some(object),
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub enum ScarletObjectType {
	Package,
	Group,
	GameObject,
	BaseObject,
}

// todo: make these ::new()
impl ScarletObjects {
	pub fn new(base: LuminousPointer<()>, ebex: &ObjectInfoRegistry) -> Option<Self> {
		Some(ScarletObjects {
			get_entity_manager_module: find_ebex_function(base, ebex, "Luminous.EntitySystem.EntityManagerModule", "GetInsntance")?, /* NOTE: this is typo'd in the game, not my fault! */
			get_entity_manager: find_ebex_function(base, ebex, "Luminous.EntitySystem.EntityManagerModule", "GetEntityManager")?,
			_activate_gameobj_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.GameObject", "Activate")?,
			_deactivate_gameobj_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.GameObject", "Inactivate")?,
			_is_active_gameobj_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.GameObject", "IsActive")?,
		})
	}

	pub fn get_packages(&self, ebex: &ObjectInfoRegistry) -> anyhow::Result<Vec<ScarletObject>> {
		let entity_manager_module = self.get_entity_manager_module.call(None, None);
		if !entity_manager_module.is_valid() {
			bail!("entity manager module is not initialized");
		}

		let entity_manager = self.get_entity_manager.call(Some(entity_manager_module), None);
		if !entity_manager.is_valid() {
			bail!("entity manager is not initialized");
		}

		let mut reader = MemoryCursor::new(MemoryReader::Process(Win32LocalMemoryReader::new(true)));
		let mut result = Vec::new();

		// self.load_packages(entity_manager.cast() + 0x8, &mut reader, &mut result, ebex)?;
		self.load_packages(entity_manager.cast() + 0x48, &mut reader, &mut result, ebex)?;

		Ok(result)
	}

	fn load_packages(
		&self,
		base: LuminousPointer<()>,
		reader: &mut MemoryCursor,
		result: &mut Vec<ScarletObject>,
		ebex: &ObjectInfoRegistry,
	) -> anyhow::Result<()> {
		let mut mutex = LuminousGameMutex::new(base.cast() + 8);

		if !mutex.mutex.is_valid() {
			bail!("mutex not initialized");
		}

		if unsafe { *mutex.mutex.unsafe_ptr() }.SpinCount != 0x64 {
			bail!("mutex not 0x64");
		}

		mutex.try_lock()?;
		let load_result = self.load_packages_inner(base.cast() + 0x30, reader, result, ebex);
		mutex.unlock();
		load_result
	}

	fn load_packages_inner(
		&self,
		base: LuminousPointer<LuminousDynamicArray<()>>,
		reader: &mut MemoryCursor,
		result: &mut Vec<ScarletObject>,
		ebex: &ObjectInfoRegistry,
	) -> anyhow::Result<()> {
		if !base.is_valid() {
			bail!("invalid pointer");
		}

		let array = base.read(&mut reader.inner)?;
		if !array.is_valid() {
			bail!("invalid array");
		}

		let array = array.read_ptrs(&mut reader.inner)?;

		for ptr in array {
			if !ptr.is_valid() {
				continue;
			}

			result.push(self.read_object(ptr, reader, ebex)?);
		}

		Ok(())
	}

	#[allow(unused)]
	pub fn read_objects(
		&self,
		base: LuminousPointer<LuminousDynamicArray<()>>,
		reader: &mut MemoryCursor,
		ebex: &ObjectInfoRegistry,
	) -> anyhow::Result<Vec<ScarletObject>> {
		if !base.is_valid() {
			return Ok(Default::default());
		}

		let mut result = Vec::new();
		let array = base.read(&mut reader.inner)?;

		if !array.is_valid() || array.is_empty() {
			return Ok(Default::default());
		}

		for ptr in array.read_ptrs(&mut reader.inner)? {
			if !ptr.is_valid() {
				continue;
			}

			result.push(self.read_object(ptr, reader, ebex)?);
		}

		Ok(result)
	}

	fn read_object(
		&self,
		base: LuminousPointer<()>,
		reader: &mut MemoryCursor,
		ebex: &ObjectInfoRegistry,
	) -> anyhow::Result<ScarletObject> {
		let type_info = Self::get_object_type(base, reader)?;
		let mut type_name = type_info.name.clone();
		let mut base_type = type_info.base_type;

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
				object_type = ScarletObjectType::GameObject;
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

		Ok(match object_type {
			ScarletObjectType::Package => ScarletObject::Package(self.read_entity_package(base, reader)?),
			ScarletObjectType::Group => ScarletObject::Group(self.read_entity_group(base, reader)?),
			ScarletObjectType::GameObject => ScarletObject::GameObject(self.read_game_object(base, reader)?),
			ScarletObjectType::BaseObject => ScarletObject::BaseObject(self.read_base_object(base, reader)?),
		})
	}

	fn read_base_object(&self, base: LuminousPointer<()>, reader: &mut MemoryCursor) -> anyhow::Result<ScarletBaseObject> {
		Ok(ScarletBaseObject {
			address: base,
			name: Self::get_object_name(base, reader).ok(),
			object_type: Self::get_object_type(base, reader)?,
		})
	}

	fn read_game_object(&self, base: LuminousPointer<()>, reader: &mut MemoryCursor) -> anyhow::Result<ScarletGameObject> {
		let inner = self.read_base_object(base, reader)?;
		let dto = base.cast::<ScarletGameObjectDto>().read(&mut reader.inner)?;

		Ok(ScarletGameObject {
			inner,
			flags: dto.flags,
			class_flags: dto.class_flags,
			object_flags: dto.object_flags,
			components: dto.components,
		})
	}

	fn read_entity_group(&self, base: LuminousPointer<()>, reader: &mut MemoryCursor) -> anyhow::Result<ScarletEntityGroup> {
		let inner = self.read_game_object(base, reader)?;

		let entities: LuminousPointer<LuminousDynamicArray<()>> = base.cast() + 0x70;
		// let transform_component: LuminousPointer<()> = base + 0xa0
		// let has_transform: LuminousPointer<bool> = base.cast() + 0xa8;
		// let scale: LuminousPointer<f32> = base.cast() + 0xb0;
		let source_name: LuminousPointer<LuminousString> = base.cast() + 0xe0;
		let name: LuminousPointer<LuminousString> = base.cast() + 0x118;

		let source_name = source_name.read(&mut reader.inner)?.as_some_string(&mut reader.inner);
		let name = name.read(&mut reader.inner)?.as_some_string(&mut reader.inner);
		// self.read_objects(entities, reader, ebex)?;

		Ok(ScarletEntityGroup {
			inner,
			name,
			source_name,
			entities,
		})
	}

	fn read_entity_package(&self, base: LuminousPointer<()>, reader: &mut MemoryCursor) -> anyhow::Result<ScarletEntityPackage> {
		let inner = self.read_entity_group(base, reader)?;

		let objects: LuminousPointer<LuminousDynamicArray<()>> = base.cast() + 0x148;

		Ok(ScarletEntityPackage {
			inner,
			objects,
		})
	}

	pub fn get_object_type(entity: LuminousPointer<()>, reader: &mut MemoryCursor) -> anyhow::Result<ObjectInfo> {
		if !entity.is_valid() {
			bail!("pointer is not an object pointer");
		}

		let ptr = unsafe { call_vtable_0::<ObjectType>(entity, 1) };
		let type_info = ptr.read(&mut reader.inner)?;
		ObjectInfo::new(reader, type_info)
	}

	pub fn get_object_name(entity: LuminousPointer<()>, reader: &mut MemoryCursor) -> anyhow::Result<String> {
		if !entity.is_valid() {
			bail!("pointer is not an object pointer");
		}

		let ptr = unsafe { call_vtable_0::<i8>(entity, 9) };
		let cstring = LuminousCString::new(ptr.cast());
		Ok(cstring.read(reader).unwrap_or("(no name)".to_string()))
	}
}
