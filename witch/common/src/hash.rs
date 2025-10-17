// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

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

pub const fn fnv1a32(bytes: &[u8]) -> u32 {
	let mut hash: u32 = 0x811c9dc5;
	let mut index = 0;

	while index < bytes.len() {
		hash ^= bytes[index] as u32;
		hash = hash.wrapping_mul(0x1000193);
		index += 1;
	}

	hash
}

pub const fn crc32(bytes: &[u8]) -> u32 {
	let mut hash = u32::MAX;
	let mut index = 0;

	while index < bytes.len() {
		hash ^= bytes[index] as u32;
		index += 1;

		let mut inner_index = 0;
		while inner_index < 8 {
			if (hash & 1) == 1 {
				hash = hash >> 1 ^ 0x04c11db7;
			} else {
				hash >>= 1;
			}

			inner_index += 1;
		}
	}

	!hash
}
