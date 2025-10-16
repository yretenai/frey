// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use bytemuck::{Pod, Zeroable};

use crate::engine::r#unsafe::map::LuminousStaticMap;
use crate::engine::{LuminousDynamicArray, LuminousPointer};

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
pub struct GameModuleVTable {
	pub alloc: LuminousPointer<()>,
	pub get_name: LuminousPointer<()>,
	pub get_dependencies: LuminousPointer<()>,
	pub init: LuminousPointer<()>,
	pub tick: LuminousPointer<()>,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct GameModuleKey {
	pub key: u32,
	pub unknown: u32,
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
pub struct GameModuleMap {
	pub pending_initialize: LuminousStaticMap<GameModuleKey, LuminousPointer<()>>,
	pub modules: LuminousStaticMap<GameModuleKey, LuminousPointer<()>>,
	pub module_list: LuminousDynamicArray<LuminousPointer<()>>,
}
