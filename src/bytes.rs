/*!
Byte string obfuscation
=======================
*/

use core::ptr::{read_volatile, write};

/// Compiletime string constant obfuscation.
///
/// The purpose of the obfuscation is to make it difficult to discover the original strings with automated analysis.
/// String obfuscation is not intended to hinder a dedicated reverse engineer from discovering the original string.
/// This should not be used to hide secrets in client binaries and the author disclaims any responsibility for any damages resulting from ignoring this warning.
///
/// The `obfstr!` macro returns the deobfuscated string as a temporary `&str` value and must be consumed in the same statement it was used:
///
/// ```
/// use obfstr::obfstr;
///
/// const HELLO_WORLD: &str = "Hello 🌍";
/// assert_eq!(obfstr!(HELLO_WORLD), HELLO_WORLD);
/// ```
///
/// Different syntax forms are supported to reuse the obfuscated strings in outer scopes:
///
/// ```
/// use obfstr::obfstr;
///
/// // Obfuscate a bunch of strings
/// obfstr! {
/// 	let s = "Hello world";
/// 	let another = "another";
/// }
/// assert_eq!(s, "Hello world");
/// assert_eq!(another, "another");
///
/// // Assign to an uninit variable in outer scope
/// let (true_string, false_string);
/// let string = if true {
/// 	obfstr!(true_string = "true")
/// }
/// else {
/// 	obfstr!(false_string = "false")
/// };
/// assert_eq!(string, "true");
///
/// // Return an obfuscated string from a function
/// fn helper(buf: &mut [u8]) -> &str {
/// 	obfstr!(buf <- "hello")
/// }
/// let mut buf = [0u8; 16];
/// assert_eq!(helper(&mut buf), "hello");
/// ```
#[macro_export]
macro_rules! obfstr {
	($(let $name:ident = $s:expr;)*) => {$(
		$crate::obfbytes! { let $name = $s.as_bytes(); }
		let $name = $crate::unsafe_as_str($name);
	)*};
	($name:ident = $s:expr) => {
		$crate::unsafe_as_str($crate::obfbytes!($name = $s.as_bytes()))
	};
	($buf:ident <- $s:expr) => {
		$crate::unsafe_as_str($crate::obfbytes!($buf <- $s.as_bytes()))
	};
	($s:expr) => {
		$crate::unsafe_as_str($crate::obfbytes!($s.as_bytes()))
	};
}

/// Compiletime byte string obfuscation.
#[macro_export]
macro_rules! obfbytes {
	($(let $name:ident = $s:expr;)*) => {
		$(let ref $name = $crate::__obfbytes!($s);)*
	};
	($name:ident = $s:expr) => {{
		$name = $crate::__obfbytes!($s);
		&$name
	}};
	($buf:ident <- $s:expr) => {{
		let buf = &mut $buf[..$s.len()];
		buf.copy_from_slice(&$crate::__obfbytes!($s));
		buf
	}};
	($s:expr) => {
		&$crate::__obfbytes!($s)
	};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __obfbytes {
	($s:expr) => {{
		const _OBFBYTES_STRING: &[u8] = $s;
		const _OBFBYTES_LEN: usize = _OBFBYTES_STRING.len();
		const _OBFBYTES_KEYSTREAM: [u8; _OBFBYTES_LEN] = $crate::bytes::keystream::<_OBFBYTES_LEN>($crate::__entropy!("key", stringify!($s)) as u32);
		static _OBFBYTES_SDATA: [u8; _OBFBYTES_LEN] = $crate::bytes::obfuscate::<_OBFBYTES_LEN>(_OBFBYTES_STRING, &_OBFBYTES_KEYSTREAM);
		$crate::bytes::deobfuscate::<_OBFBYTES_LEN>(
			$crate::__xref!(
				$crate::__entropy!("offset", stringify!($s)) as usize,
				$crate::__entropy!("xref", stringify!($s)),
				&_OBFBYTES_SDATA),
			&_OBFBYTES_KEYSTREAM)
	}};
}

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

#[test]
fn test_obfstr_let() {
	obfstr! {
		let abc = "abc";
		let def = "defdef";
	}
	assert_eq!(abc, "abc");
	assert_eq!(def, "defdef");
}

#[test]
fn test_obfstr_const() {
	assert_eq!(obfstr!("\u{20}\0"), " \0");
	assert_eq!(obfstr!("\"\n\t\\\'\""), "\"\n\t\\\'\"");

	const LONG_STRING: &str = "This literal is very very very long to see if it correctly handles long strings";
	assert_eq!(obfstr!(LONG_STRING), LONG_STRING);

	const ABC: &str = "ABC";
	const WORLD: &str = "🌍";

	assert_eq!(obfbytes!(ABC.as_bytes()), "ABC".as_bytes());
	assert_eq!(obfbytes!(WORLD.as_bytes()), "🌍".as_bytes());
}
