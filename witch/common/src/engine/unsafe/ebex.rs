// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use bytemuck::{Pod, Zeroable};

use crate::engine::{LuminousCString, LuminousDynamicArray, LuminousIntrusivePointer, LuminousPointer, LuminousString};

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct ObjectTypeElement {
	pub type_id: u64,
	pub object_type: LuminousPointer<ObjectType>,
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct ObjectType {
	pub name: LuminousCString,
	pub this_type: u32,
	padding: u32,
	pub base_type: LuminousPointer<ObjectType>,
	pub unknown: u64,
	pub func1: LuminousPointer<()>,
	pub func2: LuminousPointer<()>,
	pub func3: LuminousPointer<()>,
	pub property_container: LuminousPointer<ObjectInfoPropertyContainer>,
	pub functions: LuminousPointer<ObjectFunction>,
	pub function_count: u32,
	pub size: u32,
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct ObjectTypeXV {
	pub name: LuminousCString,
	pub this_type: u32,
	padding1: u32,
	pub base_type: LuminousPointer<ObjectType>,
	pub func1: LuminousPointer<()>,
	pub func2: LuminousPointer<()>,
	pub func3: LuminousPointer<()>,
	pub property_container: LuminousPointer<ObjectInfoPropertyContainer>,
	pub functions: LuminousPointer<ObjectFunction>,
	pub function_count: u32,
	pub size: u32,
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct ObjectFunction {
	pub name: LuminousCString,
	pub name_hash: u32,
	pub flags: u32,
	pub function: LuminousPointer<()>,
	pub function_dynamic: LuminousPointer<()>,
	pub return_type: ObjectFunctionTypeData,
	pub argument_types: LuminousPointer<ObjectFunctionTypeData>,
	pub argument_count: u32,
	padding2: u32,
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct ObjectFunctionTypeData {
	pub primitive_type: u32,
	pub type_flag: u32,
	pub type_name_hash: u32,
	padding0: u32,
	pub type_name: LuminousCString,

	pub item_primitive_type: u32,
	pub item_type_flag: u32,
	pub item_type_name_hash: u32,
	padding1: u32,
	pub item_type_name: LuminousCString,
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct ObjectInfoPropertyContainer {
	pub base: LuminousIntrusivePointer,
	pub type_name: LuminousString,
	pub hash_code: u32,
	pub version_hash_code: u32,
	pub all_properties_class_field_count: u16,
	padding1: u16,
	padding2: u32,

	pub parent_properties: LuminousPointer<ObjectInfoPropertyContainer>,
	pub my_properties: LuminousDynamicArray<ObjectInfoProperty>,
	pub all_properties: LuminousDynamicArray<ObjectInfoProperty>,
	pub my_properties_lookup: LuminousDynamicArray<ObjectInfoProperty>, // Forspoken only
	pub all_properties_lookup: LuminousDynamicArray<ObjectInfoProperty>, // Forspoken only
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct ObjectInfoProperty {
	pub base: LuminousIntrusivePointer,
	pub name: LuminousString,
	pub hash_code: u32,
	padding1: u32,
	pub type_name: LuminousString,
	pub offset: u32,
	pub size: u32,
	pub item_count: u16,
	pub primitive_type: u8,
	pub item_primitive_type: u8,
	pub attributes: u8,
	padding2: u8,
	padding3: u16,
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct ObjectInfoPropertyPair {
	pub key: u64,
	pub value: LuminousPointer<ObjectType>,
}
