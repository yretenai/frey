// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

pub(crate) mod functions;
pub(crate) mod hud;

use std::ffi::c_void;
use std::fs::File;
use std::io::{Read, Write};

use anyhow::bail;
use flexi_logger::{Duplicate, FileSpec, Logger, detailed_format};
use log::{error, info};
use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::{BOOL, FALSE, HINSTANCE, HMODULE, TRUE};
use windows::Win32::System::LibraryLoader::DisableThreadLibraryCalls;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use witch_common::engine::{LuminousGame, LuminousPointer};
use witch_common::memory::MemoryRead;
use witch_common::memory::windows_local_mem::Win32LocalMemoryReader;

use crate::scarlet::hud::ScarletRender;

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

pub const fn default_bool<const V: bool>() -> bool {
	V
}

pub fn default_log_level() -> String {
	"debug".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patches {
	#[serde(default = "default_bool::<true>")]
	pub dll_signature: bool,
	#[serde(default = "default_bool::<true>")]
	pub anti_debugger: bool,
	#[serde(default = "default_bool::<true>")]
	pub dxvk_check: bool,
	#[serde(default = "default_bool::<true>")]
	pub wine_check: bool,
}

impl Patches {
	pub(crate) fn any(&self) -> bool {
		self.dll_signature || self.anti_debugger || self.wine_check || self.dxvk_check
	}
}

impl Default for Patches {
	fn default() -> Self {
		Patches {
			dll_signature: true,
			anti_debugger: true,
			dxvk_check: true,
			wine_check: true,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	#[serde(default = "default_bool::<true>")]
	pub enable_hud: bool,
	#[serde(default = "default_log_level")]
	pub log_level: String,
	#[serde(default = "Patches::default")]
	pub patches: Patches,
}

impl Default for Config {
	fn default() -> Self {
		Config {
			enable_hud: true,
			log_level: "debug".to_string(),
			patches: Default::default(),
		}
	}
}

fn scarlet_main(module: HINSTANCE) -> anyhow::Result<()> {
	let config = read_config().unwrap_or_default();
	_ = save_config(&config);

	Logger::try_with_env_or_str(&config.log_level)?
		.log_to_file(FileSpec::default().basename("scarlet").suppress_timestamp())
		.duplicate_to_stdout(Duplicate::Info)
		.format(detailed_format)
		.start()?;

	log_panics::init();

	let mut writer = Win32LocalMemoryReader {
		query_if_safe: true,
	};

	let (game, _) = patch_exe(&mut writer, &config)?;

	if !config.enable_hud {
		return Ok(());
	}

	std::thread::spawn(move || {
		use hudhook::Hudhook;

		if game == LuminousGame::FORSPOKEN {
			info!("hooking dx12");

			use hudhook::hooks::dx12::ImguiDx12Hooks;
			if let Err(e) = Hudhook::builder().with::<ImguiDx12Hooks>(ScarletRender::default()).with_hmodule(module).build().apply() {
				error!("hudhook error! {:?}", e);
			}
		} else {
			info!("hooking dx11");

			use hudhook::hooks::dx11::ImguiDx11Hooks;
			if let Err(e) = Hudhook::builder().with::<ImguiDx11Hooks>(ScarletRender::default()).with_hmodule(module).build().apply() {
				error!("hudhook error! {:?}", e);
			}
		}
	});

	Ok(())
}

fn save_config(config: &Config) -> anyhow::Result<()> {
	let mut file = File::create("scarlet.toml")?;
	let toml = toml::to_string_pretty(&config)?;
	file.write_all(toml.as_bytes())?;
	Ok(())
}

fn read_config() -> anyhow::Result<Config> {
	let mut file = File::open("scarlet.toml")?;
	let mut contents = String::new();
	file.read_to_string(&mut contents)?;
	match toml::from_str(&contents) {
		Ok(config) => Ok(config),
		Err(err) => {
			bail!("scarlet.toml is malformed: {}", err);
		}
	}
}

fn patch_bytes(writer: &Win32LocalMemoryReader, address: LuminousPointer<()>, bytes: &[u8]) -> anyhow::Result<()> {
	if let Err(err) = writer.write(address, bytes) { bail!("failed to write bytes at {:?}: {}", address, err) } else { Ok(()) }
}

fn patch_exe(writer: &mut Win32LocalMemoryReader, config: &Config) -> anyhow::Result<(LuminousGame, LuminousPointer<()>)> {
	let process_name = writer.get_process_name().unwrap_or_default();
	info!("process: {}", process_name);
	let game = witch_common::memory::determine_game_type(&process_name);
	info!("game: {:?}", game);
	let base_address = writer.get_base_address();
	info!("base: {:?}", base_address);

	if config.patches.any() {
		match game {
			LuminousGame::FinalFantasyXV => {
				if config.patches.dll_signature {
					for dll in [0x2ddaaa0, 0xeeb46e0, 0xeeb5120] {
						patch_bytes(writer, base_address + dll, &[0xc3])?; // dll checks, stub with ret
					}
				}
			}
			LuminousGame::FORSPOKEN => {
				// dll check
				if config.patches.dll_signature {
					patch_bytes(writer, base_address + 0x3e2d9bb, &[0xeb])?;
				}

				// stub anti-debugger
				if config.patches.anti_debugger {
					patch_bytes(writer, base_address + 0x0799bac, &[0xeb])?;
				}

				// wine_get_host_version string
				if config.patches.wine_check {
					patch_bytes(writer, base_address + 0x6b9c8b0, &[0x00])?; // patch out dxvk and wine checks because they restrict the engine to be better on the SteamDeck OOB
					// patch_bytes(writer, base_address + 0x3e29068, &[0x90, 0x90, 0x90, 0x90, 0x90, 0x90])?; // wine check, disabled by breaking the string
				}

				if config.patches.dxvk_check {
					patch_bytes(writer, base_address + 0x6d502b0, &[0x00])?; // WINEDLLPATH string
					// patch_bytes(writer, base_address + 0x4db69d7, &[0x90, 0xe9, 0xb3, 0x01, 0x00, 0x00])?; // WINEPATH check, disabled by breaking the string
				}
			}
			_ => {
				unreachable!()
			}
		}

		info!("patched");
	}

	Ok((game, base_address))
}
