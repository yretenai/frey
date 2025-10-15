// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use bytemuck::{Pod, Zeroable};

use crate::engine::LuminousPointer;
use crate::memory::MemoryReaderType;

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed)]
pub struct LuminousDynamicArray {
	data: LuminousPointer,
	size: u32,
	capacity: u32,
}

impl LuminousDynamicArray {
	pub fn read<T: Pod + Default>(&self, reader: &mut MemoryReaderType) -> anyhow::Result<Vec<T>> {
		let mut address = self.data;
		let size = size_of::<T>();

		let mut vec = vec![Default::default(); size];
		for item in vec.iter_mut().take(size) {
			*item = address.read(reader)?;
			address += size;
		}
		Ok(vec)
	}
}
