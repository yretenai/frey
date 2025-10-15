// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::fmt::{Debug, Formatter};
use std::io::{Seek, SeekFrom};
use std::ops::{Add, AddAssign, Sub, SubAssign};

use anyhow::{Result, bail};
use binrw::{BinReaderExt, NullString};
use bytemuck::{Pod, Zeroable};

use crate::memory::{MemoryCursor, MemoryReaderType};

#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(transparent)]
pub struct LuminousPointer(pub u64);

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct LuminousIntrusivePointer {
	vtable: LuminousPointer,
	ref_count: u32,
	reserved: u32,
}

impl LuminousPointer {
	pub fn read<T: Pod + Default>(&self, reader: &mut MemoryReaderType) -> Result<T> {
		if !self.is_valid() {
			bail!("invalid pointer");
		}

		reader.read_type(*self)
	}

	pub fn read_null_string(&self, reader: &mut MemoryCursor) -> Result<String> {
		reader.seek(SeekFrom::Start(self.0))?;
		Ok(reader.read_ne::<NullString>()?.to_string())
	}

	pub fn is_valid(&self) -> bool {
		self.0 > 0x140000000 && self.0 < 0x7fffffffffffffff
	}

	pub fn debase(&self, base: usize) -> usize {
		if !self.is_valid() { 0 } else { self.0 as usize - base }
	}
}

impl Debug for LuminousPointer {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "0x{:016X}", self.0)
	}
}

impl AddAssign<u64> for LuminousPointer {
	fn add_assign(&mut self, rhs: u64) {
		self.0 += rhs
	}
}

impl AddAssign<usize> for LuminousPointer {
	fn add_assign(&mut self, rhs: usize) {
		self.0 += rhs as u64
	}
}

impl Add<u64> for LuminousPointer {
	type Output = LuminousPointer;

	fn add(self, rhs: u64) -> Self::Output {
		LuminousPointer(self.0 + rhs)
	}
}

impl Add<usize> for LuminousPointer {
	type Output = LuminousPointer;

	fn add(self, rhs: usize) -> Self::Output {
		LuminousPointer(self.0 + rhs as u64)
	}
}

impl SubAssign<u64> for LuminousPointer {
	fn sub_assign(&mut self, rhs: u64) {
		self.0 -= rhs
	}
}

impl SubAssign<usize> for LuminousPointer {
	fn sub_assign(&mut self, rhs: usize) {
		self.0 -= rhs as u64
	}
}

impl Sub<u64> for LuminousPointer {
	type Output = LuminousPointer;

	fn sub(self, rhs: u64) -> Self::Output {
		LuminousPointer(self.0 - rhs)
	}
}

impl Sub<usize> for LuminousPointer {
	type Output = LuminousPointer;

	fn sub(self, rhs: usize) -> Self::Output {
		LuminousPointer(self.0 - rhs as u64)
	}
}

impl From<LuminousPointer> for usize {
	fn from(value: LuminousPointer) -> Self {
		value.0 as usize
	}
}

impl From<LuminousPointer> for u64 {
	fn from(value: LuminousPointer) -> Self {
		value.0
	}
}

impl From<usize> for LuminousPointer {
	fn from(value: usize) -> Self {
		LuminousPointer(value as u64)
	}
}

impl From<u64> for LuminousPointer {
	fn from(value: u64) -> Self {
		LuminousPointer(value)
	}
}
