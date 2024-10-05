/*!
Expose MurmurHash3, a keyed hash function. Not ready for public API.
*/

use core::slice;

/// MurmurHash3 (32-bit variant) keyed hash function.
#[doc(hidden)]
#[macro_export]
macro_rules! murmur3 {
	($s:expr, $seed:expr) => { const { $crate::murmur3($s, $seed) } };
	($s:expr) => { const { $crate::murmur3($s, 0) } };
}

/// MurmurHash3 (32-bit variant) keyed hash function.
#[doc(hidden)]
pub const fn murmur3(string: &[u8], seed: u32) -> u32 {
	let mut h = seed;
	const C1: u32 = 0xcc9e2d51;
	const C2: u32 = 0x1b873593;

	let mut s = string;
	while s.len() >= 4 {
		let mut k = u32::from_le_bytes([s[0], s[1], s[2], s[3]]);
		k = k.wrapping_mul(C1);
		k = k.rotate_left(15);
		k = k.wrapping_mul(C2);

		h ^= k;
		h = h.rotate_left(13);
		h = h.wrapping_mul(5).wrapping_add(0xe6546b64);

		// The slicing operator isn't stable in const fn
		// s = &s[4..];
		s = unsafe { slice::from_raw_parts(s.as_ptr().add(4), s.len() - 4)};
	}

	if s.len() > 0 {
		let k = match s.len() {
			3 => u32::from_le_bytes([s[0], s[1], s[2], 0]),
			2 => u32::from_le_bytes([s[0], s[1], 0, 0]),
			1 => u32::from_le_bytes([s[0], 0, 0, 0]),
			_ => 0/*unreachable!()*/,
		};
		h ^= k.wrapping_mul(C1).rotate_left(15).wrapping_mul(C2);
	}

	fmix32(h ^ string.len() as u32)
}

#[inline(always)]
const fn fmix32(mut h: u32) -> u32 {
	h ^= h >> 16;
	h = h.wrapping_mul(0x85ebca6b);
	h ^= h >> 13;
	h = h.wrapping_mul(0xc2b2ae35);
	h ^= h >> 16;
	return h;
}

#[test]
fn test_vectors() {
	static TEST_VECTORS: [(u32, u32, &[u8]); 13] = [
		(0,          0,          b""), // with zero data and zero seed, everything becomes zero
		(0x514E28B7, 1,          b""), // ignores nearly all the math
		(0x81F16F39, 0xffffffff, b""), // make sure your seed uses unsigned 32-bit math
		(0x76293B50, 0,          &[0xff, 0xff, 0xff, 0xff]), // make sure 4-byte chunks use unsigned math
		(0xF55B516B, 0,          &[0x21, 0x43, 0x65, 0x87]), // Endian order. UInt32 should end up as 0x87654321
		(0x2362F9DE, 0x5082EDEE, &[0x21, 0x43, 0x65, 0x87]), // Special seed value eliminates initial key with xor
		(0x7E4A8634, 0,          &[0x21, 0x43, 0x65]), // Only three bytes. Should end up as 0x654321
		(0xA0F7B07A, 0,          &[0x21, 0x43]), // Only two bytes. Should end up as 0x4321
		(0x72661CF4, 0,          &[0x21]), // Only one byte. Should end up as 0x21
		(0x2362F9DE, 0,          &[0, 0, 0, 0]),
		(0x85F0B427, 0,          &[0, 0, 0]),
		(0x30F4C306, 0,          &[0, 0]),
		(0x514E28B7, 0,          &[0]),
	];

	for &(expected, seed, input) in TEST_VECTORS.iter() {
		assert_eq!(expected, murmur3(input, seed));
	}
}
