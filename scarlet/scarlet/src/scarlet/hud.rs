// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use hudhook::imgui::{Context, Io, Key, Ui};
use hudhook::{ImguiRenderLoop, MessageFilter, RenderContext};
use windows::Win32::UI::WindowsAndMessaging::ShowCursor;
use witch_common::engine::ebex::ObjectInfoRegistry;
use witch_common::engine::game_module::GameModules;
use witch_common::memory::MemoryCursor;

use crate::scarlet::functions::ScarletFunctions;
use crate::scarlet::functions::objects::ScarletObject;

#[derive(Default)]
pub struct ScarletRender {
	pub window_opened: bool,
	pub time: f32,
	pub ebex: ObjectInfoRegistry,
	pub _modules: Option<GameModules>,
	pub function_bag: ScarletFunctions,
}

impl ScarletRender {
	pub fn new(mut reader: MemoryCursor) -> anyhow::Result<Self> {
		let base = reader.inner.get_base_address();
		let ebex = ObjectInfoRegistry::new(&mut reader)?;
		Ok(ScarletRender {
			function_bag: ScarletFunctions::new(base, &ebex),
			ebex,
			_modules: GameModules::new(&mut reader).ok(),
			..Default::default()
		})
	}

	fn render_packages(&mut self, _ui: &Ui, _packages: &Vec<ScarletObject>) {
		// todo
	}
}

impl ImguiRenderLoop for ScarletRender {
	fn initialize<'a>(&'a mut self, _: &mut Context, _: &'a mut dyn RenderContext) {
		self.window_opened = true;
		self.time = 0.5;
	}

	fn render(&mut self, ui: &mut Ui) {
		let mut opened = self.window_opened;
		if ui.is_key_pressed(Key::F8) {
			opened = true;
		}

		// todo: maybe hook this and only call when opened/closed?
		unsafe { ShowCursor(opened) };

		if let Some(window) = ui.window("Scarlet").opened(&mut opened).begin() {
			if self.function_bag.set_world_time_impl.is_some() && ui.slider("World Time", 0f32, 1f32, &mut self.time) {
				self.function_bag.set_world_time(self.time);
			}

			if let Some(objects) = &mut self.function_bag.objects {
				match objects.get_packages(&self.ebex) {
					Ok(packages) => {
						self.render_packages(ui, &packages);
					}
					Err(err) => {
						ui.text(format!("error reading packages: {:?}", err));
						ui.text(format!("backtrace:\n{:?}", err.backtrace()));
					}
				}
			}

			window.end();
		} else {
			opened = false;
		}

		self.window_opened = opened;
	}

	fn message_filter(&self, _io: &Io) -> MessageFilter {
		if self.window_opened { MessageFilter::InputAll } else { MessageFilter::empty() }
	}
}
