// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::any::Any;

use hudhook::ImguiRenderLoop;
use hudhook::imgui::Ui;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F8};
use witch_common::engine::LuminousPointer;
use witch_common::engine::ebex::ObjectInfoRegistry;
use witch_common::engine::game_module::GameModules;
use witch_common::memory::MemoryCursor;

use crate::interop::{EbexObjectCall, call_ebex_func};

#[derive(Default)]
pub struct ScarletRender {
	pub window_opened: bool,
	pub time: i32,
	pub base: LuminousPointer<()>,
	pub _ebex: ObjectInfoRegistry,
	pub _modules: Option<GameModules>,
	pub _objects: Vec<LuminousPointer<()>>,
}

impl ScarletRender {
	pub fn new(mut reader: MemoryCursor) -> anyhow::Result<Self> {
		Ok(ScarletRender {
			window_opened: true,
			base: reader.inner.get_base_address(),
			_ebex: ObjectInfoRegistry::new(&mut reader)?,
			_modules: GameModules::new(&mut reader).ok(),
			..Default::default()
		})
	}
}

impl ImguiRenderLoop for ScarletRender {
	fn render(&mut self, ui: &mut Ui) {
		if unsafe { GetAsyncKeyState(VK_F8.0 as i32) as u16 } & 0x8000 != 0 {
			self.window_opened = true;
		}

		let call = LuminousPointer::<EbexObjectCall>::new(self.base.inner + 0x05ca1b0);
		if let Some(window) = ui.window("Scarlet").opened(&mut self.window_opened).begin() {
			if ui.slider("World Time", 0, 1440, &mut self.time) {
				let mut args: Vec<&mut dyn Any> = Vec::new();
				args.push(&mut self.time);
				call_ebex_func::<()>(call, None, &mut args);
			}

			window.end();
		}
	}
}
