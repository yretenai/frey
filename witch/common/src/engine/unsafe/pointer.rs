// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::fmt::{Debug, Display, Formatter};
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

	pub fn read_null_string(&self, reader: &mut MemoryCursor) -> Option<String> {
		if !self.is_valid() {
			return None;
		}

		reader.seek(SeekFrom::Start(self.0)).ok()?;
		Some(reader.read_ne::<NullString>().ok()?.to_string())
	}

	pub fn is_valid(&self) -> bool {
		self.0 > 0x1000 && self.0 < 0x7fffffffffffffff
	}

	pub fn debase(&self, base: LuminousPointer) -> usize {
		if !self.is_valid() { 0 } else { (*self - base).0 as usize }
	}
}

impl Debug for LuminousPointer {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "0x{:016X}", self.0)
	}
}

impl Display for LuminousPointer {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "0x{:016X}", self.0)
	}
}

macro_rules! define_arith {
    ($($type_name:ty),*$(,)?) => {
        $(
            impl AddAssign<$type_name> for LuminousPointer {
				fn add_assign(&mut self, rhs: $type_name) {
					self.0 += rhs as u64
				}
            }

            impl SubAssign<$type_name> for LuminousPointer {
				fn sub_assign(&mut self, rhs: $type_name) {
					self.0 -= rhs as u64
				}
            }

            impl Add<$type_name> for LuminousPointer {
				type Output = LuminousPointer;

				fn add(self, rhs: $type_name) -> Self::Output {
					LuminousPointer(self.0 + rhs as u64)
				}
            }

            impl Sub<$type_name> for LuminousPointer {
				type Output = LuminousPointer;

				fn sub(self, rhs: $type_name) -> Self::Output {
					LuminousPointer(self.0 - rhs as u64)
				}
            }
        )*
    }
}

define_arith!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

impl AddAssign<LuminousPointer> for LuminousPointer {
	fn add_assign(&mut self, rhs: LuminousPointer) {
		self.0 += rhs.0
	}
}

impl SubAssign<LuminousPointer> for LuminousPointer {
	fn sub_assign(&mut self, rhs: LuminousPointer) {
		self.0 -= rhs.0
	}
}

impl Add<LuminousPointer> for LuminousPointer {
	type Output = LuminousPointer;

	fn add(self, rhs: LuminousPointer) -> Self::Output {
		LuminousPointer(self.0 + rhs.0)
	}
}

impl Sub<LuminousPointer> for LuminousPointer {
	type Output = LuminousPointer;

	fn sub(self, rhs: LuminousPointer) -> Self::Output {
		LuminousPointer(self.0 - rhs.0)
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
