/*!
Wide string obfuscation
=======================
*/

use core::ptr::{read_volatile, write};

/// Compiletime wide string constant obfuscation.
#[macro_export]
macro_rules! obfwide {
	($(let $name:ident = $s:expr;)*) => {
		$(let ref $name = $crate::__obfwide!($s);)*
	};
	($name:ident = $s:expr) => {{
		$name = $crate::__obfwide!($s);
		&$name
	}};
	($buf:ident <- $s:expr) => {{
		let buf = &mut $buf[..$s.len()];
		buf.copy_from_slice(&$crate::__obfwide!($s));
		buf
	}};
	($s:expr) => {
		&$crate::__obfwide!($s)
	};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __obfwide {
	($s:expr) => {{
		const _OBFWIDE_STRING: &[u16] = $crate::wide!($s);
		const _OBFWIDE_LEN: usize = _OBFWIDE_STRING.len();
		const _OBFWIDE_KEYSTREAM: [u16; _OBFWIDE_LEN] = $crate::words::keystream::<_OBFWIDE_LEN>($crate::random!(u32, "key", stringify!($s)));
		static _OBFWIDE_SDATA: [u16; _OBFWIDE_LEN] = $crate::words::obfuscate::<_OBFWIDE_LEN>(_OBFWIDE_STRING, &_OBFWIDE_KEYSTREAM);
		$crate::words::deobfuscate::<_OBFWIDE_LEN>(
			$crate::xref::xref::<_,
				{$crate::random!(u32, "offset", stringify!($s))},
				{$crate::random!(u64, "xref", stringify!($s))}>
				(&_OBFWIDE_SDATA),
			&_OBFWIDE_KEYSTREAM)
	}};
}

// Simple XorShift to generate the key stream.
// Security doesn't matter, we just want a number of random-looking bytes.
#[inline(always)]
const fn next_round(mut x: u32) -> u32 {
	x ^= x << 13;
	x ^= x >> 17;
	x ^= x << 5;
	return x;
}

/// Generate the key stream for array of given length.
#[inline(always)]
pub const fn keystream<const LEN: usize>(key: u32) -> [u16; LEN] {
	let mut keys = [0u16; LEN];
	let mut round_key = key;
	let mut i = 0;
	// Calculate the key stream in chunks of 4 bytes
	while i < LEN & !1 {
		round_key = next_round(round_key);
		let kb = round_key.to_ne_bytes();
		keys[i + 0] = u16::from_ne_bytes([kb[0], kb[1]]);
		keys[i + 1] = u16::from_ne_bytes([kb[2], kb[3]]);
		i += 2;
	}
	// Calculate the remaining words of the key stream
	if LEN % 2 != 0 {
		round_key = next_round(round_key);
		keys[i] = round_key as u16;
	}
	return keys;
}

/// Obfuscates the input string and given key stream.
pub const fn obfuscate<const LEN: usize>(s: &[u16], k: &[u16; LEN]) -> [u16; LEN] {
	if s.len() != LEN {
		panic!("input string len not equal to key stream len");
	}
	let mut data = [0u16; LEN];
	let mut i = 0usize;
	while i < LEN {
		data[i] = s[i] ^ k[i];
		i += 1;
	}
	return data;
}

/// Deobfuscates the obfuscated input string and given key stream.
#[inline(always)]
pub fn deobfuscate<const LEN: usize>(s: &[u16; LEN], k: &[u16; LEN]) -> [u16; LEN] {
	let mut buf = [0u16; LEN];
	let mut i = 0;
	// Try to tickle the LLVM optimizer in _just_ the right way
	// Use `read_volatile` to avoid constant folding a specific read and optimize the rest
	// Volatile reads of any size larger than 8 bytes appears to cause a bunch of one byte reads
	// Hand optimize in chunks of 8 and 4 bytes to avoid this
	unsafe {
		let src = s.as_ptr();
		let dest = buf.as_mut_ptr();
		// Process in chunks of 8 bytes on 64-bit targets
		#[cfg(target_pointer_width = "64")]
		while i < LEN & !3 {
			let ct = read_volatile(src.offset(i as isize) as *const [u16; 4]);
			let tmp = [
				ct[0] ^ k[i + 0],
				ct[1] ^ k[i + 1],
				ct[2] ^ k[i + 2],
				ct[3] ^ k[i + 3],
			];
			write(dest.offset(i as isize) as *mut [u16; 4], tmp);
			i += 4;
		}
		// Process in chunks of 4 bytes
		while i < LEN & !1 {
			let ct = read_volatile(src.offset(i as isize) as *const [u16; 2]);
			let tmp = [
				ct[0] ^ k[i + 0],
				ct[1] ^ k[i + 1],
			];
			write(dest.offset(i as isize) as *mut [u16; 2], tmp);
			i += 2;
		}
		// Process the remaining bytes
		if LEN % 2 != 0 {
			let ct = read_volatile(src.offset(i as isize));
			write(dest.offset(i as isize), ct ^ k[i]);
		}
	}
	return buf;
}

// Test correct processing of less than multiple of 8 lengths
#[test]
fn test_remaining_bytes() {
	const STRING: &[u16] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
	fn test<const LEN: usize>(key: u32) {
		let keys = keystream::<LEN>(key);
		let data = obfuscate::<LEN>(&STRING[..LEN], &keys);
		let buffer = deobfuscate::<LEN>(&data, &keys);
		// Ciphertext should not equal input string
		assert_ne!(&data[..], &STRING[..LEN]);
		// Deobfuscated result should equal input string
		assert_eq!(&buffer[..], &STRING[..LEN]);
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
fn test_obfstr_let() {
	obfwide! {
		let hello = "hello";
		let world = "world";
	}
	assert_eq!(hello, crate::wide!("hello"));
	assert_eq!(world, crate::wide!("world"));
}

#[test]
fn test_obfstr_const() {
	const LONG_STRING: &str = "This literal is very very very long to see if it correctly handles long strings";
	assert_eq!(obfwide!(LONG_STRING), crate::wide!(LONG_STRING));

	const ABC: &str = "ABC";
	const WORLD: &str = "🌍";

	assert_eq!(obfwide!(ABC), &[b'A' as u16, b'B' as u16, b'C' as u16]);
	assert_eq!(obfwide!(WORLD), &[0xd83c, 0xdf0d]);
}
