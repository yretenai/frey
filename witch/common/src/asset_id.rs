// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::{LazyLock, RwLock};

use anyhow::anyhow;
use bytemuck::{Pod, Zeroable};

use crate::hash::fnv1a64;

#[derive(Copy, Clone, Default, Eq, PartialEq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[repr(transparent)]
pub struct AssetId(#[cfg_attr(feature = "serde", serde(with = "serde_hex::SerHex::<serde_hex::StrictPfx>"))] u64);

const ASSET_ID_PATH_MASK: u64 = 0xfffffffffff;
const ASSET_ID_TYPE_MASK: u64 = 0xfffff00000000000;

static STRING_LOOKUP: RwLock<LazyLock<HashMap<u64, String>>> = RwLock::new(LazyLock::new(HashMap::new));

impl AssetId {
	pub fn path_hash(&self) -> u64 {
		self.0 & ASSET_ID_PATH_MASK
	}

	pub fn type_hash(&self) -> u64 {
		self.0 & ASSET_ID_TYPE_MASK
	}

	pub fn is_null(&self) -> bool {
		self.0 == 0
	}

	pub fn from_pathlike(path: &str) -> AssetId {
		if path.is_empty() {
			return AssetId(0);
		}

		let mut hash: u64;

		if let Some(pair) = path.split_once('.') {
			hash = fnv1a64(pair.0.to_lowercase().as_bytes()) & ASSET_ID_PATH_MASK;
			hash |= (fnv1a64(pair.1.trim_end_matches('@').as_bytes())) << 44;
		} else {
			hash = fnv1a64(path.to_lowercase().as_bytes()) & ASSET_ID_PATH_MASK;
		}

		Self(hash)
	}
}

impl Hash for AssetId {
	fn hash<H: Hasher>(&self, state: &mut H) {
		state.write_u64(self.0)
	}
}

impl TryFrom<&AssetId> for String {
	type Error = anyhow::Error;

	fn try_from(value: &AssetId) -> Result<Self, Self::Error> {
		STRING_LOOKUP.read().map_err(|e| anyhow!(e.to_string()))?.get(&value.0).cloned().ok_or(anyhow!("cannot find path"))
	}
}

impl From<u64> for AssetId {
	fn from(value: u64) -> Self {
		AssetId(value)
	}
}

impl From<&str> for AssetId {
	fn from(value: &str) -> Self {
		AssetId(fnv1a64(value.as_bytes()))
	}
}

impl TryFrom<&Path> for AssetId {
	type Error = anyhow::Error;

	fn try_from(path: &Path) -> Result<Self, Self::Error> {
		Ok(AssetId::from_pathlike(path.to_str().ok_or(anyhow!("path is not valid"))?))
	}
}

impl Display for AssetId {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		if self.is_null() {
			return write!(f, "null");
		}

		if let Ok(string) = String::try_from(self) {
			write!(f, "{}", string)
		} else if let Ok(ext) = String::try_from(&AssetId(self.type_hash())) {
			write!(f, "{:#011x}.{}", self.path_hash(), ext)
		} else {
			write!(f, "{:#016x}", self.0)
		}
	}
}

impl Debug for AssetId {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "AssetId(0x{:#016x}), Path={:?})", self.0, String::try_from(self))
	}
}
