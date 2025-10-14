// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

pub(crate) struct MemoryMapping {
	pub name: String,
	pub start: usize,
	#[cfg(target_os = "linux")]
	pub end: usize,
	#[cfg(target_os = "linux")]
	pub permissions: String,
}

impl MemoryMapping {
	pub(crate) fn new(line: &str) -> Option<Self> {
		let mut split = line.split_whitespace();

		let range = split.next()?;
		let mut range_split = range.split("-");
		let start = match range_split.next() {
			None => return None,
			Some(s) => match usize::from_str_radix(s, 16) {
				Err(_) => return None,
				Ok(i) => i,
			},
		};

		let end = match range_split.next() {
			None => return None,
			Some(s) => match usize::from_str_radix(s, 16) {
				Err(_) => return None,
				Ok(i) => i,
			},
		};

		if range_split.next().is_some() || start >= end {
			return None;
		}

		#[cfg(target_os = "linux")]
		let permissions = split.next()?.to_string();
		#[cfg(not(target_os = "linux"))]
		split.next()?; // permissions

		split.next()?; // offset
		split.next()?; // dev
		split.next()?; // inode

		let name = split.next()?.to_string();

		Some(Self {
			name,
			start,
			#[cfg(target_os = "linux")]
			end,
			#[cfg(target_os = "linux")]
			permissions,
		})
	}
}
