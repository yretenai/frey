// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

#[cfg(target_os = "linux")]
pub mod linux_mem;
pub(crate) mod linux_proc;
pub mod neptuwunium_dump;
#[cfg(target_os = "windows")]
pub mod windows_local_mem;
#[cfg(target_os = "windows")]
pub mod windows_mem;
#[cfg(feature = "minidump")]
pub mod windows_minidump;

use std::io::{ErrorKind, Read, Seek, SeekFrom};

use anyhow::{Result, anyhow};
use bytemuck::Pod;

use crate::engine::{LuminousGame, LuminousPointer};

pub enum MemoryReaderType {
	#[cfg(target_os = "windows")]
	Process(windows_local_mem::Win32LocalMemoryReader),
	Neptuwunium(neptuwunium_dump::Np93DumpReader),
	#[cfg(feature = "minidump")]
	Minidump(windows_minidump::Win32DumpReader),
	#[cfg(target_os = "linux")]
	Linux(linux_mem::LinuxMemoryReader),
	#[cfg(target_os = "windows")]
	Windows(windows_mem::Win32MemoryReader),
}

pub struct MemoryCursor {
	pos: usize,
	pub inner: MemoryReaderType,
}

impl MemoryCursor {
	pub fn new(reader: MemoryReaderType) -> Self {
		Self {
			pos: reader.get_base_address().inner as usize,
			inner: reader,
		}
	}
}

pub trait MemoryReader {
	/// reads bytes at the specified address
	fn read(&mut self, address: LuminousPointer<()>, buf: &mut [u8]) -> Result<()>;
	/// reads bytes at the specified address
	fn get_base_address(&self) -> LuminousPointer<()>;
	/// gets the base address of the main module of the process
	fn get_process_name(&self) -> Option<String>;
}

pub fn determine_game_type(name: &str) -> LuminousGame {
	match name.to_lowercase().strip_prefix("scarlet-").unwrap_or(name) {
		"forspoken.exe" => LuminousGame::FORSPOKEN,
		// "forspokendemo.exe" => LuminousGame::FORSPOKENDemo,
		"ffxv_s.exe" => LuminousGame::FinalFantasyXV,
		_ => LuminousGame::LuminousEngine,
	}
}

impl MemoryReaderType {
	/// reads a given type at the specified address
	pub fn read_type<T: Pod>(&mut self, address: LuminousPointer<T>) -> Result<T> {
		let mut buf = vec![0u8; size_of::<T>()];
		self.read(address.cast(), &mut buf)?;
		Ok(*bytemuck::from_bytes::<T>(&buf))
	}

	/// determines which game is being run by process name
	pub fn game_type(&self) -> LuminousGame {
		determine_game_type(&self.get_process_name().unwrap_or_default())
	}

	/// reads bytes at the specified address
	pub fn read(&mut self, address: LuminousPointer<()>, buf: &mut [u8]) -> Result<()> {
		use MemoryReaderType::*;
		match self {
			#[cfg(target_os = "windows")]
			Process(reader) => reader.read(address, buf),
			Neptuwunium(reader) => reader.read(address, buf),
			#[cfg(feature = "minidump")]
			Minidump(reader) => reader.read(address, buf),
			#[cfg(target_os = "linux")]
			Linux(reader) => reader.read(address, buf),
			#[cfg(target_os = "windows")]
			Windows(reader) => reader.read(address, buf),
		}
	}

	/// gets the base address of the main module of the process
	pub fn get_base_address(&self) -> LuminousPointer<()> {
		use MemoryReaderType::*;
		match self {
			#[cfg(target_os = "windows")]
			Process(reader) => reader.get_base_address(),
			Neptuwunium(reader) => reader.get_base_address(),
			#[cfg(feature = "minidump")]
			Minidump(reader) => reader.get_base_address(),
			#[cfg(target_os = "linux")]
			Linux(reader) => reader.get_base_address(),
			#[cfg(target_os = "windows")]
			Windows(reader) => reader.get_base_address(),
		}
	}

	/// gets the process name of the memory being read
	pub fn get_process_name(&self) -> Option<String> {
		use MemoryReaderType::*;
		match self {
			#[cfg(target_os = "windows")]
			Process(reader) => reader.get_process_name(),
			Neptuwunium(reader) => reader.get_process_name(),
			#[cfg(feature = "minidump")]
			Minidump(reader) => reader.get_process_name(),
			#[cfg(target_os = "linux")]
			Linux(reader) => reader.get_process_name(),
			#[cfg(target_os = "windows")]
			Windows(reader) => reader.get_process_name(),
		}
	}

	/// checks if the reader and the process shares the same address space (i.e. reading own memory)
	pub fn is_same_address_space(&self) -> bool {
		match self {
			#[cfg(target_os = "windows")]
			MemoryReaderType::Windows(_) => true,
			_ => false,
		}
	}
}

impl Read for MemoryCursor {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		let pos: LuminousPointer<()> = self.pos.into();
		self.pos += buf.len();
		if let Err(err) = self.inner.read(pos, buf) { Err(std::io::Error::other(err)) } else { Ok(buf.len()) }
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
