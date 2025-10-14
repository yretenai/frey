// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::cmp::min;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::{Result, anyhow};
use binrw::{BinRead, BinReaderExt, NullString, VecArgs};

use crate::memory::MemoryReader;
use crate::memory::linux_proc::MemoryMapping;

#[derive(BinRead)]
#[br(magic = b"NP93DUMP")]
struct DumpHeader {
	_version: u64,
	proc: NullString,
}

#[derive(BinRead)]
#[repr(C)]
struct DumpMemory {
	offset: u64,
	length: u64,
	rva: u64,
}

#[derive(BinRead)]
#[br(magic = b"DUMPFOOT")]
struct DumpMagic;

pub struct NeptuwuniumReader {
	reader: File,
	memory: Vec<DumpMemory>,
	proc: Vec<MemoryMapping>,
}

impl NeptuwuniumReader {
	pub fn new(path: &Path) -> Result<NeptuwuniumReader> {
		let mut reader = File::options().read(true).open(path)?;
		reader.seek(SeekFrom::End(-8))?;
		reader.read_ne::<DumpMagic>()?;
		reader.seek(SeekFrom::Start(0))?;

		let header: DumpHeader = reader.read_ne()?;
		let mut proc: Vec<MemoryMapping> = Vec::new();
		for line in header.proc.to_string().split("\n") {
			if let Some(module) = MemoryMapping::new(line) {
				proc.push(module);
			}
		}

		reader.seek(SeekFrom::End(-16))?;
		let count = reader.read_ne::<u64>()?;
		reader.seek(SeekFrom::End(-(16i64 + size_of::<DumpMemory>() as i64)))?;
		let memory: Vec<DumpMemory> = (reader).read_ne_args(VecArgs {
			count: count as usize,
			inner: <_>::default(),
		})?;

		Ok(NeptuwuniumReader {
			reader,
			memory,
			proc,
		})
	}
}

impl MemoryReader for NeptuwuniumReader {
	fn read(&mut self, address: usize, buf: &mut [u8]) -> Result<()> {
		let mut buf_offset = 0;

		for memory in &self.memory {
			if memory.rva > address as u64 || memory.rva + memory.length < address as u64 {
				continue;
			}

			let offset = memory.offset + (address as u64 - memory.rva);
			let size = min(memory.length as usize, buf.len());

			self.reader.seek(SeekFrom::Start(offset))?;
			self.reader.read_exact(&mut buf[buf_offset..(buf_offset + size)])?;

			buf_offset += size;
		}

		if buf_offset == buf.len() { Ok(()) } else { Err(anyhow!("could not fully read buffer")) }
	}

	fn get_base_address(&self) -> usize {
		for proc in &self.proc {
			if proc.name.ends_with(".exe") {
				return proc.start;
			}
		}

		0x140000000
	}

	fn get_process_name(&self) -> Option<&String> {
		for proc in &self.proc {
			if proc.name.ends_with(".exe") {
				return Some(&proc.name);
			}
		}

		None
	}
}
