// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::any::Any;
use std::ffi::c_void;
use std::ptr::null_mut;

use witch_common::engine::LuminousPointer;

pub type _EbexObjectCall = unsafe extern "system" fn(this: *mut c_void, result: *mut c_void, args: *mut c_void);

pub fn _call_ebex_func<T>(call: LuminousPointer<_EbexObjectCall>, this: Option<LuminousPointer<()>>, args: &mut [&mut dyn Any]) -> T {
	let this_ptr: *mut c_void = this.map_or(null_mut(), |t| unsafe { t.unsafe_mut_ptr() as *mut c_void });

	let mut result = unsafe { std::mem::zeroed::<T>() };
	let res_ptr = if size_of::<T>() == 0 { null_mut() } else { &mut result as *mut T as *mut c_void };

	let mut args_ptrs: Vec<*mut c_void> = args.iter_mut().map(|arg| (*arg) as *mut _ as *mut c_void).collect();

	_call_ebex_func_inner(call, this_ptr, res_ptr, args_ptrs.as_mut_ptr() as *mut c_void);

	result
}

pub fn _call_ebex_func_inner(call: LuminousPointer<_EbexObjectCall>, this: *mut c_void, res: *mut c_void, args: *mut c_void) {
	let call = unsafe { *call.unsafe_ptr() };
	unsafe {
		call(this, res, args);
	}
}
