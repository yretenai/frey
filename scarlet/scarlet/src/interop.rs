// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr::null_mut;

use witch_common::engine::LuminousPointer;
use witch_common::engine::ebex::ObjectInfoRegistry;
use witch_common::engine::r#unsafe::ebex::EbexObjectCallDynamic;

pub fn call_ebex_func<T>(call: LuminousPointer<EbexObjectCallDynamic>, this: Option<LuminousPointer<()>>, args: &[*const c_void]) -> T {
	let this_ptr: *mut c_void = this.map_or(null_mut(), |t| unsafe { t.unsafe_mut_ptr() as *mut c_void });

	let mut result = MaybeUninit::<T>::uninit();
	let res_ptr = if size_of::<T>() == 0 { null_mut() } else { result.as_mut_ptr() as *mut c_void };

	unsafe {
		let call = std::mem::transmute::<u64, EbexObjectCallDynamic>(call.inner);
		call(this_ptr, res_ptr, args.as_ptr());
	}

	unsafe { result.assume_init() }
}

pub fn find_ebex_function(
	base: LuminousPointer<()>,
	ebex: &ObjectInfoRegistry,
	object_name: &str,
	function_name: &str,
) -> Option<LuminousPointer<EbexObjectCallDynamic>> {
	let obj = ebex.elements.get(object_name)?;
	let func = obj.functions.get(function_name)?;
	Some(base.cast() + func.function_dynamic)
}
