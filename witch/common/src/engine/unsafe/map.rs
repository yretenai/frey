// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::Hash;

use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};

use crate::engine::LuminousPointer;
use crate::memory::MemoryReader;

/// A hashmap from game internals, similar to std::unordered_map<,>
#[derive(Debug, Copy, Clone, Default)]
#[repr(C)]
pub struct LuminousDynamicMap<K: Pod + Eq + Hash + Default, V: Pod> {
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

unsafe impl<K: Pod + Eq + Hash + Default, V: Pod> Zeroable for LuminousDynamicMap<K, V> {}
unsafe impl<K: Pod + Eq + Hash + Default, V: Pod> Pod for LuminousDynamicMap<K, V> {}

/// A frozen hash map from game internals
/// Uses binary searching for keys.
#[derive(Debug, Copy, Clone, Default)]
#[repr(C)]
pub struct LuminousStaticMap<K: Pod + Eq + Hash + Default, V: Pod> {
	pub data: LuminousPointer<LuminousStaticMapPair<K, V>>,
	pub size: u32,
	pub capacity: u32,
}
unsafe impl<K: Pod + Eq + Hash + Default, V: Pod> Zeroable for LuminousStaticMap<K, V> {}
unsafe impl<K: Pod + Eq + Hash + Default, V: Pod> Pod for LuminousStaticMap<K, V> {}

#[derive(Debug, Copy, Clone, Default)]
#[repr(C)]
pub struct LuminousDynamicMapPair<K: Pod + Eq + Hash + Default, V: Pod> {
	pub next: LuminousPointer<LuminousDynamicMapPair<K, V>>,
	pub value: LuminousPointer<V>,
	pub key: K,
}

unsafe impl<K: Pod + Eq + Hash + Default, V: Pod> Zeroable for LuminousDynamicMapPair<K, V> {}
unsafe impl<K: Pod + Eq + Hash + Default, V: Pod> Pod for LuminousDynamicMapPair<K, V> {}

#[derive(Debug, Copy, Clone, Default)]
#[repr(C)]
pub struct LuminousStaticMapPair<K: Pod + Eq + Hash + Default, V: Pod> {
	pub key: K,
	pub value: V,
}

unsafe impl<K: Pod + Eq + Hash + Default, V: Pod> Zeroable for LuminousStaticMapPair<K, V> {}
unsafe impl<K: Pod + Eq + Hash + Default, V: Pod> Pod for LuminousStaticMapPair<K, V> {}

// todo: maybe iterator types
impl<K: Pod + Eq + Hash + Default, V: Pod> LuminousDynamicMap<K, V> {
	pub fn read(&self, reader: &mut MemoryReader) -> Result<HashMap<K, V>> {
		if !self.buckets.is_valid() || !self.chain.is_valid() {
			bail!("invalid pointer");
		}

		let mut hashmap: HashMap<K, V> = HashMap::new();
		Self::process_chains(reader, &mut hashmap, self.buckets, self.bucket_count)?;
		Self::process_chains(reader, &mut hashmap, self.chain, self.chain_used)?;

		Ok(hashmap)
	}

	fn process_chains(
		reader: &mut MemoryReader,
		hashmap: &mut HashMap<K, V>,
		mut address: LuminousPointer<LuminousDynamicMapPair<K, V>>,
		size: u32,
	) -> Result<()> {
		let default = Default::default();
		for _ in 0..size {
			let mut pair: LuminousDynamicMapPair<K, V> = address.read(reader)?;
			address += size_of::<LuminousDynamicMapPair<K, V>>();

			while pair.value.is_valid() && pair.key != default {
				if let Entry::Vacant(e) = hashmap.entry(pair.key) {
					let value = pair.value.read(reader)?;
					e.insert(value);
				}

				if !pair.next.is_valid() {
					break;
				}

				pair = pair.next.read(reader)?;
			}
		}

		Ok(())
	}
}

impl<K: Pod + Eq + Hash + Default, V: Pod + Default> LuminousStaticMap<K, V> {
	pub fn read(&self, reader: &mut MemoryReader) -> Result<HashMap<K, V>> {
		if !self.data.is_valid() {
			bail!("invalid pointer");
		}

		if self.size == 0 {
			return Ok(HashMap::new());
		}

		let mut hashmap: HashMap<K, V> = HashMap::new();
		let mut address = self.data;

		for _ in 0..self.size {
			let pair = address.read(reader)?;
			if let Entry::Vacant(e) = hashmap.entry(pair.key) {
				e.insert(pair.value);
			}

			address += size_of::<LuminousStaticMapPair<K, V>>();
		}

		Ok(hashmap)
	}
}
