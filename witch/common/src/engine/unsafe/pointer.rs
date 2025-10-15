// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::fmt::{Debug, Formatter};
use std::io::{Seek, SeekFrom};
use std::marker::PhantomData;
use std::ops::{Add, AddAssign, Sub, SubAssign};

use anyhow::{Result, bail};
use binrw::{BinReaderExt, NullString};
use bytemuck::{Pod, Zeroable};

use crate::memory::{MemoryCursor, MemoryReaderType};

#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(transparent)]
pub struct LuminousPointer<T> {
	pub inner: u64,
	_marker: PhantomData<T>,
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(transparent)]
pub struct LuminousCString {
	pub inner: LuminousPointer<()>,
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct LuminousIntrusivePointer {
	vtable: LuminousPointer<()>,
	ref_count: u32,
	reserved: u32,
}

impl LuminousCString {
	pub fn read(&self, reader: &mut MemoryCursor) -> Option<String> {
		if !self.is_valid() {
			return None;
		}

		reader.seek(SeekFrom::Start(self.inner.inner)).ok()?;
		Some(reader.read_ne::<NullString>().ok()?.to_string())
	}

	pub fn is_valid(&self) -> bool {
		self.inner.is_valid()
	}
}

impl<T> LuminousPointer<T> {
	pub fn new(address: u64) -> Self {
		Self {
			inner: address,
			_marker: PhantomData,
		}
	}

	pub(crate) fn cast<N>(&self) -> LuminousPointer<N> {
		LuminousPointer::new(self.inner)
	}

	pub fn is_valid(&self) -> bool {
		self.inner > 0x1000 && self.inner < 0x7fffffffffffffff
	}

	pub fn debase(&self, base: LuminousPointer<T>) -> usize {
		if !self.is_valid() { 0 } else { (self.inner - base.inner) as usize }
	}
}

impl<T: Pod> LuminousPointer<T> {
	pub fn read(&self, reader: &mut MemoryReaderType) -> Result<T> {
		if !self.is_valid() {
			bail!("invalid pointer");
		}

		reader.read_type(*self)
	}
}

impl<T> Debug for LuminousPointer<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "0x{:016X}", self.inner)
	}
}

macro_rules! define_arith {
    ($($type_name:ty),*$(,)?) => {
        $(
            impl<T> AddAssign<$type_name> for LuminousPointer<T> {
				fn add_assign(&mut self, rhs: $type_name) {
					self.inner += rhs as u64
				}
            }

            impl<T> SubAssign<$type_name> for LuminousPointer<T> {
				fn sub_assign(&mut self, rhs: $type_name) {
					self.inner -= rhs as u64
				}
            }

            impl<T> Add<$type_name> for LuminousPointer<T> {
				type Output = LuminousPointer<T>;

				fn add(self, rhs: $type_name) -> Self::Output {
					LuminousPointer::new(self.inner + rhs as u64)
				}
            }

            impl<T> Sub<$type_name> for LuminousPointer<T> {
				type Output = LuminousPointer<T>;

				fn sub(self, rhs: $type_name) -> Self::Output {
					LuminousPointer::new(self.inner - rhs as u64)
				}
            }

			impl<T> From<$type_name> for LuminousPointer<T> {
				fn from(value: $type_name) -> Self {
					LuminousPointer::new(value as u64)
				}
			}

			impl<T> From<LuminousPointer<T>> for $type_name {
				fn from(value: LuminousPointer<T>) -> Self {
					value.inner as $type_name
				}
			}
        )*
    }
}

define_arith!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

impl<L, R> AddAssign<LuminousPointer<R>> for LuminousPointer<L> {
	fn add_assign(&mut self, rhs: LuminousPointer<R>) {
		self.inner += rhs.inner
	}
}

impl<L, R> SubAssign<LuminousPointer<R>> for LuminousPointer<L> {
	fn sub_assign(&mut self, rhs: LuminousPointer<R>) {
		self.inner -= rhs.inner
	}
}

impl<L, R> Add<LuminousPointer<R>> for LuminousPointer<L> {
	type Output = LuminousPointer<L>;

	fn add(self, rhs: LuminousPointer<R>) -> Self::Output {
		LuminousPointer::new(self.inner + rhs.inner)
	}
}

impl<L, R> Sub<LuminousPointer<R>> for LuminousPointer<L> {
	type Output = LuminousPointer<L>;

	fn sub(self, rhs: LuminousPointer<R>) -> Self::Output {
		LuminousPointer::new(self.inner - rhs.inner)
	}
}
