// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::ffi::c_void;

use hudhook::ImguiRenderLoop;
use hudhook::imgui::{Ui, sys};
use log::error;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F8};
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
			base: reader.inner.get_base_address(),
			ebex: ObjectInfoRegistry::new(&mut reader)?,
			_modules: GameModules::new(&mut reader).ok(),
			..Default::default()
		})
	}
}

impl ImguiRenderLoop for ScarletRender {
	fn render(&mut self, ui: &mut Ui) {
		if unsafe { GetAsyncKeyState(VK_F8.0 as i32) as u16 } & 0x8000 != 0 {
			self.window_opened = !self.window_opened;
		}

		if let Some(window) = ui.window("Scarlet").opened(&mut self.window_opened).begin() {
			unsafe {
				(&mut *sys::igGetIO()).ConfigFlags = (*sys::igGetIO()).ConfigFlags & !32;
			}

			if ui.slider("World Time", 0f32, 1f32, &mut self.time) {
				// todo: cache me
				let func = find_ebex_static(&self.ebex, "Luminous.GameFramework.Automation.AutomationUtility", "SetWorldTime");
				if let Some(func) = func {
					call_ebex_func::<()>(self.base.cast() + func, None, &mut [&mut self.time as *mut _ as *mut c_void]);
				} else {
					error!("could not find Luminous::GameFramework::Automation::AutomationUtility::SetWorldTime");
				}
			}

			window.end();
		} else {
			unsafe {
				(&mut *sys::igGetIO()).ConfigFlags = (*sys::igGetIO()).ConfigFlags & 32;
			}
		}
	}
}
