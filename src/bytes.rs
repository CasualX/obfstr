/*!
Byte string obfuscation
=======================
*/

use core::ptr::{read_volatile, write};

// Simple XorShift to generate the key stream.
// Security doesn't matter, we just want a number of random-looking bytes.
#[inline(always)]
const fn next_round(mut x: u32) -> u32 {
	x ^= x << 13;
	x ^= x >> 17;
	x ^= x << 5;
	x
}

/// Generate the key stream for array of given length.
#[inline(always)]
pub const fn keystream<const LEN: usize>(key: u32) -> [u8; LEN] {
	let mut keys = [0u8; LEN];
	let mut round_key = key;
	let mut i = 0;
	// Calculate the key stream in chunks of 4 bytes
	while i < LEN & !3 {
		round_key = next_round(round_key);
		let kb = round_key.to_ne_bytes();
		keys[i + 0] = kb[0];
		keys[i + 1] = kb[1];
		keys[i + 2] = kb[2];
		keys[i + 3] = kb[3];
		i += 4;
	}
	// Calculate the remaining bytes of the key stream
	round_key = next_round(round_key);
	let kb = round_key.to_ne_bytes();
	match LEN % 4 {
		1 => {
			keys[i + 0] = kb[0];
		},
		2 => {
			keys[i + 0] = kb[0];
			keys[i + 1] = kb[1];
		},
		3 => {
			keys[i + 0] = kb[0];
			keys[i + 1] = kb[1];
			keys[i + 2] = kb[2];
		},
		_ => (),
	}
	keys
}

/// Obfuscates the input string and given key stream.
#[inline(always)]
pub const fn obfuscate<const LEN: usize>(s: &[u8], k: &[u8; LEN]) -> [u8; LEN] {
	if s.len() != LEN {
		panic!("input string len not equal to key stream len");
	}
	let mut data = [0u8; LEN];
	let mut i = 0usize;
	while i < LEN {
		data[i] = s[i] ^ k[i];
		i += 1;
	}
	data
}

/// Deobfuscates the obfuscated input string and given key stream.
#[inline(always)]
pub fn deobfuscate<const LEN: usize>(s: &[u8; LEN], k: &[u8; LEN]) -> [u8; LEN] {
	let mut buffer = [0u8; LEN];
	let mut i = 0;
	// Try to tickle the LLVM optimizer in _just_ the right way
	// Use `read_volatile` to avoid constant folding a specific read and optimize the rest
	// Volatile reads of any size larger than 8 bytes appears to cause a bunch of one byte reads
	// Hand optimize in chunks of 8 and 4 bytes to avoid this
	unsafe {
		let src = s.as_ptr();
		let dest = buffer.as_mut_ptr();
		// Process in chunks of 8 bytes on 64-bit targets
		#[cfg(target_pointer_width = "64")]
		while i < LEN & !7 {
			let ct = read_volatile(src.offset(i as isize) as *const [u8; 8]);
			let tmp = u64::from_ne_bytes([ct[0], ct[1], ct[2], ct[3], ct[4], ct[5], ct[6], ct[7]]) ^
				u64::from_ne_bytes([k[i + 0], k[i + 1], k[i + 2], k[i + 3], k[i + 4], k[i + 5], k[i + 6], k[i + 7]]);
			write(dest.offset(i as isize) as *mut [u8; 8], tmp.to_ne_bytes());
			i += 8;
		}
		// Process in chunks of 4 bytes
		while i < LEN & !3 {
			let ct = read_volatile(src.offset(i as isize) as *const [u8; 4]);
			let tmp = u32::from_ne_bytes([ct[0], ct[1], ct[2], ct[3]]) ^
				u32::from_ne_bytes([k[i + 0], k[i + 1], k[i + 2], k[i + 3]]);
			write(dest.offset(i as isize) as *mut [u8; 4], tmp.to_ne_bytes());
			i += 4;
		}
		// Process the remaining bytes
		match LEN % 4 {
			1 => {
				let ct = read_volatile(src.offset(i as isize));
				write(dest.offset(i as isize), ct ^ k[i]);
			},
			2 => {
				let ct = read_volatile(src.offset(i as isize) as *const [u8; 2]);
				write(dest.offset(i as isize) as *mut [u8; 2], [
					ct[0] ^ k[i + 0],
					ct[1] ^ k[i + 1],
				]);
			},
			3 => {
				let ct = read_volatile(src.offset(i as isize) as *const [u8; 3]);
				write(dest.offset(i as isize) as *mut [u8; 2], [
					ct[0] ^ k[i + 0],
					ct[1] ^ k[i + 1],
				]);
				write(dest.offset(i as isize + 2), ct[2] ^ k[i + 2]);
			},
			_ => (),
		}
	}
	buffer
}

#[inline(always)]
pub fn equals<const LEN: usize>(s: &[u8; LEN], k: &[u8; LEN], other: &[u8]) -> bool {
	if other.len() != LEN {
		return false;
	}
	let mut i = 0;
	// Try to tickle the LLVM optimizer in _just_ the right way
	// Use `read_volatile` to avoid constant folding a specific read and optimize the rest
	// Volatile reads of any size larger than 8 bytes appears to cause a bunch of one byte reads
	// Hand optimize in chunks of 8 and 4 bytes to avoid this
	unsafe {
		let src = s.as_ptr();
		// Process in chunks of 8 bytes on 64-bit targets
		#[cfg(target_pointer_width = "64")]
		while i < LEN & !7 {
			let ct = read_volatile(src.offset(i as isize) as *const [u8; 8]);
			let tmp = u64::from_ne_bytes([ct[0], ct[1], ct[2], ct[3], ct[4], ct[5], ct[6], ct[7]]) ^
				u64::from_ne_bytes([k[i + 0], k[i + 1], k[i + 2], k[i + 3], k[i + 4], k[i + 5], k[i + 6], k[i + 7]]);
			let other = u64::from_ne_bytes([other[i + 0], other[i + 1], other[i + 2], other[i + 3], other[i + 4], other[i + 5], other[i + 6], other[i + 7]]);
			if tmp != other {
				return false;
			}
			i += 8;
		}
		// Process in chunks of 4 bytes
		while i < LEN & !3 {
			let ct = read_volatile(src.offset(i as isize) as *const [u8; 4]);
			let tmp = u32::from_ne_bytes([ct[0], ct[1], ct[2], ct[3]]) ^
				u32::from_ne_bytes([k[i + 0], k[i + 1], k[i + 2], k[i + 3]]);
			let other = u32::from_ne_bytes([other[i + 0], other[i + 1], other[i + 2], other[i + 3]]);
			if tmp != other {
				return false;
			}
			i += 4;
		}
		// Process the remaining bytes
		match LEN % 4 {
			1 => {
				let ct = read_volatile(src.offset(i as isize));
				ct ^ k[i] == other[i]
			},
			2 => {
				let ct = read_volatile(src.offset(i as isize) as *const [u8; 2]);
				u16::from_ne_bytes([ct[0], ct[1]]) ^ u16::from_ne_bytes([k[i + 0], k[i + 1]]) == u16::from_ne_bytes([other[i + 0], other[i + 1]])
			},
			3 => {
				let ct = read_volatile(src.offset(i as isize) as *const [u8; 3]);
				u32::from_ne_bytes([ct[0], ct[1], ct[2], 0]) ^ u32::from_ne_bytes([k[i + 0], k[i + 1], k[i + 2], 0]) == u32::from_ne_bytes([other[i + 0], other[i + 1], other[i + 2], 0])
			},
			_ => true,
		}
	}
}

// Test correct processing of less than multiple of 8 lengths
#[test]
fn test_remaining_bytes() {
	const STRING: &[u8] = b"01234567ABCDEFGHI";
	fn test<const LEN: usize>(key: u32) {
		let keys = keystream::<LEN>(key);
		let data = obfuscate::<LEN>(&STRING[..LEN], &keys);
		let buffer = deobfuscate::<LEN>(&data, &keys);
		// Ciphertext should not equal input string
		assert_ne!(&data[..], &STRING[..LEN]);
		// Deobfuscated result should equal input string
		assert_eq!(&buffer[..], &STRING[..LEN]);
		// Specialized equals check should succeed
		assert!(equals::<LEN>(&data, &keys, &STRING[..LEN]));
	}
	test::<8>(0x1111);
	test::<9>(0x2222);
	test::<10>(0x3333);
	test::<11>(0x4444);
	test::<12>(0x5555);
	test::<13>(0x6666);
	test::<14>(0x7777);
	test::<15>(0x8888);
	test::<16>(0x9999);
}

#[test]
fn test_equals() {
	const STRING: &str = "Hello ðŸŒ";
	const LEN: usize = STRING.len();
	const KEYSTREAM: [u8; LEN] = keystream::<LEN>(0x10203040);
	const OBFSTRING: [u8; LEN] = obfuscate::<LEN>(STRING.as_bytes(), &KEYSTREAM);
	assert!(equals::<LEN>(&OBFSTRING, &KEYSTREAM, STRING.as_bytes()));
}
