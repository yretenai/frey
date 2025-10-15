// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::cmp::min;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::{Result, anyhow};
use binrw::{BinRead, BinReaderExt, NullString, VecArgs};

use crate::engine::LuminousPointer;
use crate::memory::MemoryReader;
use crate::memory::linux_proc::{DumpMemory, MemoryMapping, get_process_base};

#[derive(BinRead)]
#[br(magic = b"NP93DUMP")]
struct DumpHeader {
	version: u64,
	proc: NullString,
}

#[derive(BinRead)]
#[br(magic = b"DUMPFOOT")]
struct DumpMagic;

pub struct Np93DumpReader {
	reader: File,
	memory: Vec<DumpMemory>,
	proc: Vec<MemoryMapping>,
	comm: Option<String>,
}

impl Np93DumpReader {
	pub fn new(path: &Path) -> Result<Np93DumpReader> {
		let mut reader = File::options().read(true).open(path)?;
		reader.seek(SeekFrom::End(-8))?;
		reader.read_ne::<DumpMagic>()?;
		reader.seek(SeekFrom::Start(0))?;

		let header: DumpHeader = reader.read_ne()?;

		let comm = match header.version {
			2.. => Some(reader.read_ne::<NullString>()?.to_string()),
			_ => None,
		};

		let mut proc: Vec<MemoryMapping> = Vec::new();
		for line in header.proc.to_string().split("\n") {
			if let Some(module) = MemoryMapping::new(line) {
				proc.push(module);
			}
		}

		reader.seek(SeekFrom::End(-16))?;
		let count = reader.read_ne::<u64>()?;
		reader.seek(SeekFrom::End(-(16i64 + size_of::<DumpMemory>() as i64)))?;
		let memory: Vec<DumpMemory> = reader.read_ne_args(VecArgs {
			count: count as usize,
			inner: <_>::default(),
		})?;

		Ok(Np93DumpReader {
			reader,
			memory,
			proc,
			comm,
		})
	}
}

pub(crate) fn read_from_virtual(file: &mut File, memory: &Vec<DumpMemory>, address: LuminousPointer, buf: &mut [u8]) -> Result<()> {
	let mut buf_offset = 0;

	for memory in memory {
		if memory.rva > address.0 || memory.rva + memory.length < address.0 {
			continue;
		}

		let offset = memory.offset + (address.0 - memory.rva);
		let size = min(memory.length as usize, buf.len());

		file.seek(SeekFrom::Start(offset))?;
		file.read_exact(&mut buf[buf_offset..(buf_offset + size)])?;

		buf_offset += size;
	}

	if buf_offset == buf.len() { Ok(()) } else { Err(anyhow!("could not fully read buffer")) }
}

impl MemoryReader for Np93DumpReader {
	fn read(&mut self, address: LuminousPointer, buf: &mut [u8]) -> Result<()> {
		read_from_virtual(&mut self.reader, &self.memory, address, buf)
	}

	fn get_base_address(&self) -> usize {
		get_process_base(self.get_process_name(), &self.proc)
	}

	fn get_process_name(&self) -> Option<&String> {
		if let Some(comm) = &self.comm {
			return Some(comm);
		}

		for proc in &self.proc {
			if proc.name.ends_with(".exe") {
				return Some(&proc.name);
			}
		}

		None
	}
}
