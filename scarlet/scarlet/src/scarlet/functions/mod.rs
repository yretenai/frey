// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::ffi::c_void;

use witch_common::engine::ebex::ObjectInfoRegistry;
use witch_common::engine::r#unsafe::ebex::{EbexObjectCallDynamic, ObjectType};
use witch_common::engine::r#unsafe::pointer::EbexFunc;
use witch_common::engine::{LuminousCString, LuminousPointer};

use crate::scarlet::functions::objects::ScarletObjects;

pub(crate) mod objects;

type EbexGetType = unsafe extern "system" fn(this: *const c_void) -> LuminousPointer<ObjectType>;
type EbexGetName = unsafe extern "system" fn(this: *const c_void) -> LuminousCString;

pub fn find_ebex_function<T>(
	base: LuminousPointer<()>,
	ebex: &ObjectInfoRegistry,
	object_name: &str,
	function_name: &str,
) -> Option<LuminousPointer<EbexObjectCallDynamic<T>>> {
	let obj = ebex.elements.get(object_name)?;
	let func = obj.functions.get(function_name)?;
	Some(base.cast() + func.function_dynamic)
}

#[derive(Default)]
pub struct ScarletFunctions {
	pub set_world_time_impl: Option<EbexFunc<()>>,

	pub _activate_gameobj_impl: Option<EbexFunc<bool>>,
	pub _deactivate_gameobj_impl: Option<EbexFunc<bool>>,
	pub _is_active_gameobj_impl: Option<EbexFunc<bool>>,

	pub _objects: Option<ScarletObjects>,
}

impl ScarletFunctions {
	pub fn new(base: LuminousPointer<()>, ebex: &ObjectInfoRegistry) -> Self {
		ScarletFunctions {
			set_world_time_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.Debug.MapScreenshotUtility", "SetWorldTime"),
			_activate_gameobj_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.GameObject", "Activate"),
			_deactivate_gameobj_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.GameObject", "Inactivate"),
			_is_active_gameobj_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.GameObject", "IsActive"),
			_objects: ScarletObjects::new(base, ebex),
		}
	}

	pub fn set_world_time(&self, time: f32) {
		if let Some(func) = self.set_world_time_impl {
			let arg0: *const _ = &time;
			let args = [arg0 as *const c_void];
			func.call(None, Some(&args));
		}
	}

	pub fn _activate_gameobj(&self, gameobj: LuminousPointer<()>) -> bool {
		if let Some(func) = self._activate_gameobj_impl { func.call(Some(gameobj), None) } else { false }
	}

	pub fn _deactivate_gameobj(&self, gameobj: LuminousPointer<()>) -> bool {
		if let Some(func) = self._deactivate_gameobj_impl { func.call(Some(gameobj), None) } else { false }
	}

	pub fn _gameobject_is_active(&self, gameobj: LuminousPointer<()>) -> bool {
		if let Some(func) = self._is_active_gameobj_impl { func.call(Some(gameobj), None) } else { false }
	}
}
