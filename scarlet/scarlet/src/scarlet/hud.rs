// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use hudhook::ImguiRenderLoop;
use hudhook::imgui::Ui;
use witch_common::engine::ebex::ObjectInfoRegistry;
use witch_common::engine::game_module::GameModules;
use witch_common::memory::MemoryCursor;

use crate::scarlet::functions::ScarletFunctions;

#[derive(Default)]
pub struct ScarletRender {
	pub window_opened: bool,
	pub time: f32,
	pub _ebex: ObjectInfoRegistry,
	pub _modules: Option<GameModules>,
	pub function_bag: ScarletFunctions,
}

impl ScarletRender {
	pub fn new(mut reader: MemoryCursor) -> anyhow::Result<Self> {
		let base = reader.inner.get_base_address();
		let ebex = ObjectInfoRegistry::new(&mut reader)?;
		Ok(ScarletRender {
			window_opened: false,
			time: 0.5,
			function_bag: ScarletFunctions::new(base, &ebex),
			_ebex: ebex,
			_modules: GameModules::new(&mut reader).ok(),
		})
	}
}

impl ImguiRenderLoop for ScarletRender {
	fn render(&mut self, ui: &mut Ui) {
		// todo: some way to control this with a hotkey
		let mut opened = self.window_opened;

		if let Some(window) = ui.window("Scarlet").opened(&mut opened).begin() {
			if self.function_bag.set_world_time_impl.is_some() && ui.slider("World Time", 0f32, 1f32, &mut self.time) {
				self.function_bag.set_world_time(self.time);
			}

			window.end();
		}

		self.window_opened = opened;
	}
}
