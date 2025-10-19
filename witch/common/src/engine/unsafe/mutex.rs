// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use bytemuck::{Pod, Zeroable};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{CRITICAL_SECTION, EnterCriticalSection, LeaveCriticalSection, TryEnterCriticalSection};
#[cfg(target_os = "windows")]
use windows::core::BOOL;

use crate::engine::LuminousPointer;

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
pub struct LuminousMutexCriticalSection {
	debug_info: LuminousPointer<()>,
	lock_count: u32,
	recursion_count: u32,
	tid: u64,
	semaphore: u64,
	spin_cont: u64,
}

#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct LuminousMutex {
	reserved: u64,
	mutex: LuminousMutexCriticalSection,
}

#[derive(Debug, Default, Copy, Clone)]
#[cfg(target_os = "windows")]
pub struct LuminousGameMutex {
	pub mutex: LuminousPointer<CRITICAL_SECTION>,

	marker: std::marker::PhantomData<*mut CRITICAL_SECTION>,
}

#[cfg(target_os = "windows")]
impl LuminousGameMutex {
	pub fn new(address: LuminousPointer<CRITICAL_SECTION>) -> Self {
		Self {
			mutex: address,
			marker: std::marker::PhantomData,
		}
	}

	/// locks the mutex with [`EnterCriticalSection`]
	///
	/// # Safety
	///
	/// calls native win32 APIs,
	/// may crash if uninitialized,
	/// will crash if it fails, prefer [`try_lock`]
	pub fn lock(&mut self) {
		unsafe { EnterCriticalSection(self.mutex.unsafe_mut_ptr()) }
	}

	/// tries to lock the mutex with [`TryEnterCriticalSection`]
	///
	/// # Safety
	///
	/// calls native win32 APIs,
	/// may crash if uninitialized
	pub fn try_lock(&mut self) -> anyhow::Result<()> {
		match unsafe { TryEnterCriticalSection(self.mutex.unsafe_mut_ptr()) } {
			BOOL(0) => anyhow::bail!("already locked"),
			_ => Ok(()),
		}
	}

	/// tries to unlock the mutex with [`LeaveCriticalSection`]
	///
	/// # Safety
	///
	/// calls native win32 APIs,
	/// may crash if uninitialized,
	/// will crash if the mutex is not owned by this thread
	pub fn unlock(&mut self) {
		unsafe { LeaveCriticalSection(self.mutex.unsafe_mut_ptr()) }
	}
}
