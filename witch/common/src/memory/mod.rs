// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

#[cfg(target_os = "linux")]
pub mod linux_mem;
pub(crate) mod linux_proc;
pub mod neptuwunium_dump;
#[cfg(target_os = "windows")]
pub mod windows_mem;
pub mod windows_minidump;

use std::io::{ErrorKind, Read, Seek, SeekFrom};

use anyhow::anyhow;
use bytemuck::Pod;

pub enum MemoryReaderType {
	Neptuwunium(neptuwunium_dump::NeptuwuniumReader),
	Minidump(windows_minidump::MinidumpReader),
	#[cfg(target_os = "linux")]
	Linux(linux_mem::LinuxMemoryReader),
	#[cfg(target_os = "windows")]
	Windows(windows_mem::Win32MemoryReader),
}

pub struct MemoryCursor {
	pos: usize,
	reader: MemoryReaderType,
}

pub trait MemoryReader {
	fn read(&mut self, address: usize, buf: &mut [u8]) -> anyhow::Result<()>;
	fn get_base_address(&self) -> usize;
	fn get_process_name(&self) -> Option<&String>;
}

impl MemoryReaderType {
	pub fn read_type<T: Pod>(&mut self, address: usize) -> anyhow::Result<T> {
		let mut buf = vec![0u8; size_of::<T>()];
		self.read(address, &mut buf)?;
		Ok(*bytemuck::from_bytes::<T>(&buf))
	}
}

impl MemoryReader for MemoryReaderType {
	fn read(&mut self, address: usize, buf: &mut [u8]) -> anyhow::Result<()> {
		use MemoryReaderType::*;
		match self {
			Neptuwunium(reader) => reader.read(address, buf),
			Minidump(reader) => reader.read(address, buf),
			#[cfg(target_os = "linux")]
			Linux(reader) => reader.read(address, buf),
			#[cfg(target_os = "windows")]
			Windows(reader) => reader.read(address, buf),
		}
	}

	fn get_base_address(&self) -> usize {
		use MemoryReaderType::*;
		match self {
			Neptuwunium(reader) => reader.get_base_address(),
			Minidump(reader) => reader.get_base_address(),
			#[cfg(target_os = "linux")]
			Linux(reader) => reader.get_base_address(),
			#[cfg(target_os = "windows")]
			Windows(reader) => reader.get_base_address(),
		}
	}

	fn get_process_name(&self) -> Option<&String> {
		use MemoryReaderType::*;
		match self {
			Neptuwunium(reader) => reader.get_process_name(),
			Minidump(reader) => reader.get_process_name(),
			#[cfg(target_os = "linux")]
			Linux(reader) => reader.get_process_name(),
			#[cfg(target_os = "windows")]
			Windows(reader) => reader.get_process_name(),
		}
	}
}

impl Read for MemoryCursor {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		if let Err(err) = self.reader.read(self.pos, buf) { Err(std::io::Error::other(err)) } else { Ok(buf.len()) }
	}
}

impl Seek for MemoryCursor {
	fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
		match pos {
			SeekFrom::Start(pos) => {
				self.pos = pos as usize;
				Ok(self.pos as u64)
			}
			SeekFrom::End(_) => Err(std::io::Error::new(ErrorKind::Unsupported, anyhow!("SeekEnd is not supported"))),
			SeekFrom::Current(pos) => {
				self.pos += pos as usize;
				Ok(self.pos as u64)
			}
		}
	}
}
