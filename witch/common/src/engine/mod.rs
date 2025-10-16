// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

pub mod asset_factory;
pub mod ebex;
pub mod r#unsafe;

pub use r#unsafe::array::LuminousDynamicArray;
pub use r#unsafe::map::LuminousDynamicMap;
pub use r#unsafe::mutex::LuminousMutex;
pub use r#unsafe::pointer::{LuminousCString, LuminousIntrusivePointer, LuminousPointer};
pub use r#unsafe::string::LuminousString;

pub const EBEX_OBJECT_ARRAY_ADDR_FORSPOKEN: u64 = 0x77c5660;
pub const EBEX_OBJECT_ARRAY_ADDR_XV: u64 = 0x4f5bd30;
pub const ASSET_FACTORY_CONTAINER_ADDR_FORSPOKEN: u64 = 0x79d3458;
pub const ASSET_FACTORY_CONTAINER_ADDR_XV: u64 = 0x4ce7ba8;
pub const ASSET_FACTORY_CONTAINER_OFFS_FORSPOKEN: u64 = 0x5200;
pub const ASSET_FACTORY_CONTAINER_OFFS_XV: u64 = 0x8e0;

#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub enum LuminousGame {
	LuminousEngine,
	FinalFantasyXV,
	FORSPOKEN,
}
