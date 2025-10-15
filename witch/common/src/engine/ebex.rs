// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::collections::HashMap;

use anyhow::Result;
use int_enum::IntEnum;
use log::{debug, error, info, warn};

use crate::engine::r#unsafe::ebex::{
	ObjectFunctionTypeData, ObjectInfoProperty, ObjectInfoPropertyContainer, ObjectInfoPropertyPair, ObjectType, ObjectTypeXV,
};
use crate::engine::{LuminousCString, LuminousGame, LuminousPointer};
use crate::hash::fnv1a64;
use crate::memory::{MemoryCursor, MemoryReader};

#[derive(Debug, Copy, Clone, Default, IntEnum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ObjectFunctionFlag {
	#[default]
	Unknown = 0x0,
	Static = 0x1,
	Object = 0x2,
}

#[derive(Debug, Copy, Clone, Default, IntEnum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ObjectFunctionTypeFlag {
	#[default]
	None = 0x0,
	Const = 0x1,
	Pointer = 0x2,
	Reference = 0x4,
	Void = 0x8,
}

#[derive(Debug, Copy, Clone, Default, IntEnum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ObjectInfoAttributeFlag {
	#[default]
	None = 0x0,
	Pointer = 0x1,
	Reference = 0x2,
	DynamicArray = 0x4,
}

#[derive(Debug, Copy, Clone, Default, IntEnum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ObjectInfoPrimitiveType {
	#[default]
	Unknown = 0x0,
	ClassField = 0x1,
	Int8 = 0x2,
	Int16 = 0x3,
	Int32 = 0x4,
	Int64 = 0x5,
	UInt8 = 0x6,
	UInt16 = 0x7,
	UInt32 = 0x8,
	UInt64 = 0x9,
	SizeT = 0xa,
	Bool = 0xb,
	Float = 0xc,
	Double = 0xd,
	String = 0xe,
	Pointer = 0xf,
	Reference = 0x10,
	Array = 0x11,
	PointerArray = 0x12,
	Fixid = 0x13,
	Float4 = 0x14,
	Color = 0x15,
	Buffer = 0x16,
	Enum = 0x17,
	IntrusivePointerArray = 0x18,
	Double4 = 0x19,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ObjectClassFunctions {
	pub construct: usize,
	pub construct_inherited: usize,
	pub singleton: usize,
}

impl ObjectClassFunctions {
	pub fn new_xv(reader: &mut MemoryCursor, dto: ObjectTypeXV) -> Self {
		let base = reader.inner.get_base_address();

		ObjectClassFunctions {
			construct: dto.func1.debase(base),
			construct_inherited: dto.func2.debase(base),
			singleton: dto.func3.debase(base),
		}
	}

	pub fn new(reader: &mut MemoryCursor, dto: ObjectType) -> Self {
		let base = reader.inner.get_base_address();

		ObjectClassFunctions {
			construct: dto.func1.debase(base),
			construct_inherited: dto.func2.debase(base),
			singleton: dto.func3.debase(base),
		}
	}
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ObjectProperty {
	pub name: String,
	pub hash_code: u32,
	pub type_name: String,
	pub offset: usize,
	pub size: usize,
	pub item_count: usize,
	pub primitive_type: ObjectInfoPrimitiveType,
	pub item_primitive_type: ObjectInfoPrimitiveType,
	pub attributes: ObjectInfoAttributeFlag,
}

impl ObjectProperty {
	pub fn new(reader: &mut MemoryCursor, dto: ObjectInfoProperty) -> Self {
		ObjectProperty {
			name: dto.name.as_string(&mut reader.inner),
			hash_code: dto.hash_code,
			type_name: dto.type_name.as_string(&mut reader.inner),
			offset: dto.offset as usize,
			size: dto.size as usize,
			item_count: dto.item_count as usize,
			primitive_type: ObjectInfoPrimitiveType::try_from(dto.primitive_type as isize).unwrap_or_default(),
			item_primitive_type: ObjectInfoPrimitiveType::try_from(dto.item_primitive_type as isize).unwrap_or_default(),
			attributes: ObjectInfoAttributeFlag::try_from(dto.attributes as isize).unwrap_or_default(),
		}
	}
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ObjectFunctionType {
	pub primitive_type: ObjectInfoPrimitiveType,
	pub type_flag: ObjectFunctionTypeFlag,
	pub type_name_hash: u32,
	pub type_name: String,
	pub item_primitive_type: ObjectInfoPrimitiveType,
	pub item_type_flag: ObjectFunctionTypeFlag,
	pub item_type_name_hash: u32,
	pub item_type_name: Option<String>,
}

impl ObjectFunctionType {
	pub fn new(reader: &mut MemoryCursor, dto: ObjectFunctionTypeData) -> Result<Self> {
		Ok(Self {
			primitive_type: ObjectInfoPrimitiveType::try_from(dto.primitive_type as isize).unwrap_or_default(),
			type_flag: ObjectFunctionTypeFlag::try_from(dto.type_flag as isize).unwrap_or_default(),
			type_name_hash: dto.type_name_hash,
			type_name: dto.type_name.read(reader).unwrap(),
			item_primitive_type: ObjectInfoPrimitiveType::try_from(dto.item_primitive_type as isize).unwrap_or_default(),
			item_type_flag: ObjectFunctionTypeFlag::try_from(dto.item_type_flag as isize).unwrap_or_default(),
			item_type_name_hash: dto.item_type_name_hash,
			item_type_name: dto.item_type_name.read(reader),
		})
	}
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ObjectFunction {
	pub name: String,
	pub flags: ObjectFunctionFlag,
	pub function: usize,
	pub function_dynamic: usize,
	pub return_type: ObjectFunctionType,
	pub argument_types: Vec<ObjectFunctionType>,
}

impl ObjectFunction {
	pub fn new(reader: &mut MemoryCursor, dto: crate::engine::r#unsafe::ebex::ObjectFunction) -> Result<Self> {
		let base = reader.inner.get_base_address();

		let return_type = ObjectFunctionType::new(reader, dto.return_type)?;
		let mut argument_types_ptr = dto.argument_types;
		let mut argument_types: Vec<ObjectFunctionType> = vec![Default::default(); dto.argument_count as usize];
		for item in argument_types.iter_mut().take(dto.argument_count as usize) {
			let item_dto = argument_types_ptr.read(&mut reader.inner)?;
			*item = ObjectFunctionType::new(reader, item_dto)?;
			argument_types_ptr += size_of::<ObjectFunctionTypeData>();
		}

		Ok(Self {
			name: dto.name.read(reader).unwrap(),
			flags: ObjectFunctionFlag::try_from(dto.flags as isize).unwrap_or_default(),
			function: dto.function.debase(base),
			function_dynamic: dto.function_dynamic.debase(base),
			return_type,
			argument_types,
		})
	}
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ObjectInfoProperties {
	pub type_name: String,
	pub type_id: u64,
	pub base_type_id: u64,
	pub hash_code: u32,
	pub version_hash_code: u32,
	pub all_properties_class_field_count: usize,
	pub properties: HashMap<u32, ObjectProperty>,
}

impl ObjectInfoProperties {
	pub fn new(reader: &mut MemoryCursor, dto: ObjectInfoPropertyContainer) -> Result<Self> {
		let mut properties = HashMap::new();
		if dto.my_properties.size > 0 {
			let properties_unsafe: Vec<ObjectInfoProperty> = dto.my_properties.read(&mut reader.inner)?;

			for item in properties_unsafe {
				let property = ObjectProperty::new(reader, item);
				properties.insert(property.hash_code, property);
			}
		}

		let type_name = dto.type_name.as_string(&mut reader.inner);
		let type_id = fnv1a64(type_name.as_bytes());

		let base_type_id = ObjectInfo::read_base_type_id(reader, dto.parent_properties.cast() + 16).unwrap_or_default();

		Ok(Self {
			type_name,
			type_id,
			base_type_id,
			hash_code: dto.hash_code,
			version_hash_code: dto.version_hash_code,
			all_properties_class_field_count: dto.all_properties_class_field_count as usize,
			properties,
		})
	}
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ObjectInfo {
	pub name: String,
	pub type_id: u64,
	pub base_type_id: u64,
	pub this_id: u32,
	pub class_functions: ObjectClassFunctions,
	pub size: usize,
	pub properties: ObjectInfoProperties,
	pub functions: HashMap<u64, ObjectFunction>,
}

impl ObjectInfo {
	pub fn new(reader: &mut MemoryCursor, dto: ObjectType) -> Result<Self> {
		let name = dto.name.read(reader).unwrap();
		let type_id = fnv1a64(name.as_bytes());
		let base_type_id = Self::read_base_type_id(reader, dto.base_type.cast()).unwrap_or_default();

		let class_functions = ObjectClassFunctions::new(reader, dto);
		let properties = Self::read_properties(reader, dto.property_container)?;
		let functions = Self::read_object_functions(reader, dto.functions, dto.function_count as usize)?;

		if dto.unknown != 0 {
			debug!("encountered non-zero unknown for {:?}", dto.name);
		}

		Ok(Self {
			name,
			type_id,
			this_id: dto.this_type,
			base_type_id,
			class_functions,
			size: dto.size as usize,
			properties,
			functions,
		})
	}

	pub fn new_xv(reader: &mut MemoryCursor, dto: ObjectTypeXV) -> Result<Self> {
		let name = dto.name.read(reader).unwrap();
		let type_id = fnv1a64(name.as_bytes());
		let base_type_id = Self::read_base_type_id(reader, dto.base_type.cast()).unwrap_or_default();

		let class_functions = ObjectClassFunctions::new_xv(reader, dto);
		let properties = Self::read_properties(reader, dto.property_container)?;
		let functions = Self::read_object_functions(reader, dto.functions, dto.function_count as usize)?;

		Ok(Self {
			name: name.to_string(),
			type_id,
			this_id: dto.this_type,
			base_type_id,
			class_functions,
			size: dto.size as usize,
			properties,
			functions,
		})
	}

	fn read_base_type_id(reader: &mut MemoryCursor, pointer: LuminousPointer<LuminousCString>) -> Result<u64> {
		if !pointer.is_valid() {
			return Ok(0);
		}

		let name_pointer = pointer.read(&mut reader.inner)?;
		Ok(fnv1a64(name_pointer.read(reader).unwrap().as_bytes()))
	}

	fn read_object_functions(
		reader: &mut MemoryCursor,
		mut functions_ptr: LuminousPointer<crate::engine::r#unsafe::ebex::ObjectFunction>,
		function_count: usize,
	) -> Result<HashMap<u64, ObjectFunction>> {
		let mut functions: HashMap<u64, ObjectFunction> = HashMap::new();
		let size = size_of::<crate::engine::r#unsafe::ebex::ObjectFunction>();
		for _i in 0..function_count {
			let item_dto = functions_ptr.read(&mut reader.inner)?;
			let item = ObjectFunction::new(reader, item_dto)?;
			functions.insert(fnv1a64(item.name.as_bytes()), item);
			functions_ptr += size;
		}
		Ok(functions)
	}

	fn read_properties(
		reader: &mut MemoryCursor,
		property_container: LuminousPointer<ObjectInfoPropertyContainer>,
	) -> Result<ObjectInfoProperties> {
		Ok(match property_container.is_valid() {
			true => {
				let properties_dto = property_container.read(&mut reader.inner)?;
				ObjectInfoProperties::new(reader, properties_dto)?
			}
			false => Default::default(),
		})
	}
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ObjectInfoRegistry {
	pub elements: HashMap<u64, ObjectInfo>,
}

impl ObjectInfoRegistry {
	pub fn new(reader: &mut MemoryCursor) -> Result<Self> {
		let mut elements = HashMap::new();
		let is_xv = reader.inner.game_type() != LuminousGame::FORSPOKEN;
		let mut registry_ptr: LuminousPointer<ObjectInfoPropertyPair> = reader.inner.get_base_address().cast()
			+ match reader.inner.game_type() {
				LuminousGame::FORSPOKEN => super::EBEX_OBJECT_ARRAY_ADDR_FORSPOKEN,
				_ => super::EBEX_OBJECT_ARRAY_ADDR_XV,
			};

		for i in 0..0x20 {
			info!("reading ebex array slice {}/32", i);
			for j in 0..0x1000 {
				debug!("reading ebex array entry {}/131072", i * 0x1000 + j);

				let item_dto = registry_ptr.read(&mut reader.inner)?;
				registry_ptr += size_of::<ObjectInfoPropertyPair>();
				if item_dto.key == 0 || !item_dto.value.is_valid() {
					continue;
				}

				match match is_xv {
					true => {
						let xv: LuminousPointer<ObjectTypeXV> = item_dto.value.cast();
						xv.read(&mut reader.inner).and_then(|dto| ObjectInfo::new_xv(reader, dto))
					}
					false => item_dto.value.read(&mut reader.inner).and_then(|dto| ObjectInfo::new(reader, dto)),
				} {
					Ok(item) => {
						debug!("read entry {:?}", item);
						elements.insert(item.type_id, item);
					}
					Err(err) => {
						error!("error reading object info: {}", err);
					}
				}
			}
		}

		debug!("sanity checking elements...");
		for element in elements.values() {
			if element.base_type_id != 0 && !elements.contains_key(&element.base_type_id) {
				warn!("missing base type {:#016x} for element {}", element.base_type_id, element.name);
			}

			if element.properties.base_type_id != 0 && !elements.contains_key(&element.properties.base_type_id) {
				warn!(
					"missing properties base type {:#016x} for element {}",
					element.properties.base_type_id, element.properties.type_name
				);
			}
		}

		info!("loaded {} object infos", elements.len());

		Ok(Self {
			elements,
		})
	}
}
