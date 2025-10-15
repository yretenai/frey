// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};

use crate::engine::LuminousPointer;
use crate::memory::MemoryReaderType;

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct LuminousDynamicArray {
	pub data: LuminousPointer,
	pub size: u32,
	pub capacity: u32,
}

impl LuminousDynamicArray {
	pub fn read<T: Pod + Default>(&self, reader: &mut MemoryReaderType) -> Result<Vec<T>> {
		if !self.data.is_valid() {
			bail!("invalid pointer");
		}

		let mut address = self.data;

		let mut vec = vec![Default::default(); self.size as usize];
		for item in vec.iter_mut().take(self.size as usize) {
			let pointer: LuminousPointer = address.read(reader)?;
			*item = pointer.read(reader)?;
			address += 8;
		}
		Ok(vec)
	}
}
