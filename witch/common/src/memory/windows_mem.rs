// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use anyhow::{Result, anyhow};
use windows::Win32::Foundation::{HANDLE, HMODULE, MAX_PATH};
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::ProcessStatus::{
	EnumProcessModules, GetModuleFileNameExW, GetModuleInformation, GetProcessImageFileNameW, MODULEINFO,
};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_VM_READ};

use crate::memory::MemoryReader;

pub struct Win32MemoryReader {
	process: HANDLE,
}

impl Win32MemoryReader {
	pub fn new(pid: u32) -> Result<Win32MemoryReader> {
		match unsafe { OpenProcess(PROCESS_VM_READ, false, pid) } {
			Ok(process) => match process.is_invalid() {
				true => Err(anyhow!("invalid handle")),
				false => Ok(Win32MemoryReader {
					process,
				}),
			},
			Err(err) => Err(anyhow!("OpenProcess error: {}", err)),
		}
	}
}

impl MemoryReader for Win32MemoryReader {
	fn read(&mut self, address: usize, buf: &mut [u8]) -> Result<()> {
		match unsafe { ReadProcessMemory(self.process, address as *const _, buf.as_mut_ptr() as *mut _, buf.len(), None) } {
			Ok(_) => Ok(()),
			Err(err) => Err(anyhow!("ReadProcessMemory error: {}", err)),
		}
	}

	fn get_base_address(&self) -> usize {
		let mut modules: [HMODULE; 1024] = [HMODULE::default(); 1024];
		let mut cb_needed: u32 = 0;

		let mut module_path = vec![0u16; MAX_PATH as usize];
		let own_path = match self.get_process_name() {
			Some(path) => path,
			None => return 0,
		};

		match unsafe { EnumProcessModules(self.process, modules.as_mut_ptr(), size_of_val(&modules) as u32, &mut cb_needed) } {
			Ok(result) if result => {
				for i in 0..(cb_needed as usize / size_of::<HMODULE>()) {
					let module = modules[i];
					let len = unsafe { GetModuleFileNameExW(Some(self.process), Some(module), &mut module_path) };
					if len > 0 && String::from_utf16_lossy(&module_path[..len as usize]).eq(own_path) {
						let mut mod_info: MODULEINFO = unsafe { std::mem::zeroed() };
						return match unsafe { GetModuleInformation(self.process, module, &mut mod_info, size_of::<MODULEINFO>() as u32) } {
							Ok(_) => mod_info.lpBaseOfDll as usize,
							Err(_) => 0,
						};
					}
				}

				0
			}
			_ => 0,
		}
	}

	fn get_process_name(&self) -> Option<&String> {
		let mut module_path = vec![0u16; MAX_PATH as usize];
		let len = unsafe { GetProcessImageFileNameW(self.process, &mut module_path) };
		if len == 0 { None } else { Some(&String::from_utf16_lossy(&module_path[..len as usize])) }
	}
}
