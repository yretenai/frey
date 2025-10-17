// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::ffi::c_void;
use std::ptr;

use anyhow::{Result, bail};
use windows::Win32::System::Memory::{MEMORY_BASIC_INFORMATION, PAGE_PROTECTION_FLAGS, VirtualProtect};
use windows::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, VirtualQuery};
use windows::Win32::System::Threading::GetCurrentProcess;

use crate::engine::LuminousPointer;
use crate::memory::MemoryRead;
use crate::memory::windows_mem::{get_base_address_from_process, get_process_name_pid};

#[derive(Copy, Clone)]
pub struct Win32LocalMemoryReader {
	pub query_if_safe: bool,
}

impl Win32LocalMemoryReader {
	pub fn new(safe: bool) -> Self {
		Self {
			query_if_safe: safe,
		}
	}

	pub fn is_address_safe(&self, address: LuminousPointer<()>, size: usize) -> Result<()> {
		if !self.query_if_safe {
			return Ok(());
		}

		let mut query: MEMORY_BASIC_INFORMATION = MEMORY_BASIC_INFORMATION::default();
		let result = unsafe { VirtualQuery(Some(address.inner as *const c_void), &mut query, size_of::<MEMORY_BASIC_INFORMATION>()) };

		if result == 0 {
			bail!("VirtualQuery failed: {}", std::io::Error::last_os_error());
		}

		if size > 0 && (query.RegionSize - (address - query.BaseAddress as usize).inner as usize) < size {
			bail!("not enough data");
		}

		Ok(())
	}

	pub fn write(&self, address: LuminousPointer<()>, buf: &[u8]) -> Result<()> {
		self.is_address_safe(address, buf.len())?;

		let mut old_flags: PAGE_PROTECTION_FLAGS = Default::default();
		let result = unsafe { VirtualProtect(address.unsafe_ptr() as *const c_void, buf.len(), PAGE_EXECUTE_READWRITE, &mut old_flags) };
		if let Err(err) = result {
			bail!("VirtualProtect failed: {}", err);
		}

		unsafe {
			ptr::copy_nonoverlapping(buf.as_ptr(), address.unsafe_mut_ptr() as *mut u8, buf.len());
		}

		let result = unsafe { VirtualProtect(address.unsafe_ptr() as *const c_void, buf.len(), old_flags, &mut old_flags) };
		if let Err(err) = result {
			bail!("VirtualProtect failed: {}", err);
		}

		Ok(())
	}
}

impl MemoryRead for Win32LocalMemoryReader {
	fn read(&mut self, address: LuminousPointer<()>, buf: &mut [u8]) -> Result<()> {
		self.is_address_safe(address, buf.len())?;

		unsafe {
			ptr::copy_nonoverlapping(address.inner as *const u8, buf.as_mut_ptr(), buf.len());
		}

		Ok(())
	}

	fn get_base_address(&self) -> LuminousPointer<()> {
		get_base_address_from_process(unsafe { GetCurrentProcess() }, self.get_process_name())
	}

	fn get_process_name(&self) -> Option<String> {
		get_process_name_pid(unsafe { GetCurrentProcess() })
	}
}
