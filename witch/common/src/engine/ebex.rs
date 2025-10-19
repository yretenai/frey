// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use anyhow::Result;
use bitflags::bitflags;
use int_enum::IntEnum;
use log::{debug, error, info, warn};

use crate::engine::r#unsafe::ebex::{
	EbexObjectCall, EbexObjectCallDynamic, ObjectFunctionTypeData, ObjectInfoProperty, ObjectInfoPropertyContainer, ObjectInfoPropertyPair,
	ObjectType, ObjectTypeXV,
};
use crate::engine::{LuminousCString, LuminousGame, LuminousPointer};
use crate::hash::fnv1a64;
use crate::memory::MemoryCursor;

bitflags! {
	#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
	#[repr(transparent)]
	#[cfg_attr(feature = "serde", derive(serde::Serialize))]
	pub struct ObjectFunctionFlag: u32 {
		const Unknown = 0u32;
		const Static = 1u32;
		const Object = 2u32;
	}
}

bitflags! {
	#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
	#[repr(transparent)]
	#[cfg_attr(feature = "serde", derive(serde::Serialize))]
	pub struct ObjectFunctionTypeFlag: u32 {
		const None = 0u32;
		const Const = 1u32;
		const Pointer = 2u32;
		const Reference = 4u32;
		const Void = 8u32;
	}
}

bitflags! {
	#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
	#[repr(transparent)]
	#[cfg_attr(feature = "serde", derive(serde::Serialize))]
	pub struct ObjectInfoAttributeFlag: u32 {
		const None = 0u32;
		const Pointer = 1u32;
		const Reference = 2u32;
		const DynamicArray = 4u32;
	}
}

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, IntEnum)]
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
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::CompactPfx>"))]
	pub construct: u64,
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::CompactPfx>"))]
	pub construct_inherited: u64,
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::CompactPfx>"))]
	pub singleton: u64,
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
	/// crc32(name)
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::StrictPfx>"))]
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
			attributes: ObjectInfoAttributeFlag::from_bits_retain(dto.attributes as u32),
		}
	}
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ObjectFunctionType {
	pub primitive_type: ObjectInfoPrimitiveType,
	pub type_flag: ObjectFunctionTypeFlag,
	/// fnv1a32(name)
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::StrictPfx>"))]
	pub type_name_hash: u32,
	pub type_name: String,
	pub item_primitive_type: ObjectInfoPrimitiveType,
	pub item_type_flag: ObjectFunctionTypeFlag,
	/// fnv1a32(name)
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::StrictPfx>"))]
	pub item_type_name_hash: u32,
	pub item_type_name: Option<String>,
}

impl ObjectFunctionType {
	pub fn new(reader: &mut MemoryCursor, dto: ObjectFunctionTypeData) -> Result<Self> {
		Ok(Self {
			primitive_type: ObjectInfoPrimitiveType::try_from(dto.primitive_type as isize).unwrap_or_default(),
			type_flag: ObjectFunctionTypeFlag::from_bits_retain(dto.type_flag),
			type_name_hash: dto.type_name_hash,
			type_name: dto.type_name.read(reader).unwrap_or_default(),
			item_primitive_type: ObjectInfoPrimitiveType::try_from(dto.item_primitive_type as isize).unwrap_or_default(),
			item_type_flag: ObjectFunctionTypeFlag::from_bits_retain(dto.item_type_flag),
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
	pub function: LuminousPointer<EbexObjectCall<()>>,
	pub function_dynamic: LuminousPointer<EbexObjectCallDynamic<()>>,
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
			name: dto.name.read(reader).unwrap_or_default(),
			flags: ObjectFunctionFlag::from_bits_retain(dto.flags),
			function: dto.function.debase_typed(base).cast(),
			function_dynamic: dto.function_dynamic.debase_typed(base).cast(),
			return_type,
			argument_types,
		})
	}
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ObjectInfoProperties {
	pub type_name: String,
	/// fnv1a64(name)
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::StrictPfx>"))]
	pub type_id: u64,
	pub base_type: Option<String>,
	/// crc32(name)
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::StrictPfx>"))]
	pub hash_code: u32,
	/// crc32(?)
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::StrictPfx>"))]
	pub version_hash_code: u32,
	pub all_properties_class_field_count: usize,
	pub properties: HashMap<String, ObjectProperty>,
}

impl ObjectInfoProperties {
	pub fn new(reader: &mut MemoryCursor, dto: ObjectInfoPropertyContainer) -> Result<Self> {
		let mut properties = HashMap::new();
		if dto.my_properties.size > 0 {
			let properties_unsafe: Vec<ObjectInfoProperty> = dto.my_properties.read(&mut reader.inner)?;

			for item in properties_unsafe {
				let property = ObjectProperty::new(reader, item);
				properties.insert(property.name.clone(), property);
			}
		}

		let type_name = dto.type_name.as_string(&mut reader.inner);
		let type_id = fnv1a64(type_name.as_bytes());

		let base_type = ObjectInfo::read_base_type(reader, dto.parent_properties.cast() + 16);

		Ok(Self {
			type_name,
			type_id,
			base_type,
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
	/// fnv1a64(name)
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::StrictPfx>"))]
	pub type_id: u64,
	pub base_type: Option<String>,
	/// crc32(name)
	#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::StrictPfx>"))]
	pub this_id: u32,
	pub class_functions: ObjectClassFunctions,
	pub size: usize,
	pub properties: ObjectInfoProperties,
	pub functions: HashMap<String, ObjectFunction>,
}

impl ObjectInfo {
	pub fn new(reader: &mut MemoryCursor, dto: ObjectType) -> Result<Self> {
		let name = dto.name.read(reader).unwrap_or_default();
		let type_id = fnv1a64(name.as_bytes());
		let base_type = Self::read_base_type(reader, dto.base_type.cast());

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
			base_type,
			class_functions,
			size: dto.size as usize,
			properties,
			functions,
		})
	}

	pub fn new_xv(reader: &mut MemoryCursor, dto: ObjectTypeXV) -> Result<Self> {
		let name = dto.name.read(reader).unwrap_or_default();
		let type_id = fnv1a64(name.as_bytes());
		let base_type = Self::read_base_type(reader, dto.base_type.cast());

		let class_functions = ObjectClassFunctions::new_xv(reader, dto);
		let properties = Self::read_properties(reader, dto.property_container)?;
		let functions = Self::read_object_functions(reader, dto.functions, dto.function_count as usize)?;

		Ok(Self {
			name: name.to_string(),
			type_id,
			this_id: dto.this_type,
			base_type,
			class_functions,
			size: dto.size as usize,
			properties,
			functions,
		})
	}

	fn read_base_type(reader: &mut MemoryCursor, pointer: LuminousPointer<LuminousCString>) -> Option<String> {
		if !pointer.is_valid() {
			return None;
		}

		if let Ok(name_pointer) = pointer.read(&mut reader.inner) { name_pointer.read(reader) } else { None }
	}

	fn read_object_functions(
		reader: &mut MemoryCursor,
		mut functions_ptr: LuminousPointer<crate::engine::r#unsafe::ebex::ObjectFunction>,
		function_count: usize,
	) -> Result<HashMap<String, ObjectFunction>> {
		let mut functions: HashMap<String, ObjectFunction> = HashMap::new();
		let size = size_of::<crate::engine::r#unsafe::ebex::ObjectFunction>();
		for _i in 0..function_count {
			let item_dto = functions_ptr.read(&mut reader.inner)?;
			let item = ObjectFunction::new(reader, item_dto)?;
			functions.insert(item.name.clone(), item);
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

impl Display for ObjectInfo {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}", self.name)
	}
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ObjectInfoRegistry {
	pub elements: HashMap<String, ObjectInfo>,
}

impl ObjectInfoRegistry {
	pub fn new(reader: &mut MemoryCursor) -> Result<Self> {
		let mut elements = HashMap::new();
		let is_xv = reader.inner.game_type() != LuminousGame::FORSPOKEN;
		// note: can't use LuminousStaticMap here because we cast the type later for XV.
		let mut registry_ptr: LuminousPointer<ObjectInfoPropertyPair> = reader.inner.get_base_address().cast()
			+ match reader.inner.game_type() {
				LuminousGame::FORSPOKEN => super::EBEX_OBJECT_ARRAY_ADDR_FORSPOKEN,
				_ => super::EBEX_OBJECT_ARRAY_ADDR_XV,
			};

		debug!("ebex object info registry address: {:?}", registry_ptr);

		for _ in 0..0x20 {
			for _ in 0..0x1000 {
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
						elements.insert(item.name.to_string(), item);
					}
					Err(err) => {
						error!("error reading object info: {}", err);
					}
				}
			}
		}

		debug!("sanity checking elements...");
		for element in elements.values() {
			if let Some(base_type) = &element.base_type
				&& !elements.contains_key(base_type)
			{
				warn!("missing base type {} for element {}", base_type, element.name);
			}

			if let Some(base_type) = &element.properties.base_type
				&& !elements.contains_key(base_type)
			{
				warn!("missing properties base type {} for element {}", base_type, element.properties.type_name);
			}
		}

		info!("loaded {} object infos", elements.len());

		Ok(Self {
			elements,
		})
	}
}
