/*!
Compiletime string literal obfuscation.
!*/

#![no_std]
#![feature(fixed_size_array)]

use core::{char, fmt, mem, ops, ptr, slice, str};
use core::array::FixedSizeArray;

// Reexport these because reasons...
#[doc(hidden)]
pub use obfstr_impl::*;

/// Compiletime string literal obfuscation, returns a borrowed temporary and may not escape the statement it was used in.
///
/// Prefix the string literal with `L` to get an UTF-16 obfuscated string.
///
/// ```
/// assert_eq!(obfstr::obfstr!("Hello 🌍"), "Hello 🌍");
/// ```
#[macro_export]
macro_rules! obfstr {
	($string:literal) => {{
		#[$crate::obfstr_attribute]
		const S: $crate::ObfString<[u8; _strlen_!($string)]> = $crate::ObfString::new(_obfstr_!($string));
		S.decrypt($crate::random!(usize) % 4096).as_str()
	}};
	(L$string:literal) => {{
		#[$crate::obfstr_attribute]
		const S: $crate::WObfString<[u16; _strlen_!($string)]> = $crate::WObfString::new(_obfstr_!(L$string));
		S.decrypt($crate::random!(usize) % 4096).as_wide()
	}};
}

/// Compiletime string literal obfuscation, returns the decrypted [`ObfBuffer`](struct.ObfBuffer.html) for assignment to local variable.
///
/// Prefix the string literal with `L` to get an UTF-16 obfuscated string.
///
/// ```
/// let str_buf = obfstr::obflocal!("Hello 🌍");
/// assert_eq!(str_buf.as_str(), "Hello 🌍");
/// ```
#[macro_export]
macro_rules! obflocal {
	($string:literal) => {{
		#[$crate::obfstr_attribute]
		const S: $crate::ObfString<[u8; _strlen_!($string)]> = $crate::ObfString::new(_obfstr_!($string));
		S.decrypt($crate::random!(usize) % 4096)
	}};
	(L$string:literal) => {{
		#[$crate::obfstr_attribute]
		const S: $crate::WObfString<[u16; _strlen_!($string)]> = $crate::WObfString::new(_obfstr_!(L$string));
		S.decrypt($crate::random!(usize) % 4096)
	}};
}

/// Compiletime string literal obfuscation, returns the encrypted [`ObfString`](struct.ObfString.html) for use in constant expressions.
///
/// Prefix the string literal with `L` to get an UTF-16 obfuscated string.
///
/// ```
/// static GSTR: obfstr::ObfString<[u8; 10]> = obfstr::obfconst!("Hello 🌍");
/// assert_eq!(GSTR.decrypt(0).as_str(), "Hello 🌍");
/// ```
#[macro_export]
macro_rules! obfconst {
	($string:literal) => {{
		#[$crate::obfstr_attribute]
		const S: $crate::ObfString<[u8; _strlen_!($string)]> = $crate::ObfString::new(_obfstr_!($string)); S
	}};
	(L$string:literal) => {{
		#[$crate::obfstr_attribute]
		const S: $crate::WObfString<[u16; _strlen_!($string)]> = $crate::WObfString::new(_obfstr_!(L$string)); S
	}};
}

/// Compiletime string obfuscation for serde.
///
/// Serde unhelpfully requires `&'static str` literals in various places.
/// To work around these limitations an unsafe macro is provided which unsafely returns a static string slice.
/// This is probably fine as long as the underlying serializer doesn't rely on the staticness of the string slice.
#[cfg(feature = "unsafe_static_str")]
#[macro_export]
macro_rules! unsafe_obfstr {
	($string:literal) => {{
		#[$crate::obfstr_attribute]
		const S: $crate::ObfString<[u8; _strlen_!($string)]> = $crate::ObfString::new(_obfstr_!($string));
		S.decrypt($crate::random!(usize) % 4096).unsafe_as_static_str()
	}};
}

/// Wide string literal of type `&'static [u16; LEN]`.
///
/// ```
/// let expected = &['W' as u16, 'i' as u16, 'd' as u16, 'e' as u16];
/// assert_eq!(obfstr::wide!("Wide"), expected);
/// ```
#[macro_export]
macro_rules! wide {
	($s:literal) => {{
		#[$crate::wide_attribute]
		const W: &[u16] = _wide_!($s); W
	}};
}

/// Compiletime random number generator.
///
/// Every time the code is compiled, a new random number literal is generated.
/// Recompilation (and thus regeneration of the number) is not triggered automatically.
///
/// Supported types are `u8`, `u16`, `u32`, `u64`, `usize`, `i8`, `i16`, `i32`, `i64`, `isize`, `bool`, `f32` and `f64`.
///
/// ```
/// const RND: i32 = obfstr::random!(u8) as i32;
/// assert!(RND >= 0 && RND <= 255);
/// ```
#[macro_export]
macro_rules! random {
	($ty:ident) => {{
		#[$crate::random_attribute]
		const N: $ty = _random_!($ty); N
	}};
}

//----------------------------------------------------------------

fn next_round(mut x: u32) -> u32 {
	x ^= x << 13;
	x ^= x >> 17;
	x ^= x << 5;
	x
}

const XREF_SHIFT: usize = ((random!(u8) & 31) + 32) as usize;

//----------------------------------------------------------------
// String implementation

/// Obfuscated string constant data.
///
/// This type represents the data baked in the binary and holds the key and obfuscated string.
#[repr(C)]
pub struct ObfString<A> {
	pub key: u32,
	pub data: A,
}
impl<A> ObfString<A> {
	/// Constructor.
	pub const fn new(key: u32, data: A) -> ObfString<A> {
		ObfString { key, data }
	}
}
impl<A: FixedSizeArray<u8>> ObfString<A> {
	/// Decrypts the obfuscated string and returns the buffer.
	#[inline(always)]
	pub fn decrypt(&self, x: usize) -> ObfBuffer<A> {
		unsafe {
			let mut buffer = ObfBuffer::<A>::uninit();
			let data = self.data.as_slice();
			let src = data.as_ptr() as usize - data.len() * XREF_SHIFT;
			let f: unsafe fn(&mut [u8], usize) = mem::transmute(ptr::read_volatile(&(decryptbuf as usize + x)) - x);
			f(buffer.0.as_mut_slice(), src);
			buffer
		}
	}
}
#[inline(never)]
unsafe fn decryptbuf(dest: &mut [u8], src: usize) {
	let mut key = *((src + dest.len() * XREF_SHIFT - 4) as *const u32);
	let data = slice::from_raw_parts((src + dest.len() * XREF_SHIFT) as *const u8, dest.len());
	for i in 0..data.len() {
		key = next_round(key);
		dest[i] = data[i].wrapping_add(key as u8);
	}
}
/// Obfuscated string buffer.
///
/// This type represents the string buffer after decryption on the stack.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct ObfBuffer<A>(A);
impl<A: FixedSizeArray<u8>> ObfBuffer<A> {
	unsafe fn uninit() -> Self {
		mem::uninitialized()
	}
	#[inline]
	pub fn as_str(&self) -> &str {
		#[cfg(debug_assertions)]
		return str::from_utf8(self.0.as_slice()).unwrap();
		#[cfg(not(debug_assertions))]
		return unsafe { str::from_utf8_unchecked(self.0.as_slice()) };
	}
	// For use with serde's stupid 'static limitations...
	#[cfg(feature = "unsafe_static_str")]
	#[inline]
	pub fn unsafe_as_static_str(&self) -> &'static str {
		unsafe { &*(self.as_str() as *const _) }
	}
}
impl<A: FixedSizeArray<u8>> ops::Deref for ObfBuffer<A> {
	type Target = str;
	#[inline]
	fn deref(&self) -> &str {
		self.as_str()
	}
}
impl<A: FixedSizeArray<u8>> fmt::Debug for ObfBuffer<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.as_str().fmt(f)
	}
}
impl<A: FixedSizeArray<u8>> fmt::Display for ObfBuffer<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.as_str().fmt(f)
	}
}

//----------------------------------------------------------------
// Widestr implementation

/// Obfuscated wide string constant data.
///
/// This type represents the data baked in the binary and holds the key and obfuscated wide string.
#[repr(C)]
pub struct WObfString<A> {
	pub key: u32,
	pub data: A,
}
impl<A> WObfString<A> {
	/// Constructor.
	pub const fn new(key: u32, data: A) -> WObfString<A> {
		WObfString { key, data }
	}
}
impl<A: FixedSizeArray<u16>> WObfString<A> {
	/// Decrypts the obfuscated wide string and returns the buffer.
	#[inline(always)]
	pub fn decrypt(&self, x: usize) -> WObfBuffer<A> {
		unsafe {
			let mut buffer = WObfBuffer::<A>::uninit();
			let data = self.data.as_slice();
			let src = data.as_ptr() as usize - data.len() * XREF_SHIFT;
			let f: unsafe fn(&mut [u16], usize) = mem::transmute(ptr::read_volatile(&(wdecryptbuf as usize + x)) - x);
			f(buffer.0.as_mut_slice(), src);
			buffer
		}
	}
}
#[inline(never)]
unsafe fn wdecryptbuf(dest: &mut [u16], src: usize) {
	let mut key = *((src + dest.len() * XREF_SHIFT - 4) as *const u32);
	let data = slice::from_raw_parts((src + dest.len() * XREF_SHIFT) as *const u16, dest.len());
	for i in 0..data.len() {
		key = next_round(key);
		dest[i] = data[i].wrapping_add(key as u16);
	}
}
/// Obfuscated wide string buffer.
///
/// This type represents the wide string buffer after decryption on the stack.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct WObfBuffer<A>(A);
impl<A: FixedSizeArray<u16>> WObfBuffer<A> {
	unsafe fn uninit() -> Self {
		mem::uninitialized()
	}
	#[inline]
	pub fn as_wide(&self) -> &[u16] {
		self.0.as_slice()
	}
}
impl<A: FixedSizeArray<u16>> ops::Deref for WObfBuffer<A> {
	type Target = [u16];
	#[inline]
	fn deref(&self) -> &[u16] {
		self.as_wide()
	}
}
impl<A: FixedSizeArray<u16>> fmt::Debug for WObfBuffer<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use fmt::Write;
		f.write_str("\"")?;
		for chr in char::decode_utf16(self.as_wide().iter().cloned()) {
			f.write_char(chr.unwrap_or(char::REPLACEMENT_CHARACTER))?;
		}
		f.write_str("\"")
	}
}
impl<A: FixedSizeArray<u16>> fmt::Display for WObfBuffer<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use fmt::Write;
		for chr in char::decode_utf16(self.as_wide().iter().cloned()) {
			f.write_char(chr.unwrap_or(char::REPLACEMENT_CHARACTER))?;
		}
		Ok(())
	}
}
