// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use hudhook::ImguiRenderLoop;
use hudhook::imgui::Ui;
use witch_common::engine::LuminousPointer;
use witch_common::engine::ebex::ObjectInfoRegistry;
use witch_common::engine::game_module::GameModules;
use witch_common::memory::windows_local_mem::Win32LocalMemoryReader;
use witch_common::memory::{MemoryCursor, MemoryReader};

#[derive(Default)]
pub struct ScarletRender {
	demo_opened: bool,
	pub _ebex: ObjectInfoRegistry,
	pub _modules: Option<GameModules>,
	pub _objects: Vec<LuminousPointer<()>>,
}

impl ScarletRender {
	pub fn new() -> anyhow::Result<Self> {
		let mut reader = MemoryCursor::new(MemoryReader::Process(Win32LocalMemoryReader::new(false)));
		Ok(ScarletRender {
			demo_opened: true,
			_ebex: ObjectInfoRegistry::new(&mut reader)?,
			_modules: GameModules::new(&mut reader).ok(),
			..Default::default()
		})
	}
}

impl ImguiRenderLoop for ScarletRender {
	fn render(&mut self, ui: &mut Ui) {
		ui.show_demo_window(&mut self.demo_opened);
	}
}
