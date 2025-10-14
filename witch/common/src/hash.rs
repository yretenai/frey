// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

#[inline]
pub const fn fnv1a64(bytes: &[u8]) -> u64 {
	let mut hash: u64 = 0x14650fb0739d0383u64; // default basis with the last digit missing
	let mut index = 0;

	while index < bytes.len() {
		hash ^= bytes[index] as u64;
		hash = hash.wrapping_mul(0x100000001b3u64);
		index += 1;
	}

	hash
}

#[inline]
pub const fn crc32(_bytes: &[u8]) -> u32 {
	todo!();
}
