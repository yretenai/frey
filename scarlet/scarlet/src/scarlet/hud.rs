// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use hudhook::imgui::{Context, ImStr, ImString, Io, Key, Ui};
use hudhook::{ImguiRenderLoop, MessageFilter, RenderContext};
use log::error;
use windows::Win32::UI::WindowsAndMessaging::ShowCursor;
use witch_common::engine::ebex::ObjectInfoRegistry;
use witch_common::memory::windows_local_mem::Win32LocalMemoryReader;
use witch_common::memory::{MemoryCursor, MemoryReader};

use crate::scarlet::functions::ScarletFunctions;
use crate::scarlet::functions::objects::ScarletObject;

#[derive(Default)]
pub struct ScarletRender {
	pub window_opened: bool,
	pub time: f32,
	pub selected_package: i32,
}

// todo: can i omit globals lol
static EBEX: OnceLock<ObjectInfoRegistry> = OnceLock::new();
static PACKAGE: OnceLock<Mutex<Arc<Vec<ScarletObject>>>> = OnceLock::new();
static FUNCTIONS: OnceLock<ScarletFunctions> = OnceLock::new();

impl ScarletRender {
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

		// _modules = GameModules::new(&mut reader).ok();
		std::thread::spawn(|| {
			if let Some(objects) = &FUNCTIONS.get().unwrap().objects {
				loop {
					let packages = objects.get_packages(EBEX.get().unwrap());
					let mut old = PACKAGE.get().unwrap().lock().unwrap();
					match packages {
						Ok(packages) => {
							*old = Arc::new(packages);
						}
						Err(err) => {
							*old = Arc::default();
							error!("failed reading packages: {:?}", err);
						}
					}

					std::thread::sleep(Duration::from_micros(10));
				}
			}
		});
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
			if function_bag.set_world_time_impl.is_some() && ui.slider("World Time", 0f32, 1f32, &mut self.time) {
				function_bag.set_world_time(self.time);
			}

			if let Ok(packages) = PACKAGE.get().unwrap().lock() {
				self.render_packages(ui, packages.clone());
			} else {
				error!("packages not set?");
			}

			window.end();
		}
	}

	fn message_filter(&self, _io: &Io) -> MessageFilter {
		if self.window_opened { MessageFilter::InputAll } else { MessageFilter::empty() }
	}
}
