// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::ffi::{OsString, c_void};
use std::os::windows::ffi::OsStrExt;
use std::process::exit;

use log::{error, info};
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, VirtualAllocEx};
use windows::Win32::System::Threading::{
	CREATE_SUSPENDED, CreateProcessW, CreateRemoteThread, GetCurrentProcess, GetExitCodeThread, INFINITE, LPTHREAD_START_ROUTINE,
	PROCESS_INFORMATION, ResumeThread, STARTUPINFOW, WaitForSingleObject,
};
use windows::core::{PCSTR, PCWSTR};
use witch_common::engine::LuminousGame;
use witch_common::memory::determine_game_type;
use witch_common::memory::windows_mem::get_process_name_pid;

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

	if let Err(err) =
		unsafe { CreateProcessW(PCWSTR(cmd.as_mut_ptr()), None, None, None, false, CREATE_SUSPENDED, None, None, &startup, &mut process) }
	{
		anyhow::bail!("failed to create process: {}", err);
	}

	if let Err(err) = attach_dll(process.hProcess) {
		error!("failed to attach dll: {}", err);
	}

	unsafe {
		ResumeThread(process.hThread);
		if let Err(err) = CloseHandle(process.hThread) {
			error!("failed to close thread handle: {}", err);
		}
		if let Err(err) = CloseHandle(process.hProcess) {
			error!("failed to close process handle: {}", err);
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
	let dll_size = dll_path.len() * size_of::<u16>();
	let dll_addr = unsafe { VirtualAllocEx(handle, None, dll_size, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE) };

	if dll_addr.is_null() {
		error!("could not allocate dll path");
		return Ok(());
	}

	unsafe { WriteProcessMemory(handle, dll_addr, dll_path.as_ptr() as *const c_void, dll_size, None) }?;

	let kernel_path = wide_string(&OsString::from("kernel32.dll"));
	let kernel32 = unsafe { GetModuleHandleW(PCWSTR(kernel_path.as_ptr())) }?;

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
