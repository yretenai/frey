// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};

use crate::engine::LuminousPointer;
use crate::memory::MemoryReader;

/// A Dynamic Array from the Game Engine internals
/// sized with a capacity, std::vector-like
#[derive(Debug, Copy, Clone, Default)]
#[repr(C)]
pub struct LuminousDynamicArray<T: Pod> {
	pub inner: LuminousPointer<LuminousPointer<T>>,
	pub size: u32,
	pub capacity: u32,
}

unsafe impl<T: Pod> Zeroable for LuminousDynamicArray<T> {}
unsafe impl<T: Pod> Pod for LuminousDynamicArray<T> {}

impl<T: Pod + Default> LuminousDynamicArray<T> {
	pub fn read(&self, reader: &mut MemoryReader) -> Result<Vec<T>> {
		if !self.is_valid() {
			bail!("invalid pointer");
		}

		if self.is_empty() {
			return Ok(Default::default());
		}

		let mut address = self.inner;

		let mut vec = vec![Default::default(); self.size as usize];
		for item in vec.iter_mut().take(self.size as usize) {
			let pointer: LuminousPointer<T> = address.read(reader)?;
			*item = pointer.read(reader)?;
			address += 8;
		}
		Ok(vec)
	}

	pub fn is_valid(&self) -> bool {
		self.inner.is_valid() && self.size <= self.capacity
	}

	pub fn is_empty(&self) -> bool {
		self.size == 0
	}

	pub fn read_ptrs(&self, reader: &mut MemoryReader) -> Result<Vec<LuminousPointer<T>>> {
		if !self.is_valid() {
			bail!("invalid pointer");
		}

		if self.is_empty() {
			return Ok(Default::default());
		}

		let mut address = self.inner;

		let mut vec = vec![Default::default(); self.size as usize];
		for item in vec.iter_mut().take(self.size as usize) {
			*item = address.read(reader)?;
			address += 8;
		}

		Ok(vec)
	}
}
