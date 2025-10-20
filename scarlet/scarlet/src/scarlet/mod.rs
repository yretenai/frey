// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

pub(crate) mod functions;
pub(crate) mod hud;

use std::ffi::c_void;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Not;
use std::str::FromStr;

use anyhow::bail;
use flexi_logger::{Duplicate, FileSpec, Logger, detailed_format};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use toml_edit::Table;
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

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum WinePatchMode {
	#[default]
	Auto,
	Disable,
	Enable,
	Force,
}

impl From<bool> for WinePatchMode {
	fn from(value: bool) -> Self {
		match value {
			true => WinePatchMode::Enable,
			false => WinePatchMode::Disable,
		}
	}
}

impl From<WinePatchMode> for bool {
	fn from(value: WinePatchMode) -> Self {
		!matches!(value, WinePatchMode::Disable)
	}
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Patches {
	#[serde(default = "default_bool::<true>")]
	pub dll_signature: bool,
	#[serde(default = "default_bool::<true>")]
	pub anti_debugger: bool,
	#[serde(default = "WinePatchMode::default")]
	pub wine: WinePatchMode,
	#[serde(default = "WinePatchMode::default")]
	pub dxvk: WinePatchMode,
	#[serde(default = "WinePatchMode::default")]
	pub steamdeck: WinePatchMode,
	#[serde(default = "default_bool::<true>")]
	pub wine_enable_directstorage: bool,
	#[serde(default = "default_bool::<true>")]
	pub wine_reenable_terrain_shader: bool,
	#[serde(default = "default_bool::<true>")]
	pub wine_reenable_xess: bool,
	#[serde(default = "default_bool::<true>")]
	pub wine_reenable_raytracing: bool,
	#[serde(default = "default_bool::<true>")]
	pub wine_allow_more_threads: bool,
	#[serde(default = "default_bool::<true>")]
	pub disable_benchmark_results: bool,
}

impl Patches {
	pub(crate) fn write_comments(document: &mut Table) {
		let root = document.decor_mut();
		root.set_prefix("# for wine, dxvk, and steamdeck options the options are:\n# auto = determine based on SteamOS and SteamDeck environment variables\n# disable, enable = disable or enable the patch\n# force = makes the game believe it to be true\n");

		for (mut key, _value) in document.iter_mut() {
			let comment = match key.get() {
				"dll_signature" => "# dll signature checking, allows for things like ReShade to load\n",
				"anti_debugger" => "# breaks the anti-debugger loop allowing for debuggers to be attached\n",
				"wine" => "# controls how the game checks if running under wine\n# note, this crashes when the benchmark results screen\n",
				"dxvk" => "# controls how the game checks for dxvk 1.0 to disable 1 shader\n",
				"steamdeck" => "# controls how the game checks if running on a SteamDeck\n",
				"wine_enable_directstorage" => {
					"# enables the game to use directstorage on wine\n# may crash on wine/proton 8 and earlier versions\n"
				}
				"wine_reenable_terrain_shader" => "# re-enables a terrain blending shader that is disabled on wine\n",
				"wine_reenable_xess" => "# re-enables XeSS\n",
				"wine_reenable_raytracing" => "# re-enables ray tracing (if supported)\n",
				"wine_allow_more_threads" => "# allows the game to use more than 6 threads\n# this is a steamdeck optimization\n",
				"disable_benchmark_results" => "# disables writing benchmark results\n# see wine_main_check\n",
				_ => continue,
			};

			let decor = key.leaf_decor_mut();
			decor.set_prefix(comment);
		}
	}
}

impl Default for Patches {
	fn default() -> Self {
		Patches {
			dll_signature: true,
			anti_debugger: true,
			dxvk: WinePatchMode::Auto,
			wine: WinePatchMode::Auto,
			steamdeck: WinePatchMode::Auto,
			wine_enable_directstorage: true,
			wine_reenable_terrain_shader: true,
			wine_reenable_xess: true,
			wine_reenable_raytracing: true,
			wine_allow_more_threads: true,
			disable_benchmark_results: true,
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
	let mut toml = toml_edit::DocumentMut::from_str(toml.as_str())?;
	let patches = toml.get_mut("patches").unwrap();
	let table = patches.as_table_mut().unwrap();
	Patches::write_comments(table);
	file.write_all(toml.to_string().as_bytes())?;
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

fn patch_bytes(
	writer: &mut Win32LocalMemoryReader,
	address: LuminousPointer<()>,
	bytes: &[u8],
	check_bytes: Option<&[u8]>,
) -> anyhow::Result<()> {
	debug!("patching bytes {:x?} at {:?}", bytes, address);
	let mut buf = vec![0u8; bytes.len()];
	if let Some(check_bytes) = check_bytes
		&& check_bytes.len() == bytes.len()
	{
		writer.read(address, &mut buf)?;
		if buf != check_bytes {
			bail!("not writing where we think we're writing! ({:?} yielded {:x?}, expected {:x?})", address, buf, check_bytes);
		}
	}

	if let Err(err) = writer.write(address, bytes) {
		bail!("failed to write bytes at {:?}: {}", address, err)
	} else {
		writer.read(address, &mut buf)?;
		if buf != bytes {
			bail!("malformed write at {:?}? yielded {:x?}", address, buf);
		}

		Ok(())
	}
}

fn patch_exe(writer: &mut Win32LocalMemoryReader, config: &Config) -> anyhow::Result<(LuminousGame, LuminousPointer<()>)> {
	let process_name = writer.get_process_name().unwrap_or_default();
	info!("process: {}", process_name);
	let game = witch_common::memory::determine_game_type(&process_name);
	info!("game: {:?}", game);
	let base_address = writer.get_base_address();
	info!("base: {:?}", base_address);

	let is_steamdeck = std::env::var("SteamDeck").map(|r| r.eq("1")).unwrap_or(false);
	let is_steamos: WinePatchMode = (std::env::var("SteamOS").map(|r| r.eq("1")).unwrap_or(false) || is_steamdeck).not().into();
	let is_steamdeck: WinePatchMode = is_steamdeck.not().into();

	let mut patches = config.patches;

	if patches.steamdeck == WinePatchMode::Auto {
		patches.steamdeck = is_steamdeck;
	}

	if patches.dxvk == WinePatchMode::Auto {
		patches.dxvk = is_steamos;
	}

	if patches.wine == WinePatchMode::Auto {
		patches.wine = is_steamos;
	}

	info!("patch config: {:?}", patches);

	match game {
		LuminousGame::FinalFantasyXV => {
			if patches.dll_signature {
				for dll in [0x2ddaaa0, 0xeeb46e0, 0xeeb5120] {
					patch_bytes(writer, base_address + dll, &[0xc3], None)?;
				}
			}
		}
		LuminousGame::FORSPOKEN => {
			if patches.dll_signature {
				patch_bytes(writer, base_address + 0x3e2d9bb, &[0xeb], Some(&[0x75]))?;
			}

			if patches.anti_debugger {
				patch_bytes(writer, base_address + 0x0799bac, &[0xeb], Some(&[0x74]))?;
			}

			match patches.steamdeck {
				WinePatchMode::Enable => {
					patch_bytes(
						writer,
						base_address + 0x3e29034,
						&[0xb8, 0x00, 0x00, 0x00, 0x00, 0x90],
						Some(&[0xff, 0x90, 0x10, 0x01, 0x00, 0x00]),
					)?;
				}
				WinePatchMode::Force => {
					patch_bytes(
						writer,
						base_address + 0x3e29034,
						&[0xb8, 0x01, 0x00, 0x00, 0x00, 0x90],
						Some(&[0xff, 0x90, 0x10, 0x01, 0x00, 0x00]),
					)?;
				}
				_ => {}
			}

			match patches.wine {
				WinePatchMode::Enable => {
					patch_bytes(writer, base_address + 0x6b9c8b0, &[0x00], Some(&[0x77]))?;
				}
				WinePatchMode::Force => {
					patch_bytes(
						writer,
						base_address + 0x3e29059,
						&[0x48, 0xc7, 0xc0, 0x01, 0x00, 0x00, 0x00, 0x90, 0x90],
						Some(&[0x48, 0x8b, 0xc8, 0xff, 0x15, 0x0e, 0x82, 0x28, 0x01]),
					)?;
				}
				_ => {
					if patches.wine_enable_directstorage {
						patch_bytes(writer, base_address + 0x4705a5b, &[0x90, 0x90], Some(&[0x75, 0x21]))?;
					}
					if patches.wine_reenable_terrain_shader {
						patch_bytes(
							writer,
							base_address + 0x4299712,
							&[0x90, 0x90, 0x90, 0x90, 0x90, 0x90],
							Some(&[0x0f, 0x85, 0xfd, 0x2d, 0x00, 0x00]),
						)?;
					}
					if patches.wine_reenable_xess {
						patch_bytes(
							writer,
							base_address + 0x43ff1a3,
							&[0x90, 0x90, 0x90, 0x90, 0x90, 0x90],
							Some(&[0x0f, 0x84, 0x7e, 0x00, 0x00, 0x00]),
						)?;
						patch_bytes(
							writer,
							base_address + 0x43ff6fd,
							&[0x90, 0x90, 0x90, 0x90, 0x90, 0x90],
							Some(&[0x0f, 0x84, 0x9a, 0x00, 0x00, 0x00]),
						)?;
						patch_bytes(writer, base_address + 0x43ff940, &[0xeb], Some(&[0x74]))?;
					}
					if patches.wine_reenable_raytracing {
						patch_bytes(writer, base_address + 0x429f9d9, &[0xeb], Some(&[0x74]))?;
						patch_bytes(
							writer,
							base_address + 0x43e8860,
							&[0x90, 0x90, 0x90, 0x90, 0x90, 0x90],
							Some(&[0x0f, 0x85, 0x42, 0x02, 0x00, 0x00]),
						)?;
					}
					if patches.wine_allow_more_threads {
						patch_bytes(writer, base_address + 0x3e32b2a, &[0xeb], Some(&[0x74]))?;
					}
				}
			}

			match patches.dxvk {
				WinePatchMode::Enable => {
					patch_bytes(writer, base_address + 0x6d502b0, &[0x00], Some(&[0x57]))?;
				}
				WinePatchMode::Force => {
					patch_bytes(writer, base_address + 0x6d502b0, &[0xeb, 0x12], Some(&[0x74, 0x19]))?;
				}
				_ => {}
			}

			if patches.disable_benchmark_results {
				patch_bytes(writer, base_address + 0x3185590, &[0xc3], Some(&[0x48]))?;
			}
		}
		_ => {
			unreachable!()
		}
	}

	Ok((game, base_address))
}
