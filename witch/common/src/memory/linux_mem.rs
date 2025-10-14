// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::path::Path;

use anyhow::Result;

use crate::memory::MemoryReader;

pub struct LinuxMemoryReader {}

impl LinuxMemoryReader {
	pub fn new(_path: &Path) -> LinuxMemoryReader {
		todo!()
	}
}

impl MemoryReader for LinuxMemoryReader {
	fn read(&mut self, address: usize, buf: &mut [u8]) -> Result<()> {
		todo!()
	}

	fn get_base_address(&self) -> usize {
		todo!()
	}

	fn get_process_name(&self) -> Option<&String> {
		todo!()
	}
}
