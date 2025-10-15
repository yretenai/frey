// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use bytemuck::{Pod, Zeroable};

use crate::engine::LuminousPointer;
use crate::memory::{MemoryReader, MemoryReaderType};

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct LuminousString {
	pub address: LuminousPointer,
	pub memory_size: u32,
	pub flags: u32,
}

impl LuminousString {
	pub fn as_string(&self, reader: &mut MemoryReaderType) -> String {
		if self.size() == 0 {
			return String::new();
		}

		let mut buf = vec![0u8; self.size()];
		if reader.read(self.address, &mut buf).is_err() {
			return String::new();
		}

		String::from_utf8_lossy(&buf).to_string()
	}

	pub fn size(&self) -> usize {
		(self.flags & 0xffffff) as usize
	}

	pub fn is_read_only(&self) -> bool {
		(self.flags >> 30) & 1 == 1
	}

	pub fn is_allocated(&self) -> bool {
		(self.flags >> 31) & 1 == 1
	}
}
