// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::ffi::c_void;

use bytemuck::Pod;
use witch_common::engine::LuminousPointer;
use witch_common::engine::ebex::ObjectInfoRegistry;
use witch_common::engine::r#unsafe::ebex::EbexObjectCallDynamic;
use witch_common::engine::r#unsafe::pointer::EbexFunc;

use crate::scarlet::functions::objects::ScarletObjects;

pub(crate) mod objects;

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

pub unsafe fn call_vtable_0<T: Pod>(this: LuminousPointer<()>, index: usize) -> LuminousPointer<T> {
	unsafe {
		let vtable = *(this.unsafe_mut_ptr() as *mut *const *const c_void);
		let func: unsafe extern "C" fn(*mut c_void) -> *const T = std::mem::transmute(*vtable.add(index));
		LuminousPointer::new(func(this.unsafe_mut_ptr() as *mut _) as u64)
	}
}

#[derive(Default)]
pub struct ScarletFunctions {
	pub set_world_time_impl: Option<EbexFunc<()>>,

	pub objects: Option<ScarletObjects>,
}

impl ScarletFunctions {
	pub fn new(base: LuminousPointer<()>, ebex: &ObjectInfoRegistry) -> Self {
		ScarletFunctions {
			set_world_time_impl: find_ebex_function(base, ebex, "Luminous.GameFramework.Debug.MapScreenshotUtility", "SetWorldTime"),
			objects: ScarletObjects::new(base, ebex),
		}
	}

	pub fn set_world_time(&self, time: f32) {
		if let Some(func) = self.set_world_time_impl {
			let arg0: *const _ = &time;
			let args = [arg0 as *const c_void];
			func.call(None, Some(&args));
		}
	}
}
