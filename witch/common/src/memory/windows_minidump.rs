// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::fs::File;
use std::io::Cursor;
use std::path::Path;

use anyhow::Result;
use binrw::{BinRead, BinReaderExt, VecArgs};
use minidump::format::MINIDUMP_STREAM_TYPE;
use minidump::{MinidumpModule, MinidumpModuleList, Module};

use crate::engine::LuminousPointer;
use crate::memory::MemoryReader;
use crate::memory::linux_proc::DumpMemory;
use crate::memory::neptuwunium_dump::read_from_virtual;

pub struct Win32DumpReader {
	reader: File,
	memory: Vec<DumpMemory>,
	modules: Vec<MinidumpModule>,
}

#[derive(BinRead)]
#[br(little)]
struct Minidump64Header {
	count: u64,
	offset: u64,
}

#[derive(BinRead)]
#[br(little)]
struct Minidump64 {
	rva: u64,
	length: u64,
}

impl Win32DumpReader {
	pub fn new(path: &Path) -> Result<Win32DumpReader> {
		let minidump = minidump::Minidump::read_path(path)?;
		let mut memory_list = Cursor::new(minidump.get_raw_stream(MINIDUMP_STREAM_TYPE::Memory64ListStream as u32)?);
		let module_list = &minidump.get_stream::<MinidumpModuleList>()?;

		let memory_header: Minidump64Header = memory_list.read_ne()?;
		let memory_segm: Vec<Minidump64> = memory_list.read_ne_args(VecArgs {
			count: memory_header.count as usize,
			inner: Default::default(),
		})?;

		let mut memory: Vec<DumpMemory> = Vec::new();
		let mut offset = memory_header.offset;
		for memory_seg in memory_segm {
			memory.push(DumpMemory {
				offset,
				length: memory_seg.length,
				rva: memory_seg.rva,
			});
			offset += memory_seg.length;
		}

		let mut modules: Vec<MinidumpModule> = Vec::new();
		for module in module_list.iter().collect::<Vec<_>>() {
			modules.push(module.clone());
		}

		let reader = File::options().read(true).open(path)?;
		Ok(Win32DumpReader {
			reader,
			memory,
			modules,
		})
	}
}

impl MemoryReader for Win32DumpReader {
	fn read(&mut self, address: LuminousPointer, buf: &mut [u8]) -> Result<()> {
		read_from_virtual(&mut self.reader, &self.memory, address, buf)
	}

	fn get_base_address(&self) -> LuminousPointer {
		let own_path = match self.get_process_name() {
			Some(path) => path,
			None => return LuminousPointer(0),
		};

		for module in &self.modules {
			if module.name.eq(&own_path) {
				return LuminousPointer(module.base_address());
			}
		}

		LuminousPointer(0)
	}

	fn get_process_name(&self) -> Option<String> {
		for module in &self.modules {
			if module.name.ends_with(".exe") {
				return Some(module.name.split(&['\\', '/'][..]).next_back()?.to_string());
			}
		}

		None
	}
}
