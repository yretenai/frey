// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use hudhook::imgui::{Context, ImStr, ImString, Io, Key, Ui};
use hudhook::{ImguiRenderLoop, MessageFilter, RenderContext};
use log::error;
use windows::Win32::UI::WindowsAndMessaging::ShowCursor;
use witch_common::engine::ebex::ObjectInfoRegistry;
use witch_common::engine::{LuminousCString, LuminousGame, LuminousPointer};
use witch_common::memory::windows_local_mem::Win32LocalMemoryReader;
use witch_common::memory::{MemoryCursor, MemoryReader};

use crate::scarlet::Config;
use crate::scarlet::functions::ScarletFunctions;
use crate::scarlet::functions::objects::ScarletObject;

#[derive(Default)]
pub struct ScarletRender {
	pub enable_introspection: bool,
	pub window_opened: bool,
	pub is_wine: bool,
	pub is_deck: bool,
	pub dxvk_version: Option<String>,
	pub time: f32,
	pub selected_package: i32,
}

// todo: can i omit globals lol
static EBEX: OnceLock<ObjectInfoRegistry> = OnceLock::new();
static PACKAGE: OnceLock<Mutex<Arc<Vec<ScarletObject>>>> = OnceLock::new();
static FUNCTIONS: OnceLock<ScarletFunctions> = OnceLock::new();

impl ScarletRender {
	pub fn new(config: Config) -> Self {
		Self {
			enable_introspection: config.enable_introspection,
			..Default::default()
		}
	}

	fn render_packages(&mut self, ui: &Ui, packages: Arc<Vec<ScarletObject>>) {
		let names: Vec<ImString> = packages.iter().map(|package| ImString::new(package.name().unwrap_or_default())).collect();
		let names: Vec<&ImStr> = names.iter().map(|e| e.as_ref()).collect();
		if ui.list_box("Packages", &mut self.selected_package, &names, 16) {}
	}
}

impl ImguiRenderLoop for ScarletRender {
	fn initialize<'a>(&'a mut self, _: &mut Context, _: &'a mut dyn RenderContext) {
		self.window_opened = true;
		self.time = 0.5;

		let mut reader = MemoryCursor::new(MemoryReader::Process(Win32LocalMemoryReader::new(true)));
		let base = reader.inner.get_base_address();

		EBEX.get_or_init(|| {
			ObjectInfoRegistry::new(&mut reader).unwrap_or_else(|err| {
				error!("failed loading ebex: {}", err);
				Default::default()
			})
		});

		PACKAGE.get_or_init(Default::default);

		FUNCTIONS.get_or_init(|| ScarletFunctions::new(base, EBEX.get().unwrap()));

		// MODULES.get_or_init(|| GameModules::new(&mut reader).ok());

		self.is_wine = match reader.inner.game_type() {
			LuminousGame::FORSPOKEN => {
				let address: LuminousPointer<u8> = base.cast() + 0x77c5630;
				address.read(&mut reader.inner).unwrap_or_default() != 0
			}
			_ => false,
		};

		self.is_deck = match reader.inner.game_type() {
			LuminousGame::FORSPOKEN => {
				let address: LuminousPointer<u8> = base.cast() + 0x77c5609;
				address.read(&mut reader.inner).unwrap_or_default() != 0
			}
			_ => false,
		};

		let dxvk_address: LuminousCString = LuminousCString::new(
			match reader.inner.game_type() {
				LuminousGame::FORSPOKEN => {
					let address: LuminousPointer<u64> = base.cast() + 0x9bef0f0;
					address.read(&mut reader.inner).unwrap_or_default()
				}
				_ => 0,
			}
			.into(),
		);

		if let Some(str) = dxvk_address.read(&mut reader)
			&& !str.is_empty()
		{
			self.dxvk_version = Some(str);
		}

		if self.enable_introspection {
			std::thread::spawn(|| {
				if let Some(objects) = &FUNCTIONS.get().unwrap().objects {
					loop {
						let packages = objects.get_packages(EBEX.get().unwrap());
						let mut old = PACKAGE.get().unwrap().lock().unwrap();
						match packages {
							Ok(packages) => {
								*old = Arc::new(packages);
							}
							Err(_) => {
								*old = Arc::default();
							}
						}

						std::thread::sleep(Duration::from_micros(333));
					}
				}
			});
		}
	}

	fn render(&mut self, ui: &mut Ui) {
		let mut opened = self.window_opened;

		if ui.is_key_pressed(Key::F8) {
			opened = !opened;
		}

		// todo: maybe hook this and only call when opened/closed?
		unsafe { ShowCursor(opened) };

		self.window_opened = opened;

		let function_bag = FUNCTIONS.get().unwrap();

		if opened && let Some(window) = ui.window("Scarlet").begin() {
			if self.is_wine {
				ui.text_colored([1.0, 0.0, 0.0, 1.0], "Detected as Wine!");
			}

			if self.is_deck {
				ui.text_colored([1.0, 0.0, 0.0, 1.0], "Detected as SteamDeck!");
			}

			if let Some(version) = &self.dxvk_version {
				ui.text_colored([1.0, 0.0, 0.0, 1.0], format!("Detected as DXVK version {}!", version));
			}

			if function_bag.set_world_time_impl.is_some() && ui.slider("World Time", 0f32, 1f32, &mut self.time) {
				function_bag.set_world_time(self.time);
			}

			if self.enable_introspection
				&& let Ok(packages) = PACKAGE.get().unwrap().lock()
			{
				self.render_packages(ui, packages.clone());
			}

			window.end();
		}
	}

	fn message_filter(&self, _io: &Io) -> MessageFilter {
		if self.window_opened { MessageFilter::InputAll } else { MessageFilter::empty() }
	}
}
