// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use anyhow::{Result, anyhow};
use log::error;
use windows::Win32::Foundation::{HANDLE, HMODULE, MAX_PATH};
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::ProcessStatus::{
	EnumProcessModules, GetModuleFileNameExW, GetModuleInformation, GetProcessImageFileNameW, MODULEINFO,
};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_VM_READ};

use crate::engine::LuminousPointer;
use crate::memory::MemoryRead;

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

	pub fn from_handle(process: HANDLE) -> Win32MemoryReader {
		Win32MemoryReader {
			process,
		}
	}
}

impl MemoryRead for Win32MemoryReader {
	fn read(&mut self, address: LuminousPointer<()>, buf: &mut [u8]) -> Result<()> {
		match unsafe { ReadProcessMemory(self.process, address.inner as *const _, buf.as_mut_ptr() as *mut _, buf.len(), None) } {
			Ok(_) => Ok(()),
			Err(err) => Err(anyhow!("ReadProcessMemory error: {}", err)),
		}
	}

	fn get_base_address(&self) -> LuminousPointer<()> {
		get_base_address_from_process(self.process, self.get_process_name())
	}

	fn get_process_name(&self) -> Option<String> {
		get_process_name_pid(self.process)
	}
}

pub fn get_process_name_pid(process: HANDLE) -> Option<String> {
	let mut module_path = vec![0u16; MAX_PATH as usize];
	let len = unsafe { GetProcessImageFileNameW(process, &mut module_path) };
	if len == 0 {
		None
	} else {
		Some(String::from_utf16_lossy(&module_path[..len as usize]).split(&['\\', '/'][..]).next_back()?.to_string())
	}
}

pub(crate) fn get_base_address_from_process(process: HANDLE, own_path: Option<String>) -> LuminousPointer<()> {
	let mut modules: [HMODULE; 1024] = [HMODULE::default(); 1024];
	let mut cb_needed: u32 = 0;

	let mut module_path = vec![0u16; MAX_PATH as usize];
	let default = LuminousPointer::new(0x140000000);
	let own_path = match own_path {
		Some(path) => path,
		None => return default,
	};

	match unsafe { EnumProcessModules(process, modules.as_mut_ptr(), size_of_val(&modules) as u32, &mut cb_needed) } {
		Ok(_) => {
			for module in modules.iter().take(cb_needed as usize / size_of::<HMODULE>()) {
				let len = unsafe { GetModuleFileNameExW(Some(process), Some(*module), &mut module_path) };
				if len == 0 {
					continue;
				}

				let path = String::from_utf16_lossy(&module_path[..len as usize]);

				if path.ends_with(&own_path) {
					let mut mod_info: MODULEINFO = unsafe { std::mem::zeroed() };
					return match unsafe { GetModuleInformation(process, *module, &mut mod_info, size_of::<MODULEINFO>() as u32) } {
						Ok(_) => (mod_info.lpBaseOfDll as usize).into(),
						Err(err) => {
							error!("GetModuleInformation error: {:?}", err);
							default
						}
					};
				}
			}

			default
		}
		_ => {
			error!("EnumProcessModules error");
			default
		}
	}
}
