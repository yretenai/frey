// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use bytemuck::{Pod, Zeroable};

use crate::engine::LuminousPointer;

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct LuminousMutexCriticalSection {
	debug_info: LuminousPointer<()>,
	lock_count: u32,
	recursion_count: u32,
	tid: u64,
	semaphore: u64,
	spin_cont: u64,
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed(8))]
pub struct LuminousMutex {
	reserved: u64,
	mutex: LuminousMutexCriticalSection,
}
