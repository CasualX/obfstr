/*!
Compiletime string constant obfuscation.
*/

#![cfg_attr(not(test), no_std)]

use core::str;

#[doc(hidden)]
pub mod wide;

#[doc(hidden)]
pub mod cfo;

mod murmur3;
pub use self::murmur3::murmur3;

mod pos;
pub use self::pos::position;

mod xref;
pub use self::xref::{xref, xref_mut};

//----------------------------------------------------------------

/// Compiletime random number generator.
///
/// Supported types are `u8`, `u16`, `u32`, `u64`, `usize`, `i8`, `i16`, `i32`, `i64`, `isize`, `bool`, `f32` and `f64`.
///
/// The integer types generate a random value in their respective range.  
/// The float types generate a random value in range of `[1.0, 2.0)`.
///
/// While the result is generated at compiletime only the integer types are available in const contexts.
///
/// Note that the seed _must_ be a uniformly distributed random `u64` value.
/// If such a value is not available, see the [`splitmix`](fn.splitmix.html) function to generate it from non uniform random value.
///
/// ```
/// const RND: i32 = obfstr::random!(u8) as i32;
/// assert!(RND >= 0 && RND <= 255);
/// ```
///
/// The random machinery is robust enough that it avoids exact randomness when mixed with other macros:
///
/// ```
/// assert_ne!(obfstr::random!(u64), obfstr::random!(u64));
/// ```
#[macro_export]
macro_rules! random {
	($ty:ident) => {{ const _RANDOM_ENTROPY: u64 = $crate::entropy(file!(), line!(), column!()); $crate::random!($ty, _RANDOM_ENTROPY) }};

	(u8, $seed:expr) => { $seed as u8 };
	(u16, $seed:expr) => { $seed as u16 };
	(u32, $seed:expr) => { $seed as u32 };
	(u64, $seed:expr) => { $seed as u64 };
	(usize, $seed:expr) => { $seed as usize };
	(i8, $seed:expr) => { $seed as i8 };
	(i16, $seed:expr) => { $seed as i16 };
	(i32, $seed:expr) => { $seed as i32 };
	(i64, $seed:expr) => { $seed as i64 };
	(isize, $seed:expr) => { $seed as isize };
	(bool, $seed:expr) => { $seed as i64 >= 0 };
	(f32, $seed:expr) => { f32::from_bits(0b0_01111111 << (f32::MANTISSA_DIGITS - 1) | ($seed as u32 >> 9)) };
	(f64, $seed:expr) => { f64::from_bits(0b0_01111111111 << (f64::MANTISSA_DIGITS - 1) | ($seed >> 12)) };

	($ty:ident, $seed:expr) => { compile_error!(concat!("unsupported type: ", stringify!($ty))) };
}

/// Compiletime bitmixing.
///
/// Takes an intermediate hash that may not be thoroughly mixed and increase its entropy to obtain both better distribution.
/// See [Better Bit Mixing](https://zimbry.blogspot.com/2011/09/better-bit-mixing-improving-on.html) for reference.
#[inline(always)]
pub const fn splitmix(seed: u64) -> u64 {
	let next = seed.wrapping_add(0x9e3779b97f4a7c15);
	let mut z = next;
	z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
	z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
	return z ^ (z >> 31);
}

/// Compiletime string constant hash.
///
/// Implemented using the [DJB2 hash function](http://www.cse.yorku.ca/~oz/hash.html#djb2) xor variation.
#[inline(always)]
pub const fn hash(s: &str) -> u32 {
	let s = s.as_bytes();
	let mut result = 3581u32;
	let mut i = 0usize;
	while i < s.len() {
		result = result.wrapping_mul(33) ^ s[i] as u32;
		i += 1;
	}
	return result;
}

/// Compiletime string constant hash.
///
/// Helper macro guarantees compiletime evaluation of the string constant hash.
///
/// ```
/// const STRING: &str = "Hello World";
/// assert_eq!(obfstr::hash!(STRING), 0x6E4A573D);
/// ```
#[macro_export]
macro_rules! hash {
	($s:expr) => {{ const _DJB2_HASH: u32 = $crate::hash($s); _DJB2_HASH }};
}

/// Produces pseudorandom entropy given the file, line and column information.
#[doc(hidden)]
#[inline(always)]
pub const fn entropy(file: &str, line: u32, column: u32) -> u64 {
	splitmix(splitmix(splitmix(SEED ^ hash(file) as u64) ^ line as u64) ^ column as u64)
}

/// Compiletime RNG seed.
///
/// This value is derived from the environment variable `OBFSTR_SEED` and has a fixed value if absent.
/// If it changes all downstream dependents are recompiled automatically.
pub const SEED: u64 = splitmix(hash(match option_env!("OBFSTR_SEED") { Some(seed) => seed, None => "FIXED" }) as u64);

//----------------------------------------------------------------

#[doc(hidden)]
pub mod bytes;

#[doc(hidden)]
pub mod words;

#[doc(hidden)]
#[inline(always)]
pub fn unsafe_as_str(bytes: &[u8]) -> &str {
	// When used correctly by this crate's macros this should be safe
	#[cfg(debug_assertions)]
	return str::from_utf8(bytes).unwrap();
	#[cfg(not(debug_assertions))]
	return unsafe { str::from_utf8_unchecked(bytes) };
}

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
/// To reuse the deobfuscated string in the current scope it must be assigned to a local variable:
///
/// ```
/// use obfstr::obfstr;
///
/// obfstr! {
/// 	let s = "Hello 🌍";
///# 	let _another = "another";
/// }
/// assert_eq!(s, "Hello 🌍");
/// ```
///
/// To return an obfuscated string from a function pass a buffer.
/// Panics if the buffer is too small:
///
/// ```
/// use obfstr::obfstr;
///
/// fn helper(buf: &mut [u8]) -> &str {
/// 	obfstr!(buf <- "hello")
/// }
///
/// let mut buf = [0u8; 16];
/// assert_eq!(helper(&mut buf), "hello");
/// ```
///
/// The string constants can be prefixed with `L` to get an UTF-16 equivalent obfuscated string as `&[u16; LEN]`.
#[macro_export]
macro_rules! obfstr {
	($buf:ident <- $s:expr) => {{
		const _OBFSTR_STRING: &str = $s;
		const _OBFSTR_LEN: usize = _OBFSTR_STRING.len();
		const _OBFSTR_KEYSTREAM: [u8; _OBFSTR_LEN] = $crate::bytes::keystream::<_OBFSTR_LEN>($crate::random!(u32));
		static mut _OBFSTR_DATA: [u8; _OBFSTR_LEN] = $crate::bytes::obfuscate::<_OBFSTR_LEN>(_OBFSTR_STRING.as_bytes(), &_OBFSTR_KEYSTREAM);
		let buf = &mut $buf[.._OBFSTR_LEN];
		buf.copy_from_slice(&$crate::bytes::deobfuscate::<_OBFSTR_LEN>($crate::xref(unsafe { &_OBFSTR_DATA }, $crate::random!(usize) & 0xffff), &_OBFSTR_KEYSTREAM));
		$crate::unsafe_as_str(buf)
	}};
	($buf:ident <- L$s:expr) => {{
		const _OBFSTR_STRING: &[u16] = $crate::wide!($s);
		const _OBFSTR_LEN: usize = _OBFSTR_STRING.len();
		const _OBFSTR_KEYSTREAM: [u16; _OBFSTR_LEN] = $crate::words::keystream::<_OBFSTR_LEN>($crate::random!(u32));
		static mut _OBFSTR_DATA: [u16; _OBFSTR_LEN] = $crate::words::obfuscate::<_OBFSTR_LEN>(_OBFSTR_STRING, &_OBFSTR_KEYSTREAM);
		let buf = &mut $buf[.._OBFSTR_LEN];
		buf.copy_from_slice(&$crate::words::deobfuscate::<_OBFSTR_LEN>($crate::xref(unsafe { &_OBFSTR_DATA }, $crate::random!(usize) & 0xffff), &_OBFSTR_KEYSTREAM));
		buf
	}};

	($s:expr) => {{
		const _OBFSTR_STRING: &str = $s;
		const _OBFSTR_LEN: usize = _OBFSTR_STRING.len();
		const _OBFSTR_KEYSTREAM: [u8; _OBFSTR_LEN] = $crate::bytes::keystream::<_OBFSTR_LEN>($crate::random!(u32));
		static _OBFSTR_DATA: [u8; _OBFSTR_LEN] = $crate::bytes::obfuscate::<_OBFSTR_LEN>(_OBFSTR_STRING.as_bytes(), &_OBFSTR_KEYSTREAM);
		$crate::unsafe_as_str(&$crate::bytes::deobfuscate::<_OBFSTR_LEN>($crate::xref(&_OBFSTR_DATA, $crate::random!(usize) & 0xffff), &_OBFSTR_KEYSTREAM))
	}};
	(L$s:expr) => {{
		const _OBFSTR_STRING: &[u16] = $crate::wide!($s);
		const _OBFSTR_LEN: usize = _OBFSTR_STRING.len();
		const _OBFSTR_KEYSTREAM: [u16; _OBFSTR_LEN] = $crate::words::keystream::<_OBFSTR_LEN>($crate::random!(u32));
		static _OBFSTR_DATA: [u16; _OBFSTR_LEN] = $crate::words::obfuscate::<_OBFSTR_LEN>(_OBFSTR_STRING, &_OBFSTR_KEYSTREAM);
		&$crate::words::deobfuscate::<_OBFSTR_LEN>($crate::xref(&_OBFSTR_DATA, $crate::random!(usize) & 0xffff), &_OBFSTR_KEYSTREAM)
	}};

	($(let $name:ident = $s:expr;)*) => {$(
		let $name = {
			const _OBFSTR_STRING: &str = $s;
			const _OBFSTR_LEN: usize = _OBFSTR_STRING.len();
			const _OBFSTR_KEYSTREAM: [u8; _OBFSTR_LEN] = $crate::bytes::keystream::<_OBFSTR_LEN>($crate::random!(u32));
			static _OBFSTR_DATA: [u8; _OBFSTR_LEN] = $crate::bytes::obfuscate::<_OBFSTR_LEN>(_OBFSTR_STRING.as_bytes(), &_OBFSTR_KEYSTREAM);
			$crate::bytes::deobfuscate::<_OBFSTR_LEN>($crate::xref(&_OBFSTR_DATA, $crate::random!(usize) & 0xffff), &_OBFSTR_KEYSTREAM)
		};
		let $name = $crate::unsafe_as_str(&$name);
	)*};
	($(let $name:ident = L$s:expr;)*) => {$(
		let $name = {
			const _OBFSTR_STRING: &[u16] = $crate::wide!($s);
			const _OBFSTR_LEN: usize = _OBFSTR_STRING.len();
			const _OBFSTR_KEYSTREAM: [u16; _OBFSTR_LEN] = $crate::words::keystream::<_OBFSTR_LEN>($crate::random!(u32));
			static _OBFSTR_DATA: [u16; _OBFSTR_LEN] = $crate::words::obfuscate::<_OBFSTR_LEN>(_OBFSTR_STRING, &_OBFSTR_KEYSTREAM);
			$crate::words::deobfuscate::<_OBFSTR_LEN>($crate::xref(&_OBFSTR_DATA, $crate::random!(usize) & 0xffff), &_OBFSTR_KEYSTREAM)
		};
		let $name = &$name;
	)*};
}

#[test]
fn test_obfstr_let() {
	obfstr! {
		let abc = "abc";
		let def = "defdef";
	}
	assert_eq!(abc, "abc");
	assert_eq!(def, "defdef");
	obfstr! {
		let hello = L"hello";
		let world = L"world";
	}
	assert_eq!(hello, wide!("hello"));
	assert_eq!(world, wide!("world"));
}

#[test]
fn test_obfstr_const() {
	assert_eq!(obfstr!("\u{20}\0"), " \0");
	assert_eq!(obfstr!("\"\n\t\\\'\""), "\"\n\t\\\'\"");

	const LONG_STRING: &str = "This literal is very very very long to see if it correctly handles long strings";
	assert_eq!(obfstr!(LONG_STRING), LONG_STRING);

	const ABC: &str = "ABC";
	const WORLD: &str = "🌍";

	assert_eq!(obfstr!(L ABC), &[b'A' as u16, b'B' as u16, b'C' as u16]);
	assert_eq!(obfstr!(L WORLD), &[0xd83c, 0xdf0d]);
}
