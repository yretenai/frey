// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use binrw::BinRead;

use crate::engine::LuminousPointer;

#[derive(BinRead)]
#[repr(C)]
pub(crate) struct DumpMemory {
	pub(crate) offset: u64,
	pub(crate) length: u64,
	pub(crate) rva: u64,
}

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

		let name = split.collect::<Vec<&str>>().join(" ");

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

const FALLBACK: &str = ".exe";

pub(crate) fn get_process_base(name: Option<String>, proc: &Vec<MemoryMapping>) -> LuminousPointer {
	let fallback = FALLBACK.to_string();
	let name = name.unwrap_or(fallback);
	for proc in proc {
		if proc.name.ends_with(&name) {
			return LuminousPointer(proc.start as u64);
		}
	}

	LuminousPointer(0x140000000)
}
