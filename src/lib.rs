/*!
Compiletime string literal obfuscation.
!*/

#![no_std]

// Reexport these because reasons...
#[doc(hidden)]
pub use obfstr_impl::*;
pub use obfstr_impl::wide;

#[cfg(feature = "rand")]
pub use cfgd::*;
#[cfg(feature = "rand")]
mod cfgd {

use core::{char, fmt, mem, ops, ptr, slice, str};
use core::mem::MaybeUninit;

pub use obfstr_impl::random;

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
		S.decrypt($crate::random!(usize) & 0xffff).as_str()
	}};
	(L$string:literal) => {{
		#[$crate::obfstr_attribute]
		const S: $crate::WObfString<[u16; _strlen_!($string)]> = $crate::WObfString::new(_obfstr_!(L$string));
		S.decrypt($crate::random!(usize) & 0xffff).as_wide()
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
		S.decrypt($crate::random!(usize) & 0xffff)
	}};
	(L$string:literal) => {{
		#[$crate::obfstr_attribute]
		const S: $crate::WObfString<[u16; _strlen_!($string)]> = $crate::WObfString::new(_obfstr_!(L$string));
		S.decrypt($crate::random!(usize) & 0xffff)
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
		S.decrypt($crate::random!(usize) & 0xffff).unsafe_as_static_str()
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ObfString<A> {
	key: u32,
	data: A,
}
impl<A> ObfString<A> {
	/// Constructor.
	pub const fn new(key: u32, data: A) -> ObfString<A> {
		ObfString { key, data }
	}
}
impl<A: AsRef<[u8]> + AsMut<[u8]>> ObfString<A> {
	/// Decrypts the obfuscated string and returns the buffer.
	///
	/// The `x` argument should be a compiletime random 16-bit value.
	/// It is used to obfuscate the underlying call to the decrypt routine.
	#[inline(always)]
	pub fn decrypt(&self, x: usize) -> ObfBuffer<A> {
		unsafe {
			let mut buffer = MaybeUninit::uninit();
			let data = self.data.as_ref();
			let src = data.as_ptr() as usize - data.len() * XREF_SHIFT;
			let f: unsafe fn(*mut u8, usize, usize) = mem::transmute(ptr::read_volatile(&(decryptbuf as usize + x)) - x);
			f(buffer.as_mut_ptr() as *mut u8, mem::size_of::<A>(), src);
			buffer.assume_init()
		}
	}
}
impl<A: AsRef<[u8]> + AsMut<[u8]>> fmt::Debug for ObfString<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.decrypt(random!(usize) & 0xffff).fmt(f)
	}
}
impl<A: AsRef<[u8]> + AsMut<[u8]>> fmt::Display for ObfString<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.decrypt(random!(usize) & 0xffff).fmt(f)
	}
}
#[inline(never)]
unsafe fn decryptbuf(dest: *mut u8, dest_len: usize, src: usize) {
	let mut key = *((src + dest_len * XREF_SHIFT - 4) as *const u32);
	let data = slice::from_raw_parts((src + dest_len * XREF_SHIFT) as *const u8, dest_len);
	for i in 0..data.len() {
		key = next_round(key);
		*dest.add(i) = data[i].wrapping_add(key as u8);
	}
}
/// Obfuscated string buffer.
///
/// This type represents the string buffer after decryption on the stack.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct ObfBuffer<A>(A);
impl<A: AsRef<[u8]>> ObfBuffer<A> {
	#[inline]
	pub fn as_str(&self) -> &str {
		#[cfg(debug_assertions)]
		return str::from_utf8(self.0.as_ref()).unwrap();
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
impl<A: AsRef<[u8]>> ops::Deref for ObfBuffer<A> {
	type Target = str;
	#[inline]
	fn deref(&self) -> &str {
		self.as_str()
	}
}
impl<A: AsRef<[u8]>> AsRef<str> for ObfBuffer<A> {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}
impl<A: AsRef<[u8]>> fmt::Debug for ObfBuffer<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.as_str().fmt(f)
	}
}
impl<A: AsRef<[u8]>> fmt::Display for ObfBuffer<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.as_str().fmt(f)
	}
}

//----------------------------------------------------------------
// Widestr implementation

/// Obfuscated wide string constant data.
///
/// This type represents the data baked in the binary and holds the key and obfuscated wide string.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct WObfString<A> {
	key: u32,
	data: A,
}
impl<A> WObfString<A> {
	/// Constructor.
	pub const fn new(key: u32, data: A) -> WObfString<A> {
		WObfString { key, data }
	}
}
impl<A: AsRef<[u16]> + AsMut<[u16]>> WObfString<A> {
	/// Decrypts the obfuscated wide string and returns the buffer.
	///
	/// The `x` argument should be a compiletime random 16-bit value.
	/// It is used to obfuscate the underlying call to the decrypt routine.
	#[inline(always)]
	pub fn decrypt(&self, x: usize) -> WObfBuffer<A> {
		unsafe {
			let mut buffer = MaybeUninit::uninit();
			let data = self.data.as_ref();
			let src = data.as_ptr() as usize - data.len() * XREF_SHIFT;
			let f: unsafe fn(*mut u16, usize, usize) = mem::transmute(ptr::read_volatile(&(wdecryptbuf as usize + x)) - x);
			f(buffer.as_mut_ptr() as *mut u16, mem::size_of::<A>() / 2, src);
			buffer.assume_init()
		}
	}
}
impl<A: AsRef<[u16]> + AsMut<[u16]>> fmt::Debug for WObfString<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.decrypt(random!(usize) & 0xffff).fmt(f)
	}
}
impl<A: AsRef<[u16]> + AsMut<[u16]>> fmt::Display for WObfString<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.decrypt(random!(usize) & 0xffff).fmt(f)
	}
}
#[inline(never)]
unsafe fn wdecryptbuf(dest: *mut u16, dest_len: usize, src: usize) {
	let mut key = *((src + dest_len * XREF_SHIFT - 4) as *const u32);
	let data = slice::from_raw_parts((src + dest_len * XREF_SHIFT) as *const u16, dest_len);
	for i in 0..data.len() {
		key = next_round(key);
		*dest.add(i) = data[i].wrapping_add(key as u16);
	}
}
/// Obfuscated wide string buffer.
///
/// This type represents the wide string buffer after decryption on the stack.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct WObfBuffer<A>(A);
impl<A: AsRef<[u16]>> WObfBuffer<A> {
	#[inline]
	pub fn as_wide(&self) -> &[u16] {
		self.0.as_ref()
	}
}
impl<A: AsRef<[u16]>> ops::Deref for WObfBuffer<A> {
	type Target = [u16];
	#[inline]
	fn deref(&self) -> &[u16] {
		self.as_wide()
	}
}
impl<A: AsRef<[u16]>> AsRef<[u16]> for WObfBuffer<A> {
	fn as_ref(&self) -> &[u16] {
		self.as_wide()
	}
}
impl<A: AsRef<[u16]>> fmt::Debug for WObfBuffer<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use fmt::Write;
		f.write_str("\"")?;
		for chr in char::decode_utf16(self.as_wide().iter().cloned()) {
			f.write_char(chr.unwrap_or(char::REPLACEMENT_CHARACTER))?;
		}
		f.write_str("\"")
	}
}
impl<A: AsRef<[u16]>> fmt::Display for WObfBuffer<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use fmt::Write;
		for chr in char::decode_utf16(self.as_wide().iter().cloned()) {
			f.write_char(chr.unwrap_or(char::REPLACEMENT_CHARACTER))?;
		}
		Ok(())
	}
}
}
