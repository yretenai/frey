// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::ffi::c_void;

use flexi_logger::{Duplicate, FileSpec, Logger, detailed_format};
use log::{error, info};
use windows::Win32::Foundation::{BOOL, FALSE, HINSTANCE, HMODULE, TRUE};
use windows::Win32::System::LibraryLoader::DisableThreadLibraryCalls;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use witch_common::engine::{LuminousGame, LuminousPointer};
use witch_common::memory::windows_local_mem::Win32LocalMemoryReader;
use witch_common::memory::{MemoryCursor, MemoryRead, MemoryReader};

use crate::hud::ScarletRender;

// noinspection RsFunctionNaming
#[unsafe(no_mangle)]
pub extern "system" fn DllMain(module: HINSTANCE, reason: u32, _: *const c_void) -> BOOL {
	if reason == DLL_PROCESS_ATTACH {
		_ = unsafe { DisableThreadLibraryCalls(HMODULE(module.0)) };
		if let Err(err) = scarlet_main(module) {
			error!("fatal error! {}", err);
			return FALSE;
		}
	}
	TRUE
}

fn scarlet_main(module: HINSTANCE) -> anyhow::Result<()> {
	Logger::try_with_env_or_str("debug")?
		.log_to_file(FileSpec::default().basename("scarlet").suppress_timestamp())
		.duplicate_to_stdout(Duplicate::Info)
		.format(detailed_format)
		.start()?;

	hudhook::alloc_console()?;

	let mut writer = Win32LocalMemoryReader {
		query_if_safe: true,
	};

	let (game, base_addr) = patch_exe(&mut writer)?;

	std::thread::spawn(move || {
		use hudhook::Hudhook;
		use hudhook::hooks::dx12::ImguiDx12Hooks;

		let mut reader = MemoryCursor::new(MemoryReader::Process(Win32LocalMemoryReader::new(true)));

		if game == LuminousGame::FORSPOKEN {
			info!("waiting until Luminous::Game::Application shows up");

			let application_ptr = LuminousPointer::<LuminousPointer<()>>::new(base_addr.inner + 0x79d34a8u64);

			while !application_ptr.read(&mut reader.inner).unwrap_or_default().is_valid() {
				info!("still not init...");
				std::thread::sleep(std::time::Duration::from_secs(1));
			}
		}

		info!("sleeping for an extra 10 seconds");

		std::thread::sleep(std::time::Duration::from_secs(10));

		info!("reading objects");

		let render = match ScarletRender::new(reader) {
			Ok(render) => render,
			Err(err) => {
				error!("failed getting renderer set up: {}", err);
				return;
			}
		};

		info!("hooking dx12");

		if let Err(e) = Hudhook::builder().with::<ImguiDx12Hooks>(render).with_hmodule(module).build().apply() {
			error!("hudhook error! {:?}", e);
		}
	});

	Ok(())
}

fn patch_bytes(writer: &Win32LocalMemoryReader, address: LuminousPointer<()>, bytes: &[u8]) -> anyhow::Result<()> {
	if let Err(err) = writer.write(address, bytes) { anyhow::bail!("failed to write bytes at {:?}: {}", address, err) } else { Ok(()) }
}

fn patch_exe(writer: &mut Win32LocalMemoryReader) -> anyhow::Result<(LuminousGame, LuminousPointer<()>)> {
	let process_name = writer.get_process_name().unwrap_or_default();
	info!("process: {}", process_name);
	let game = witch_common::memory::determine_game_type(&process_name);
	info!("game: {:?}", game);
	let base_address = writer.get_base_address();
	info!("base: {:?}", base_address);

	match game {
		LuminousGame::FinalFantasyXV => {
			for dll in [0x2ddaaa0, 0xeeb46e0, 0xeeb5120] {
				patch_bytes(writer, base_address + dll, &[0xc3])?; // dll checks, stub with ret
			}
		}
		LuminousGame::FORSPOKEN => {
			patch_bytes(writer, base_address + 0x3e2d9bb, &[0xeb])?; // dll check
			// also crashes
			// patch_bytes(writer, base_address + 0x0799bac, &[0x90, 0x90])?; // stub anti-debugger

			// patch out dxvk and wine checks because they restrict the engine to be better on the SteamDeck OOB
			patch_bytes(writer, base_address + 0x6b9c8b0, &[0x00])?; // wine_get_host_version string
			patch_bytes(writer, base_address + 0x6d502b0, &[0x00])?; // WINEDLLPATH string

			// these are also broken
			// patch_bytes(writer, base_address + 0x3e29068, &[0x90, 0x90, 0x90, 0x90, 0x90, 0x90])?; // wine check, disabled by breaking the string
			// patch_bytes(writer, base_address + 0x4db69d7, &[0x90, 0xe9, 0xb3, 0x01, 0x00, 0x00])?; // WINEPATH check, disabled by breaking the string
		}
		_ => {
			unreachable!()
		}
	}

	info!("patched");

	Ok((game, base_address))
}
