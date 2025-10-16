// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::ffi::c_void;

use flexi_logger::{FileSpec, Logger, detailed_format};
use log::{error, info};
use windows::Win32::Foundation::{HINSTANCE, HMODULE};
use windows::Win32::System::LibraryLoader::DisableThreadLibraryCalls;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use witch_common::engine::{LuminousGame, LuminousPointer};
use witch_common::memory::MemoryRead;
use witch_common::memory::windows_local_mem::Win32LocalMemoryReader;

use crate::hud::ScarletRender;

// noinspection RsFunctionNaming
#[unsafe(no_mangle)]
pub extern "system" fn DllMain(module: HINSTANCE, reason: u32, _: *const c_void) -> i32 {
	if reason == DLL_PROCESS_ATTACH {
		_ = unsafe { DisableThreadLibraryCalls(HMODULE(module.0)) };
		if let Err(err) = scarlet_main(module) {
			error!("fatal error! {}", err);
			hudhook::eject();
		}
	}
	1
}

fn scarlet_main(module: HINSTANCE) -> anyhow::Result<()> {
	hudhook::alloc_console()?;
	hudhook::enable_console_colors();

	Logger::try_with_env_or_str("debug")?
		.log_to_file(FileSpec::default().basename("scarlet").suppress_timestamp())
		.format(detailed_format)
		.start()?;

	patch_exe()?;

	use hudhook::Hudhook;
	use hudhook::hooks::dx12::ImguiDx12Hooks;

	std::thread::spawn(move || {
		std::thread::sleep(std::time::Duration::from_secs(3));
		let render = match ScarletRender::new() {
			Ok(render) => render,
			Err(err) => {
				error!("failed getting renderer set up: {}", err);
				hudhook::eject();
				return;
			}
		};

		if let Err(e) = Hudhook::builder().with::<ImguiDx12Hooks>(render).with_hmodule(module).build().apply() {
			error!("hudhook error! {:?}", e);
			hudhook::eject();
		}
	});

	Ok(())
}

fn patch_bytes(writer: &Win32LocalMemoryReader, address: LuminousPointer<()>, bytes: &[u8]) -> anyhow::Result<()> {
	if let Err(err) = writer.write(address, bytes) { anyhow::bail!("failed to write bytes at {:?}: {}", address, err) } else { Ok(()) }
}

fn patch_exe() -> anyhow::Result<()> {
	let writer = Win32LocalMemoryReader {
		query_if_safe: true,
	};

	let process_name = writer.get_process_name().unwrap_or_default();
	let game = witch_common::memory::determine_game_type(&process_name);
	let base_address = writer.get_base_address();

	match game {
		LuminousGame::FinalFantasyXV => {
			for dll in [0x2ddaaa0, 0xeeb46e0, 0xeeb5120] {
				patch_bytes(&writer, base_address + dll, &[0xc3])?; // dll checks, stub with ret
			}
		}
		LuminousGame::FORSPOKEN => {
			patch_bytes(&writer, base_address + 0x3e2d9bb, &[0xeb])?; // dll check
			patch_bytes(&writer, base_address + 0x0799bac, &[0x90, 0x90])?; // stub anti-debugger

			// patch out dxvk and wine checks because they restrict the engine to be better on the SteamDeck OOB
			patch_bytes(&writer, base_address + 0x6b9c8b0, &[0x00])?; // wine_get_host_version string
			patch_bytes(&writer, base_address + 0x6d502b0, &[0x00])?; // WINEDLLPATH string

			// these are also broken
			// patch_bytes(&writer, base_address + 0x3e29068, &[0x90, 0x90, 0x90, 0x90, 0x90, 0x90])?; // wine check, disabled by breaking the string
			// patch_bytes(&writer, base_address + 0x4db69d7, &[0x90, 0xe9, 0xb3, 0x01, 0x00, 0x00])?; // WINEPATH check, disabled by breaking the string
		}
		_ => {
			unreachable!()
		}
	}

	info!("patched");

	Ok(())
}
