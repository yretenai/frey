// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::ffi::c_void;
use std::ops::BitAnd;
use std::ptr;

use anyhow::{Result, bail};
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::System::Memory::{MEMORY_BASIC_INFORMATION, PAGE_PROTECTION_FLAGS, VirtualProtect};
use windows::Win32::System::Memory::{PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_READONLY, PAGE_READWRITE, VirtualQuery};
use windows::Win32::System::ProcessStatus::GetProcessImageFileNameW;
use windows::Win32::System::Threading::GetCurrentProcess;

use crate::engine::LuminousPointer;
use crate::memory::MemoryReader;
use crate::memory::windows_mem::get_base_address_from_process;

pub struct Win32LocalMemoryReader {
	pub query_if_safe: bool,
}

impl Win32LocalMemoryReader {
	pub fn is_address_safe(&self, address: LuminousPointer, size: usize) -> Result<()> {
		if !self.query_if_safe {
			return Ok(());
		}

		let mut query: MEMORY_BASIC_INFORMATION = MEMORY_BASIC_INFORMATION::default();
		let result = unsafe { VirtualQuery(Some(address.0 as *const c_void), &mut query, size_of::<MEMORY_BASIC_INFORMATION>()) };

		if result == 0 {
			bail!("VirtualQuery failed: {:#016x}", result);
		}

		if !query.Protect.contains(PAGE_READWRITE)
			&& !query.Protect.contains(PAGE_READONLY)
			&& !query.Protect.contains(PAGE_EXECUTE_READ)
			&& !query.Protect.contains(PAGE_EXECUTE_READWRITE)
		{
			bail!("no permissions");
		}

		if size > 0 && query.RegionSize - (address.into() - query.BaseAddress as usize) < size {
			bail!("not enough data");
		}

		Ok(())
	}

	pub fn write(&self, address: LuminousPointer, buf: &[u8]) -> Result<()> {
		self.is_address_safe(address, buf.len())?;

		let mut query: MEMORY_BASIC_INFORMATION = MEMORY_BASIC_INFORMATION::default();
		let result = unsafe { VirtualQuery(Some(address.0 as *const c_void), &mut query, size_of::<MEMORY_BASIC_INFORMATION>()) };
		if result == 0 {
			bail!("VirtualQuery failed: {}", result);
		}

		let mut old_flags: PAGE_PROTECTION_FLAGS = query.Protect;

		if !query.Protect.contains(PAGE_READWRITE) || !query.Protect.contains(PAGE_EXECUTE_READWRITE) {
			let new_flags: PAGE_PROTECTION_FLAGS = query.Protect.bitand(PAGE_READWRITE);
			let result = unsafe { VirtualProtect(query.BaseAddress, query.RegionSize, new_flags, &mut old_flags) };
			if let Err(err) = result {
				bail!("VirtualProtect failed: {}", err);
			}
		}

		unsafe {
			ptr::copy_nonoverlapping(buf.as_ptr(), query.BaseAddress as *mut u8, buf.len());
		}

		if !query.Protect.contains(PAGE_READWRITE) || !query.Protect.contains(PAGE_EXECUTE_READWRITE) {
			let result = unsafe { VirtualProtect(query.BaseAddress, query.RegionSize, old_flags, &mut old_flags) };
			if let Err(err) = result {
				bail!("VirtualProtect failed: {}", err);
			}
		}

		Ok(())
	}
}

impl MemoryReader for Win32LocalMemoryReader {
	fn read(&mut self, address: LuminousPointer, buf: &mut [u8]) -> Result<()> {
		self.is_address_safe(address, buf.len())?;

		unsafe {
			ptr::copy_nonoverlapping(address.0 as *const u8, buf.as_mut_ptr(), buf.len());
		}

		Ok(())
	}

	fn get_base_address(&self) -> usize {
		get_base_address_from_process(unsafe { GetCurrentProcess() }, self.get_process_name())
	}

	fn get_process_name(&self) -> Option<&String> {
		let mut module_path = vec![0u16; MAX_PATH as usize];
		let len = unsafe { GetProcessImageFileNameW(GetCurrentProcess(), &mut module_path) };
		if len == 0 { None } else { Some(&String::from_utf16_lossy(&module_path[..len as usize])) }
	}
}
