// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use bytemuck::{Pod, Zeroable};

use crate::asset_id::AssetId;
use crate::engine::{LuminousDynamicMap, LuminousMutex, LuminousPointer};

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct AssetFactory {
	pub loader: LuminousPointer<()>,
	pub alloc: LuminousPointer<()>,
	pub init: LuminousPointer<()>,
	pub fs: LuminousPointer<()>,
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct AssetFactoryStatistics {
	pub current_size: u64,
	pub peak_size: u64,
	pub current_count: u32,
	pub peak_count: u32,
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct AssetFactoryStatisticsHolder {
	pub factory: LuminousPointer<AssetFactory>,
	pub statistics: AssetFactoryStatistics,
	pub name: [u8; 0x10], // XV is 0x20 but useless
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct AssetFactoryContainer {
	pub mutex: LuminousMutex,
	pub factory_holder: LuminousDynamicMap<AssetId, AssetFactoryStatisticsHolder>,
	pub default_factory: AssetFactory,
}
