// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr::{null, null_mut};

use witch_common::engine::LuminousPointer;
use witch_common::engine::ebex::ObjectInfoRegistry;
use witch_common::engine::r#unsafe::ebex::EbexObjectCallDynamic;

pub fn call_ebex_func<T>(
	call: LuminousPointer<EbexObjectCallDynamic<T>>,
	this: Option<LuminousPointer<()>>,
	args: Option<&[*const c_void]>,
) -> T {
	let this_ptr: *mut c_void = this.map_or(null_mut(), |t| unsafe { t.unsafe_mut_ptr() as *mut c_void });

	let mut result = MaybeUninit::<T>::uninit();
	let res_ptr = if size_of::<T>() == 0 { null_mut() } else { result.as_mut_ptr() };

	let arg_ptr: *const *const c_void = args.map_or(null(), |a| a.as_ptr());

	unsafe {
		let call = std::mem::transmute::<u64, EbexObjectCallDynamic<T>>(call.inner);
		call(res_ptr, this_ptr, arg_ptr);
	}

	unsafe { result.assume_init() }
}

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
