// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::collections::HashMap;
use std::hash::Hash;

use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};

use crate::engine::LuminousPointer;
use crate::memory::MemoryReaderType;

#[derive(Debug, Copy, Clone, Default)]
#[repr(C, packed(8))]
pub struct LuminousDynamicMap<K: Pod + Eq + Hash, V: Pod> {
	pub buckets: LuminousPointer<LuminousDynamicMapPair<K, V>>,
	pub chain: LuminousPointer<LuminousDynamicMapPair<K, V>>,
	pub free_chain: LuminousPointer<LuminousDynamicMapPair<K, V>>,
	pub bucket_count: u32,
	pub chain_count: u32,
	pub occupancy: u32,
	pub chain_used: u32,
	pub expand_rate: f32,
	pub chain_to_bucket_ratio: f32,
	pub hasher: u64,
}
unsafe impl<K: Pod + Eq + Hash, V: Pod> Zeroable for LuminousDynamicMap<K, V> {}
unsafe impl<K: Pod + Eq + Hash, V: Pod> Pod for LuminousDynamicMap<K, V> {}

#[derive(Debug, Copy, Clone, Default)]
#[repr(C, packed(8))]
pub struct LuminousDynamicMapPair<K: Pod + Eq + Hash, V: Pod> {
	pub next: LuminousPointer<LuminousDynamicMapPair<K, V>>,
	pub value: LuminousPointer<V>,
	pub key: K,
}

unsafe impl<K: Pod + Eq + Hash, V: Pod> Zeroable for LuminousDynamicMapPair<K, V> {}
unsafe impl<K: Pod + Eq + Hash, V: Pod> Pod for LuminousDynamicMapPair<K, V> {}

// todo: maybe iterator types
impl<K: Pod + Eq + Hash, V: Pod> LuminousDynamicMap<K, V> {
	pub fn read(&self, reader: &mut MemoryReaderType) -> Result<HashMap<K, V>> {
		if !self.buckets.is_valid() || !self.chain.is_valid() {
			bail!("invalid pointer");
		}

		let mut hashmap: HashMap<K, V> = HashMap::new();
		Self::process_chains(reader, &mut hashmap, self.buckets, self.bucket_count)?;
		Self::process_chains(reader, &mut hashmap, self.chain, self.chain_used)?;

		Ok(hashmap)
	}

	fn process_chains(
		reader: &mut MemoryReaderType,
		hashmap: &mut HashMap<K, V>,
		mut address: LuminousPointer<LuminousDynamicMapPair<K, V>>,
		size: u32,
	) -> Result<()> {
		for _ in 0..size {
			let mut pair: LuminousDynamicMapPair<K, V> = address.read(reader)?;
			address += size_of::<LuminousDynamicMapPair<K, V>>();

			while !pair.value.is_valid() {
				let value = pair.value.read(reader)?;
				hashmap.insert(pair.key, value);

				if !pair.next.is_valid() {
					break;
				}

				pair = pair.next.read(reader)?;
			}
		}

		Ok(())
	}
}
