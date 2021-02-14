/*!
Compiletime string constant obfuscation.
*/

#![cfg_attr(not(test), no_std)]

use core::{char, fmt, ptr, str};

#[doc(hidden)]
pub mod wide;

#[doc(hidden)]
pub mod cfo;

mod murmur3;
pub use self::murmur3::murmur3;

mod pos;
pub use self::pos::position;

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
	($_:ident, $seed:expr) => { compile_error!(concat!("unsupported type: ", stringify!($_))) };
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
/// Implemented using the [DJB2 hash function](http://www.cse.yorku.ca/~oz/hash.html#djb2).
#[inline(always)]
pub const fn hash(s: &str) -> u32 {
	let s = s.as_bytes();
	let mut result = 3581u32;
	let mut i = 0usize;
	while i < s.len() {
		result = result.wrapping_mul(33).wrapping_add(s[i] as u32);
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
/// assert_eq!(obfstr::hash!(STRING), 1481604729);
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
pub const SEED: u64 = splitmix(hash(env!("OBFSTR_SEED")) as u64);

//----------------------------------------------------------------

/// Obfuscated string constant data.
///
/// This type represents the data baked in the binary and holds the key and obfuscated string.
#[doc(hidden)]
#[repr(C)]
pub struct ObfString<A> {
	key: u32,
	data: A,
}

/// Deobfuscated string buffer.
#[doc(hidden)]
#[repr(transparent)]
pub struct ObfBuffer<A: ?Sized>(#[doc(hidden)] pub A);

impl<A: ?Sized> AsRef<A> for ObfBuffer<A> {
	#[inline]
	fn as_ref(&self) -> &A {
		&self.0
	}
}

//----------------------------------------------------------------
// Byte strings.

#[doc(hidden)]
pub mod bytes;

impl<const LEN: usize> ObfString<[u8; LEN]> {
	/// Obfuscates the string with the given key.
	///
	/// Do not call this function directly, use the provided macros instead.
	#[doc(hidden)]
	#[inline(always)]
	pub const fn obfuscate(key: u32, s: &str) -> ObfString<[u8; LEN]> {
		let keys = self::bytes::keystream::<LEN>(key);
		let data = self::bytes::obfuscate::<LEN>(s.as_bytes(), &keys);
		ObfString { key, data }
	}
	/// Deobfuscates the string and returns the buffer.
	#[inline(always)]
	pub fn deobfuscate(&self, _x: usize) -> ObfBuffer<[u8; LEN]> {
		let keys = self::bytes::keystream::<LEN>(self.key);
		let buffer = self::bytes::deobfuscate::<LEN>(&self.data, &keys);
		ObfBuffer(buffer)
	}
}
impl<const LEN: usize> PartialEq<&str> for ObfString<[u8; LEN]> {
	#[inline(always)]
	fn eq(&self, other: &&str) -> bool {
		let keys = self::bytes::keystream::<LEN>(self.key);
		self::bytes::equals::<LEN>(&self.data, &keys, other.as_bytes())
	}
}
impl<const LEN: usize> PartialEq<ObfString<[u8; LEN]>> for &str {
	#[inline(always)]
	fn eq(&self, other: &ObfString<[u8; LEN]>) -> bool {
		let keys = self::bytes::keystream::<LEN>(other.key);
		self::bytes::equals::<LEN>(&other.data, &keys, self.as_bytes())
	}
}

impl<const LEN: usize> ObfBuffer<[u8; LEN]> {
	#[inline]
	pub const fn as_slice(&self) -> &[u8] {
		&self.0
	}
	#[inline]
	pub fn as_str(&self) -> &str {
		// This should be safe as it can only be constructed from a string constant...
		#[cfg(debug_assertions)]
		return str::from_utf8(&self.0).unwrap();
		#[cfg(not(debug_assertions))]
		return unsafe { str::from_utf8_unchecked(&self.0) };
	}
	// For use with serde's stupid 'static limitations...
	#[cfg(feature = "unsafe_static_str")]
	#[inline]
	pub fn unsafe_as_static_str(&self) -> &'static str {
		unsafe { &*(self.as_str() as *const str) }
	}
}
impl<const LEN: usize> fmt::Debug for ObfBuffer<[u8; LEN]> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.as_str().fmt(f)
	}
}

//----------------------------------------------------------------
// Word strings.

#[doc(hidden)]
pub mod words;

impl<const LEN: usize> ObfString<[u16; LEN]> {
	/// Obfuscates the string with the given key.
	///
	/// Do not call this function directly, use the provided macros instead.
	#[doc(hidden)]
	pub const fn obfuscate(key: u32, string: &str) -> ObfString<[u16; LEN]> {
		let keys = self::words::keystream::<LEN>(key);
		let string = self::wide::encode::<LEN>(string);
		let data = self::words::obfuscate::<LEN>(&string, &keys);
		ObfString { key, data }
	}
	/// Deobfuscates the string and returns the buffer.
	#[inline(always)]
	pub fn deobfuscate(&self, _x: usize) -> ObfBuffer<[u16; LEN]> {
		let keys = self::words::keystream::<LEN>(self.key);
		let buffer = self::words::deobfuscate::<LEN>(&self.data, &keys);
		ObfBuffer(buffer)
	}
}
impl<const LEN: usize> fmt::Debug for ObfString<[u16; LEN]> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.deobfuscate(0).fmt(f)
	}
}

impl<const LEN: usize> ObfBuffer<[u16; LEN]> {
	#[inline]
	pub const fn as_slice(&self) -> &[u16] {
		&self.0
	}
}
impl<const LEN: usize> fmt::Debug for ObfBuffer<[u16; LEN]> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use fmt::Write;
		f.write_str("\"")?;
		for chr in char::decode_utf16(self.as_slice().iter().cloned()) {
			f.write_char(chr.unwrap_or(char::REPLACEMENT_CHARACTER))?;
		}
		f.write_str("\"")
	}
}

//----------------------------------------------------------------

/// Obfuscates the xref to static data.
///
/// ```
/// static FOO: i32 = 42;
/// let foo = obfstr::xref!(&FOO);
///
/// // When looking at the disassembly the reference to `FOO` has been obfuscated.
/// assert_eq!(foo as *const _, &FOO as *const _);
/// ```
#[macro_export]
macro_rules! xref {
	($e:expr) => { $crate::xref($e, $crate::random!(usize) & 0xffff) };
}

/// Obfuscates the xref to static data.
///
/// The offset can be initialized with [`random!`] for a compiletime random value.
///
/// ```
/// static FOO: i32 = 42;
/// let foo = obfstr::xref(&FOO, 0x123);
///
/// // When looking at the disassembly the reference to `FOO` has been obfuscated.
/// assert_eq!(foo as *const _, &FOO as *const _);
/// ```
#[inline(always)]
pub fn xref<T: ?Sized>(p: &'static T, offset: usize) -> &'static T {
	unsafe {
		let mut p: *const T = p;
		// To avoid LLMV optimizing away the obfuscation, launder it through read_volatile
		let val = ptr::read_volatile(&(p as *const u8).wrapping_sub(offset)).wrapping_add(offset);
		// set_ptr_value
		*(&mut p as *mut *const T as *mut *const u8) = val;
		&*p
	}
}

/// Obfuscates the xref to static data.
///
/// ```
/// static mut FOO: i32 = 42;
/// let foo = obfstr::xref_mut!(unsafe { &mut FOO });
///
/// // When looking at the disassembly the reference to `FOO` has been obfuscated.
/// assert_eq!(foo as *mut _, unsafe { &mut FOO } as *mut _);
/// ```
#[macro_export]
macro_rules! xref_mut {
	($e:expr) => { $crate::xref_mut($e, $crate::random!(usize) & 0xffff) };
}

/// Obfuscates the xref to static data.
///
/// The offset can be initialized with [`random!`] for a compiletime random value.
///
/// ```
/// static mut FOO: i32 = 42;
/// let foo = obfstr::xref_mut(unsafe { &mut FOO }, 0x321);
///
/// // When looking at the disassembly the reference to `FOO` has been obfuscated.
/// assert_eq!(foo as *mut _, unsafe { &mut FOO } as *mut _);
/// ```
#[inline(always)]
pub fn xref_mut<T: ?Sized>(p: &'static mut T, offset: usize) -> &'static mut T {
	unsafe {
		let mut p: *mut T = p;
		// To avoid LLMV optimizing away the obfuscation, launder it through read_volatile
		let val = ptr::read_volatile(&(p as *mut u8).wrapping_sub(offset)).wrapping_add(offset);
		// set_ptr_value
		*(&mut p as *mut *mut T as *mut *mut u8) = val;
		&mut *p
	}
}

#[test]
fn test_xref_slice() {
	static FOO: [i32; 42] = [13; 42];
	let foo = xref::<[i32]>(&FOO[..], 0x1000);
	assert_eq!(foo as *const _, &FOO as *const _);
}

//----------------------------------------------------------------

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
		static mut _OBFSTR_DATA: [u8; _OBFSTR_LEN] = $crate::bytes::obfuscate::<_OBFSTR_LEN>(_OBFSTR_STRING.as_bytes(), &_OBFSTR_KEYSTREAM);
		$crate::unsafe_as_str(&$crate::bytes::deobfuscate::<_OBFSTR_LEN>($crate::xref(unsafe { &_OBFSTR_DATA }, $crate::random!(usize) & 0xffff), &_OBFSTR_KEYSTREAM))
	}};
	(L$s:expr) => {{
		const _OBFSTR_STRING: &[u16] = $crate::wide!($s);
		const _OBFSTR_LEN: usize = _OBFSTR_STRING.len();
		const _OBFSTR_KEYSTREAM: [u16; _OBFSTR_LEN] = $crate::words::keystream::<_OBFSTR_LEN>($crate::random!(u32));
		static mut _OBFSTR_DATA: [u16; _OBFSTR_LEN] = $crate::words::obfuscate::<_OBFSTR_LEN>(_OBFSTR_STRING, &_OBFSTR_KEYSTREAM);
		&$crate::words::deobfuscate::<_OBFSTR_LEN>($crate::xref(unsafe { &_OBFSTR_DATA }, $crate::random!(usize) & 0xffff), &_OBFSTR_KEYSTREAM)
	}};

	($(let $name:ident = $s:expr;)*) => {$(
		let $name = {
			const _OBFSTR_STRING: &str = $s;
			const _OBFSTR_LEN: usize = _OBFSTR_STRING.len();
			const _OBFSTR_KEYSTREAM: [u8; _OBFSTR_LEN] = $crate::bytes::keystream::<_OBFSTR_LEN>($crate::random!(u32));
			static mut _OBFSTR_DATA: [u8; _OBFSTR_LEN] = $crate::bytes::obfuscate::<_OBFSTR_LEN>(_OBFSTR_STRING.as_bytes(), &_OBFSTR_KEYSTREAM);
			$crate::bytes::deobfuscate::<_OBFSTR_LEN>($crate::xref(unsafe { &_OBFSTR_DATA }, $crate::random!(usize) & 0xffff), &_OBFSTR_KEYSTREAM)
		};
		let $name = $crate::unsafe_as_str(&$name);
	)*};
	($(let $name:ident = L$s:expr;)*) => {$(
		let $name = {
			const _OBFSTR_STRING: &[u16] = $crate::wide!($s);
			const _OBFSTR_LEN: usize = _OBFSTR_STRING.len();
			const _OBFSTR_KEYSTREAM: [u16; _OBFSTR_LEN] = $crate::words::keystream::<_OBFSTR_LEN>($crate::random!(u32));
			static mut _OBFSTR_DATA: [u16; _OBFSTR_LEN] = $crate::words::obfuscate::<_OBFSTR_LEN>(_OBFSTR_STRING, &_OBFSTR_KEYSTREAM);
			$crate::words::deobfuscate::<_OBFSTR_LEN>($crate::xref(unsafe { &_OBFSTR_DATA }, $crate::random!(usize) & 0xffff), &_OBFSTR_KEYSTREAM)
		};
		let $name = &$name;
	)*};
}

// Backwards compatibility.
//
// Prefer `obfstr! { let name = "string"; }` which avoids leaking the `ObfBuffer` type.
#[doc(hidden)]
#[macro_export]
macro_rules! obflocal {
	($s:expr) => { $crate::obfconst!($s).deobfuscate(0) };
	(L$s:expr) => { $crate::obfconst!(L$s).deobfuscate(0) };
}

// Backwards compatibility.
#[doc(hidden)]
#[macro_export]
macro_rules! obfconst {
	($s:expr) => {{ const STRING: $crate::ObfString<[u8; {$s.len()}]> = $crate::ObfString::<[u8; {$s.len()}]>::obfuscate($crate::random!(u32), $s); STRING }};
	(L$s:expr) => {{ const STRING: $crate::ObfString<[u16; {$crate::wide_len($s)}]> = $crate::ObfString::<[u16; {$crate::wide_len($s)}]>::obfuscate($crate::random!(u32), $s); STRING }};
}

// Backwards compatibility.
//
// This macro was removed due to confusion of the order of arguments.
#[doc(hidden)]
#[macro_export]
macro_rules! obfeq {
	($e:expr, $s:expr) => {
		$e == $crate::obfconst!($s)
	};
	($e:expr, L$s:expr) => {
		$e == $crate::obfstr!(L$s)
	};
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

#[test]
fn test_obfconst_equals() {
	const LONG_STRING: &str = "This literal is very very very long to see if it correctly handles long strings";

	assert!(LONG_STRING == obfconst!(LONG_STRING));
	assert!("Hello ðŸŒ" == obfconst!("Hello ðŸŒ"));
}
