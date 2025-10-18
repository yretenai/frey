// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::ffi::c_void;

use hudhook::ImguiRenderLoop;
use hudhook::imgui::Ui;
use witch_common::engine::ebex::{ObjectInfo, ObjectInfoRegistry};
use witch_common::engine::game_module::GameModules;
use witch_common::engine::r#unsafe::ebex::{EbexObjectCallDynamic, ObjectType};
use witch_common::engine::{LuminousCString, LuminousPointer, LuminousString};
use witch_common::memory::windows_local_mem::Win32LocalMemoryReader;
use witch_common::memory::{MemoryCursor, MemoryReader};

use crate::interop::{call_ebex_func, find_ebex_function};

type EbexFunc<T> = Option<LuminousPointer<EbexObjectCallDynamic<T>>>;
type _EbexGetType = unsafe extern "system" fn(this: *const c_void) -> LuminousPointer<ObjectType>;
type _EbexGetName = unsafe extern "system" fn(this: *const c_void) -> LuminousCString;

#[derive(Default)]
pub struct ScarletFunctions {
	pub set_world_time_impl: EbexFunc<()>,

	pub _activate_gameobj_impl: EbexFunc<bool>,
	pub _deactivate_gameobj_impl: EbexFunc<bool>,
	pub _is_active_gameobj_impl: EbexFunc<bool>,

	// note this allocs a string, real string is returned via vtable + 0x48 (note everything has this function)
	pub _get_entity_name: EbexFunc<LuminousString>,
	pub _get_source_path: EbexFunc<LuminousCString>,

	// this calls the package system, EntityGroup has another array at 0x70
	// this array is at 0x148, which is horribly slow
	// there's also a name array at 0x160
	pub _get_loaded_entities_count: EbexFunc<u32>,
	pub _get_loaded_entity_by_index: EbexFunc<LuminousPointer<()>>,
}

impl ScarletFunctions {
	pub fn new(base: LuminousPointer<()>, ebex: &ObjectInfoRegistry) -> Self {
		ScarletFunctions {
			set_world_time_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.Debug.MapScreenshotUtility", "SetWorldTime"),
			_activate_gameobj_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.GameObject", "Activate"),
			_deactivate_gameobj_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.GameObject", "Inactivate"),
			_is_active_gameobj_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.GameObject", "IsActive"),
			_get_entity_name: find_ebex_function(base, ebex, "Luminous.EntitySystem.Entity", "GetNameCS"),
			_get_source_path: find_ebex_function(base, ebex, "Luminous.EntitySystem.EntityPackageReference", "GetSourcePath"),
			_get_loaded_entities_count: find_ebex_function(base, ebex, "Luminous.EntitySystem.EntityPackage", "GetLoadedEntitiesCount"),
			_get_loaded_entity_by_index: find_ebex_function(base, ebex, "Luminous.EntitySystem.EntityPackage", "GetLoadedEntityByIndex"),
		}
	}

	pub fn _get_object_type(entity: LuminousPointer<()>) -> anyhow::Result<ObjectInfo> {
		let mut reader = MemoryCursor::new(MemoryReader::Process(Win32LocalMemoryReader::new(true)));
		let vtable = entity.cast::<LuminousPointer<()>>().read(&mut reader.inner)?; // void* -> void** (ptr to vtable)
		let get_type_info: LuminousPointer<()> = vtable + 8; // vtable entry 2
		let call = unsafe { std::mem::transmute::<u64, _EbexGetType>(get_type_info.inner) };
		let type_info_ptr = unsafe { call(entity.cast().unsafe_ptr()) };
		let type_info = type_info_ptr.read(&mut reader.inner)?;
		ObjectInfo::new(&mut reader, type_info)
	}

	pub fn _get_object_name(entity: LuminousPointer<()>) -> anyhow::Result<String> {
		let mut reader = MemoryCursor::new(MemoryReader::Process(Win32LocalMemoryReader::new(true)));
		let vtable = entity.cast::<LuminousPointer<()>>().read(&mut reader.inner)?; // void* -> void** (ptr to vtable)
		let get_type_info: LuminousPointer<()> = vtable + 0x48; // vtable entry 9
		let call = unsafe { std::mem::transmute::<u64, _EbexGetName>(get_type_info.inner) };
		let name_ptr = unsafe { call(entity.cast().unsafe_ptr()) };
		Ok(name_ptr.read(&mut reader).unwrap_or("(no name)".to_string()))
	}

	pub fn set_world_time(&self, time: f32) {
		if let Some(func) = self.set_world_time_impl {
			let arg0: *const _ = &time;
			let args = [arg0 as *const c_void];
			call_ebex_func(func, None, Some(&args));
		}
	}

	pub fn _activate_gameobj(&self, gameobj: LuminousPointer<()>) -> bool {
		if let Some(func) = self._activate_gameobj_impl { call_ebex_func(func, Some(gameobj), None) } else { false }
	}

	pub fn _deactivate_gameobj(&self, gameobj: LuminousPointer<()>) -> bool {
		if let Some(func) = self._deactivate_gameobj_impl { call_ebex_func(func, Some(gameobj), None) } else { false }
	}

	pub fn _gameobject_is_active(&self, gameobj: LuminousPointer<()>) -> bool {
		if let Some(func) = self._is_active_gameobj_impl { call_ebex_func(func, Some(gameobj), None) } else { false }
	}
}

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
