// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use hudhook::ImguiRenderLoop;
use hudhook::imgui::Ui;
use log::error;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F8};
use witch_common::engine::LuminousPointer;
use witch_common::engine::ebex::ObjectInfoRegistry;
use witch_common::engine::game_module::GameModules;
use witch_common::memory::windows_local_mem::Win32LocalMemoryReader;
use witch_common::memory::{MemoryCursor, MemoryReader};

#[derive(Default)]
pub struct ScarletRender {
	pub window_opened: bool,
	pub time: u64,
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

		let proc = Win32LocalMemoryReader {
			query_if_safe: true,
		};
		let mut reader = MemoryReader::Process(proc);
		let world_time_ptr: LuminousPointer<LuminousPointer<()>> = self.base.cast() + 0x79c5670;
		if let Some(window) = ui.window("Scarlet").opened(&mut self.window_opened).begin() {
			if ui.slider("World Time", 0, 0x608f3d000, &mut self.time) && world_time_ptr.is_valid() {
				if let Ok(ptr) = world_time_ptr.read(&mut reader)
					&& ptr.is_valid()
				{
					let bytes = self.time.to_le_bytes();
					if let Err(err) = proc.write(ptr.cast(), &bytes) {
						error!("failed to write time: {:?}", err);
					}
				} else {
					error!("tried to update time but can't read time pointer");
				}
			}

			window.end();
		}
	}
}
