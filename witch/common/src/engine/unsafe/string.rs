// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use bytemuck::{Pod, Zeroable};

use crate::engine::LuminousPointer;
use crate::memory::MemoryReader;

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
pub struct LuminousString {
	pub address: LuminousPointer<u8>,
	pub memory_size: u32,
	pub flags: u32,
}

impl LuminousString {
	pub fn as_string(&self, reader: &mut MemoryReader) -> String {
		self.as_some_string(reader).unwrap_or_default()
	}

	pub fn as_some_string(&self, reader: &mut MemoryReader) -> Option<String> {
		if self.size() == 0 {
			return None;
		}

		let mut buf = vec![0u8; self.size()];
		if reader.read(self.address.cast(), &mut buf).is_err() {
			return None;
		}

		Some(String::from_utf8_lossy(&buf).to_string())
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
