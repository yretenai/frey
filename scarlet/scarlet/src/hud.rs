// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::ffi::c_void;

use hudhook::ImguiRenderLoop;
use hudhook::imgui::Ui;
use log::error;
use witch_common::engine::LuminousPointer;
use witch_common::engine::ebex::ObjectInfoRegistry;
use witch_common::engine::game_module::GameModules;
use witch_common::memory::MemoryCursor;

use crate::interop::{call_ebex_func, find_ebex_static};

#[derive(Default)]
pub struct ScarletRender {
	pub window_opened: bool,
	pub time: f32,
	pub base: LuminousPointer<()>,
	pub ebex: ObjectInfoRegistry,
	pub _modules: Option<GameModules>,
	pub _objects: Vec<LuminousPointer<()>>,
}

impl ScarletRender {
	pub fn new(mut reader: MemoryCursor) -> anyhow::Result<Self> {
		Ok(ScarletRender {
			window_opened: true,
			time: 0.5,
			base: reader.inner.get_base_address(),
			ebex: ObjectInfoRegistry::new(&mut reader)?,
			_modules: GameModules::new(&mut reader).ok(),
			..Default::default()
		})
	}
}

impl ImguiRenderLoop for ScarletRender {
	fn render(&mut self, ui: &mut Ui) {
		if let Some(window) = ui.window("Scarlet").opened(&mut self.window_opened).begin() {
			if ui.slider("World Time", 0f32, 1f32, &mut self.time) {
				// todo: cache me
				let func = find_ebex_static(&self.ebex, "Luminous.GameFramework.Debug.MapScreenshotUtility", "SetWorldTime");
				if let Some(func) = func {
					let arg0: *const _ = &self.time;
					let args = [arg0 as *const c_void];
					call_ebex_func::<()>(self.base.cast() + func, None, &args);
				} else {
					error!("could not find Luminous::GameFramework::Debug::MapScreenshotUtility::SetWorldTime");
				}
			}

			window.end();
		} else {
			self.window_opened = false;
		}
	}
}
