/*!
Compiletime string literal obfuscation.
!*/

#![no_std]

// Reexport these because reasons...
#[doc(hidden)]
pub use obfstr_impl::*;

#[cfg(feature = "rand")]
pub use cfgd::*;
#[cfg(feature = "rand")]
mod cfgd {

use core::{char, fmt, mem, ops, ptr, slice, str};

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
		const S: $crate::ObfString<[u8; $crate::_strlen_!($string)]> = {
			let (key, data) = $crate::_obfstr_!($string);
			let _: &[u8] = &data;
			unsafe {
				// Safety: The _obfstr_ macro will always return a byte array in
				// data.
				$crate::ObfString::new(key, data)
			}
		};
		S.decrypt($crate::random!(usize) & 0xffff).as_str()
	}};
	(L$string:literal) => {{
		const S: $crate::WObfString<[u16; $crate::_strlen_!($string)]> = {
			let (key, data) = $crate::_obfstr_!(L$string);
			let _: &[u16] = &data;
			unsafe {
				// Safety: The _obfstr_ macro will always return a u16 array in
				// data.
				$crate::WObfString::new(key, data)
			}
		};
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
		const S: $crate::ObfString<[u8; $crate::_strlen_!($string)]> = {
			let (key, data) = $crate::_obfstr_!($string);
			let _: &[u8] = &data;
			unsafe {
				// Safety: The _obfstr_ macro will always return a byte array in
				// data.
				$crate::ObfString::new(key, data)
			}
		};
		S.decrypt($crate::random!(usize) & 0xffff)
	}};
	(L$string:literal) => {{
		const S: $crate::WObfString<[u16; $crate::_strlen_!($string)]> = {
			let (key, data) = $crate::_obfstr_!(L$string);
			let _: &[u16] = &data;
			$crate::WObfString::new(key, data)
		};
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
		const S: $crate::ObfString<[u8; $crate::_strlen_!($string)]> = {
			let (key, data) = $crate::_obfstr_!($string);
			let _: &[u8] = &data;
			unsafe {
				// Safety: The _obfstr_ macro will always return a byte array in
				// data.
				$crate::ObfString::new(key, data)
			}
		};S
	}};
	(L$string:literal) => {{
		const S: $crate::WObfString<[u16; $crate::_strlen_!($string)]> = {
			let (key, data) = $crate::_obfstr_!(L$string);
			let _: &[u16] = &data;
			unsafe {
				// Safety: The _obfstr_ macro will always return a u16 array in
				// data.
				$crate::WObfString::new(key, data)
			}
		};S
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
		const S: $crate::ObfString<[u8; $crate::_strlen_!($string)]> = {
			let (key, data) = $crate::_obfstr_!($string);
			let _: &[u8] = &data;
			$crate::ObfString::new(key, data)
		};
		S.decrypt($crate::random!(usize) & 0xffff).unsafe_as_static_str()
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
		const N: $ty = $crate::_random_!($ty); N
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
	///
	/// # Safety
	///
	/// Must always be called with a byte array, e.g. an array of type [u8; N].
	#[doc(hidden)]
	pub const unsafe fn new(key: u32, data: A) -> ObfString<A> {
		ObfString { key, data }
	}
}
impl<A> ObfString<A> {
	/// Decrypts the obfuscated string and returns the buffer.
	///
	/// The `x` argument should be a compiletime random 16-bit value.
	/// It is used to obfuscate the underlying call to the decrypt routine.
	#[inline(always)]
	pub fn decrypt(&self, x: usize) -> ObfBuffer<A> {
		unsafe {
			let mut buffer = ObfBuffer::<A>::uninit();
			let data = {
				// Safety: ObfString::new guarantees that the input type is a
				// byte array.
				slice::from_raw_parts(&self.data as *const A as *const u8, mem::size_of::<A>())
			};
			let src = data.as_ptr() as usize - data.len() * XREF_SHIFT;
			let f: unsafe fn(&mut [u8], usize) = mem::transmute(ptr::read_volatile(&(decryptbuf as usize + x)) - x);
			f(buffer.as_mut_slice(), src);
			buffer
		}
	}
}
impl<A> fmt::Debug for ObfString<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.decrypt(random!(usize) & 0xffff).fmt(f)
	}
}
impl<A> fmt::Display for ObfString<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.decrypt(random!(usize) & 0xffff).fmt(f)
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
impl<A> ObfBuffer<A> {
	#[allow(deprecated)]
	unsafe fn uninit() -> Self {
		mem::uninitialized()
	}
	#[inline]
	pub fn as_str(&self) -> &str {
		let slice = unsafe {
			// Safety: ObfBuffer can only be created from ObfString::decrypt,
			// which guarantees that A is a byte array.
			slice::from_raw_parts(&self.0 as *const A as *const u8, mem::size_of::<A>())
		};
		#[cfg(debug_assertions)]
		return str::from_utf8(slice).unwrap();
		#[cfg(not(debug_assertions))]
		return unsafe { str::from_utf8_unchecked(slice) };
	}
	fn as_mut_slice(&mut self) -> &mut [u8] {
		unsafe {
			// Safety: ObfBuffer can only be created from ObfString::decrypt,
			// which guarantees that A is a byte array.
			slice::from_raw_parts_mut(&mut self.0 as *mut A as *mut u8, mem::size_of::<A>())
		}
	}
	// For use with serde's stupid 'static limitations...
	#[cfg(feature = "unsafe_static_str")]
	#[inline]
	pub fn unsafe_as_static_str(&self) -> &'static str {
		unsafe { &*(self.as_str() as *const _) }
	}
}
impl<A> ops::Deref for ObfBuffer<A> {
	type Target = str;
	#[inline]
	fn deref(&self) -> &str {
		self.as_str()
	}
}
impl<A> AsRef<str> for ObfBuffer<A> {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}
impl<A> fmt::Debug for ObfBuffer<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.as_str().fmt(f)
	}
}
impl<A> fmt::Display for ObfBuffer<A> {
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
	///
	/// # Safety
	///
	/// Must always be called with a u16 array, e.g. an array of type [u16; N].
	#[doc(hidden)]
	pub const unsafe fn new(key: u32, data: A) -> WObfString<A> {
		WObfString { key, data }
	}
}
impl<A> WObfString<A> {
	/// Decrypts the obfuscated wide string and returns the buffer.
	///
	/// The `x` argument should be a compiletime random 16-bit value.
	/// It is used to obfuscate the underlying call to the decrypt routine.
	#[inline(always)]
	pub fn decrypt(&self, x: usize) -> WObfBuffer<A> {
		unsafe {
			let mut buffer = WObfBuffer::<A>::uninit();

			let data = {
				// Safety: WObfString::new guarantees that the input type is a
				// u16 array.
				slice::from_raw_parts(&self.data as *const A as *const u16, mem::size_of::<A>() / 2)
			};
			let src = data.as_ptr() as usize - data.len() * XREF_SHIFT;
			let f: unsafe fn(&mut [u16], usize) = mem::transmute(ptr::read_volatile(&(wdecryptbuf as usize + x)) - x);
			f(buffer.as_mut_slice(), src);
			buffer
		}
	}
}
impl<A> fmt::Debug for WObfString<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.decrypt(random!(usize) & 0xffff).fmt(f)
	}
}
impl<A> fmt::Display for WObfString<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.decrypt(random!(usize) & 0xffff).fmt(f)
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
impl<A> WObfBuffer<A> {
	#[allow(deprecated)]
	unsafe fn uninit() -> Self {
		mem::uninitialized()
	}
	#[inline]
	pub fn as_wide(&self) -> &[u16] {
		unsafe {
			// Safety: ObfBuffer can only be created from ObfString::decrypt,
			// which guarantees that A is a u16 array.
			slice::from_raw_parts(&self.0 as *const A as *const u16, mem::size_of::<A>() / 2)
		}
	}
	fn as_mut_slice(&mut self) -> &mut [u16] {
		unsafe {
			// Safety: ObfBuffer can only be created from WObfString::decrypt,
			// which guarantees that A is a u16 array.
			slice::from_raw_parts_mut(&mut self.0 as *mut A as *mut u16, mem::size_of::<A>() / 2)
		}
	}
}
impl<A> ops::Deref for WObfBuffer<A> {
	type Target = [u16];
	#[inline]
	fn deref(&self) -> &[u16] {
		self.as_wide()
	}
}
impl<A> AsRef<[u16]> for WObfBuffer<A> {
	fn as_ref(&self) -> &[u16] {
		self.as_wide()
	}
}
impl<A> fmt::Debug for WObfBuffer<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use fmt::Write;
		f.write_str("\"")?;
		for chr in char::decode_utf16(self.as_wide().iter().cloned()) {
			f.write_char(chr.unwrap_or(char::REPLACEMENT_CHARACTER))?;
		}
		f.write_str("\"")
	}
}
impl<A> fmt::Display for WObfBuffer<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use fmt::Write;
		for chr in char::decode_utf16(self.as_wide().iter().cloned()) {
			f.write_char(chr.unwrap_or(char::REPLACEMENT_CHARACTER))?;
		}
		Ok(())
	}
}
}

/// Wide string literal, returns an array of words.
///
/// The type of the returned literal is `&'static [u16; LEN]`.
///
/// ```
/// let expected = &['W' as u16, 'i' as u16, 'd' as u16, 'e' as u16];
/// assert_eq!(obfstr::wide!("Wide"), expected);
/// ```
#[macro_export]
macro_rules! wide {
	($s:literal) => {{
		const W: &[u16] = $crate::_wide_!($s); W
	}};
}
