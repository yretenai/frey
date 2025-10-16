// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::ffi::{OsString, c_void};
use std::ops::BitAnd;
use std::os::windows::ffi::OsStrExt;
use std::process::exit;

use log::{debug, error, info};
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::Memory::{
	MEM_COMMIT, MEM_RESERVE, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, PAGE_READWRITE, VirtualAllocEx,
	VirtualProtectEx, VirtualQueryEx,
};
use windows::Win32::System::Threading::{
	CREATE_SUSPENDED, CreateProcessW, CreateRemoteThread, GetCurrentProcess, GetExitCodeThread, INFINITE, LPTHREAD_START_ROUTINE,
	PROCESS_INFORMATION, ResumeThread, STARTUPINFOW, WaitForSingleObject,
};
use windows::core::{PCSTR, PCWSTR, PWSTR};
use witch_common::engine::{LuminousGame, LuminousPointer};
use witch_common::memory::windows_mem::{Win32MemoryReader, get_process_name_pid};
use witch_common::memory::{MemoryReader, determine_game_type};

pub fn scarlet_main() -> anyhow::Result<()> {
	let own_process = unsafe { GetCurrentProcess() };
	let process_name = get_process_name_pid(own_process).unwrap_or_default();
	let game_type = determine_game_type(&process_name);

	if game_type == LuminousGame::LuminousEngine {
		error!("please rename the game to scarlet-name.exe i.e. scarlet-FORSPOKEN.exe");
		exit(1);
	}

	let cwd = std::env::current_dir()?;
	let game_path = cwd.join(format!("scarlet-{}", process_name));
	if !game_path.exists() {
		error!("please rename the game to scarlet-name.exe i.e. scarlet-FORSPOKEN.exe");
		exit(2);
	}

	let mut cmd = wide_string(&game_path.into_os_string());
	let mut startup = STARTUPINFOW::default();
	let mut process = PROCESS_INFORMATION::default();
	startup.cb = size_of::<STARTUPINFOW>() as u32;

	info!("starting game");

	if let Err(err) = unsafe {
		CreateProcessW(
			PCWSTR(cmd.as_mut_ptr()),
			Some(PWSTR(cmd.as_mut_ptr())),
			None,
			None,
			false,
			CREATE_SUSPENDED,
			None,
			None,
			&startup,
			&mut process,
		)
	} {
		anyhow::bail!("failed to create process: {}", err);
	}

	info!("patching...");

	if let Err(err) = patch_exe(process.hProcess, game_type) {
		error!("failed to patch executable: {}", err);
	}

	if let Err(err) = attach_dll(process.hProcess) {
		error!("failed to attach dll: {}", err);
	}

	info!("finished patching, launching game");

	unsafe {
		ResumeThread(process.hThread);
		if let Err(err) = CloseHandle(process.hThread) {
			error!("failed to close thread: {}", err);
		}
		if let Err(err) = CloseHandle(process.hProcess) {
			error!("failed to close process: {}", err);
		}
	}

	Ok(())
}

fn wide_string(s: &OsString) -> Vec<u16> {
	s.encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>()
}

fn c_string(s: &str) -> Vec<u8> {
	s.bytes().chain(std::iter::once(0u8)).collect::<Vec<u8>>()
}

pub fn write(handle: HANDLE, address: LuminousPointer<()>, buf: &[u8]) -> windows::core::Result<()> {
	let mut query: MEMORY_BASIC_INFORMATION = MEMORY_BASIC_INFORMATION::default();
	let result =
		unsafe { VirtualQueryEx(handle, Some(address.unsafe_ptr() as *const c_void), &mut query, size_of::<MEMORY_BASIC_INFORMATION>()) };
	if result == 0 {
		return Err(windows::core::Error::from_thread());
	}

	let mut old_flags: PAGE_PROTECTION_FLAGS = query.Protect;

	if !query.Protect.contains(PAGE_READWRITE) || !query.Protect.contains(PAGE_EXECUTE_READWRITE) {
		let new_flags: PAGE_PROTECTION_FLAGS = query.Protect.bitand(PAGE_READWRITE);
		unsafe { VirtualProtectEx(handle, query.BaseAddress, query.RegionSize, new_flags, &mut old_flags) }?;
	}

	unsafe { WriteProcessMemory(handle, address.unsafe_ptr() as *const c_void, buf.as_ptr() as *const c_void, buf.len(), None) }?;

	if !query.Protect.contains(PAGE_READWRITE) || !query.Protect.contains(PAGE_EXECUTE_READWRITE) {
		unsafe { VirtualProtectEx(handle, query.BaseAddress, query.RegionSize, old_flags, &mut old_flags) }?;
	}

	Ok(())
}

fn patch_bytes(handle: HANDLE, address: LuminousPointer<()>, bytes: &[u8]) -> anyhow::Result<()> {
	debug!("patching {:?} with bytes {:?}", address, bytes);
	if let Err(err) = write(handle, address, bytes) { anyhow::bail!("failed to write bytes at {:?}: {}", address, err) } else { Ok(()) }
}

fn patch_exe(handle: HANDLE, game: LuminousGame) -> anyhow::Result<()> {
	let base_address = Win32MemoryReader::from_handle(handle).get_base_address();
	match game {
		LuminousGame::FinalFantasyXV => {
			for dll in [0x2ddaaa0, 0xeeb46e0, 0xeeb5120] {
				patch_bytes(handle, base_address + dll, &[0xc3])?; // dll checks, stub with ret
			}
		}
		LuminousGame::FORSPOKEN => {
			patch_bytes(handle, base_address + 0x3e2d9bb, &[0xeb])?; // dll check

			// patch out dxvk and wine checks because they restrict the engine to be better on the SteamDeck OOB
			patch_bytes(handle, base_address + 0x6b9c8b0, &[0x00])?; // wine_get_host_version string
			patch_bytes(handle, base_address + 0x3e29068, &[0x90, 0x90, 0x90, 0x90, 0x90, 0x90])?; // wine check
			patch_bytes(handle, base_address + 0x4db69d7, &[0x90, 0xe9, 0xb3, 0x01, 0x00, 0x00])?; // WINEPATH check
		}
		_ => {
			unreachable!()
		}
	}

	Ok(())
}

fn attach_dll(handle: HANDLE) -> windows::core::Result<()> {
	let cwd = match std::env::current_dir() {
		Ok(d) => d,
		Err(_) => return Ok(()),
	};

	let dll_path = cwd.join("scarlet.dll");

	if !dll_path.exists() {
		error!("scarlet.dll does not exist");
		return Ok(());
	}

	let dll_path = wide_string(&dll_path.into_os_string());
	let dll_addr = unsafe { VirtualAllocEx(handle, None, dll_path.len(), MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE) };

	if dll_addr.is_null() {
		error!("could not allocate dll path");
		return Ok(());
	}

	unsafe { WriteProcessMemory(handle, dll_addr, dll_path.as_ptr() as *const c_void, dll_path.len(), None) }?;

	let kernel_path = wide_string(&OsString::from("kernel32.dll"));
	let kernel32 = unsafe { LoadLibraryW(PCWSTR(kernel_path.as_ptr())) }?;

	let proc = c_string("LoadLibraryW");
	let load_library: LPTHREAD_START_ROUTINE = match unsafe { GetProcAddress(kernel32, PCSTR(proc.as_ptr())) } {
		None => unsafe {
			error!("could not load LoadLibraryW: {:?}", GetLastError());
			return Ok(());
		},
		Some(handle) => {
			type Proc = unsafe extern "system" fn() -> isize;
			type ThreadStartRoute = unsafe extern "system" fn(*mut c_void) -> u32;
			Some(unsafe { std::mem::transmute::<Proc, ThreadStartRoute>(handle) })
		}
	};

	unsafe {
		let thread = CreateRemoteThread(handle, None, 0, load_library, Some(dll_addr), 0, None)?;
		if thread.is_invalid() {
			error!("could not create thread: {:?}", GetLastError());
			return Ok(());
		}
		info!("remote thread for LoadLibraryW loaded, waiting...");
		WaitForSingleObject(thread, INFINITE);
		let mut exit_code = 0;
		GetExitCodeThread(thread, &mut exit_code)?;
		info!("thread exited with code {}", exit_code);
		CloseHandle(thread)?;
		info!("remote thread for LoadLibraryW closed");
	};

	Ok(())
}
