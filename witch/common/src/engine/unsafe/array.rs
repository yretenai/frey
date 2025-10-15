// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::marker::PhantomData;

use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};

use crate::engine::LuminousPointer;
use crate::memory::MemoryReaderType;

#[derive(Debug, Copy, Clone, Default)]
#[repr(C, packed(8))]
pub struct LuminousDynamicArray<T: Pod> {
	pub data: LuminousPointer<LuminousPointer<T>>,
	pub size: u32,
	pub capacity: u32,
	_marker: PhantomData<T>,
}

unsafe impl<T: Pod> Zeroable for LuminousDynamicArray<T> {}
unsafe impl<T: Pod> Pod for LuminousDynamicArray<T> {}

impl<T: Pod + Default> LuminousDynamicArray<T> {
	pub fn read(&self, reader: &mut MemoryReaderType) -> Result<Vec<T>> {
		if !self.data.is_valid() {
			bail!("invalid pointer");
		}

		let mut address = self.data;

		let mut vec = vec![Default::default(); self.size as usize];
		for item in vec.iter_mut().take(self.size as usize) {
			let pointer: LuminousPointer<T> = address.read(reader)?;
			*item = pointer.read(reader)?;
			address += 8;
		}
		Ok(vec)
	}
}
