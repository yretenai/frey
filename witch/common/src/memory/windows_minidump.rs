// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::path::Path;

use anyhow::Result;

use crate::memory::MemoryReader;

pub struct MinidumpReader {}

impl MinidumpReader {
	pub fn new(_path: &Path) -> MinidumpReader {
		todo!()
	}
}

impl MemoryReader for MinidumpReader {
	fn read(&mut self, _address: usize, _buf: &mut [u8]) -> Result<()> {
		todo!()
	}

	fn get_base_address(&self) -> usize {
		todo!()
	}

	fn get_process_name(&self) -> Option<&String> {
		todo!()
	}
}
