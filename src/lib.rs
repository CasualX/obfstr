/*!
String Obfuscation
==================

Enter any text and copy the result in your code.

<div>
<textarea id="input" rows="1" cols="80" style="resize:vertical;">Hello 🌍</textarea>
<input id="wide" type="checkbox">
<pre id="output" style="word-wrap:break-word;"></pre>
<script>
function utf8_encode(s) {
  return unescape(encodeURIComponent(s));
}
function next_round(x) {
  x = (x ^ (x << 13) >>> 0) >>> 0;
  x = (x ^ (x >>> 17) >>> 0) >>> 0;
  x = (x ^ (x << 5) >>> 0) >>> 0;
  return x;
};
function obfstr(text, key, wide) {
  let result = "obfstr::obfstr!(" + (wide ? "L" : "") + "/*" + text.replace(/(?:\r|\n|\t)/g, "") + "*/ " + key;
  let s = wide ? text : utf8_encode(text);
  let mask = wide ? 0xffff : 0xff;
  for (let i = 0; i < s.length; ++i) {
    key = next_round(key);
    let x = ((s.charCodeAt(i) & mask) - (key & mask)) & mask;
    result += "," + x;
  }
  return result + ")";
}
function oninput() {
  let key = crypto.getRandomValues(new Uint32Array(1))[0];
  let text = document.getElementById('input').value;
  let wide = document.getElementById('wide').checked;
  let result = obfstr(text, key, wide);
  document.getElementById('output').textContent = result;
}
document.getElementById('input').addEventListener('input', oninput);
document.getElementById('wide').addEventListener('change', oninput);
oninput();
</script>
</div>

Paste the generated code in your source.

```
let s = obfstr::obfstr!(/*Hello 🌍*/ 2803150042,11,63,105,38,140,200,70,29,83,200);
assert_eq!(s.as_str(), "Hello 🌍");
```
!*/

#![no_std]
#![feature(fixed_size_array)]

use core::{fmt, mem, ops, slice, str};
use core::array::FixedSizeArray;

/// Pretty syntax.
#[macro_export]
macro_rules! obfstr {
	($key:literal $(,$byte:literal)*) => {
		(&$crate::StrDesc { key: $key, data: [$($byte),*] }).decrypt()
	};
	(L $key:literal $(,$word:literal)*) => {
		(&$crate::WStrDesc { key: $key, data: [$($word),*] }).decrypt()
	};
}

fn next_round(mut x: u32) -> u32 {
	x ^= x << 13;
	x ^= x >> 17;
	x ^= x << 5;
	x
}

//----------------------------------------------------------------
// String implementation

#[repr(C)]
pub struct StrDesc<A> {
	pub key: u32,
	pub data: A,
}
impl<A: FixedSizeArray<u8>> StrDesc<A> {
	#[inline(always)]
	pub fn decrypt(&self) -> StrBuf<A> {
		unsafe {
			let mut buffer = StrBuf::<A>::uninit();
			let data = self.data.as_slice();
			let src = data.as_ptr() as usize - data.len() * 33;
			decryptbuf(buffer.0.as_mut_slice(), src);
			buffer
		}
	}
}
#[inline(never)]
unsafe fn decryptbuf(dest: &mut [u8], src: usize) {
	let mut key = *((src + dest.len() * 33 - 4) as *const u32);
	let data = slice::from_raw_parts((src + dest.len() * 33) as *const u8, dest.len());
	for i in 0..data.len() {
		key = next_round(key);
		dest[i] = data[i].wrapping_add(key as u8);
	}
}
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct StrBuf<A>(A);
impl<A: FixedSizeArray<u8>> StrBuf<A> {
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
	#[inline]
	pub fn unsafe_as_static_str(&self) -> &'static str {
		unsafe { &*(self.as_str() as *const _) }
	}
}
impl<A: FixedSizeArray<u8>> ops::Deref for StrBuf<A> {
	type Target = str;
	#[inline]
	fn deref(&self) -> &str {
		self.as_str()
	}
}
impl<A: FixedSizeArray<u8>> fmt::Debug for StrBuf<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.as_str().fmt(f)
	}
}
impl<A: FixedSizeArray<u8>> fmt::Display for StrBuf<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.as_str().fmt(f)
	}
}

//----------------------------------------------------------------
// Widestr implementation

#[repr(C)]
pub struct WStrDesc<A> {
	pub key: u32,
	pub data: A,
}
impl<A: FixedSizeArray<u16>> WStrDesc<A> {
	#[inline(always)]
	pub fn decrypt(&self) -> WStrBuf<A> {
		unsafe {
			let mut buffer = WStrBuf::<A>::uninit();
			let data = self.data.as_slice();
			let src = data.as_ptr() as usize - data.len() * 33;
			wdecryptbuf(buffer.0.as_mut_slice(), src);
			buffer
		}
	}
}
#[inline(never)]
unsafe fn wdecryptbuf(dest: &mut [u16], src: usize) {
	let mut key = *((src + dest.len() * 33 - 4) as *const u32);
	let data = slice::from_raw_parts((src + dest.len() * 33) as *const u16, dest.len());
	for i in 0..data.len() {
		key = next_round(key);
		dest[i] = data[i].wrapping_add(key as u16);
	}
}
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct WStrBuf<A>(A);
impl<A: FixedSizeArray<u16>> WStrBuf<A> {
	unsafe fn uninit() -> Self {
		mem::uninitialized()
	}
	#[inline]
	pub fn as_wide(&self) -> &[u16] {
		self.0.as_slice()
	}
}
impl<A: FixedSizeArray<u16>> ops::Deref for WStrBuf<A> {
	type Target = [u16];
	#[inline]
	fn deref(&self) -> &[u16] {
		self.as_wide()
	}
}
impl<A: FixedSizeArray<u16>> fmt::Debug for WStrBuf<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.as_wide().fmt(f)
	}
}

//----------------------------------------------------------------

#[test]
fn foo() {
	let s = obfstr!(/*abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz*/3154653501,247,
	107,175,19,79,255,115,150,24,33,255,94,48,145,245,128,60,147,110,111,59,51,101,
	56,75,206,48,97,40,136,132,234,108,129,58,112,55,159,187,140,146,140,204,123,93,
	22,35,25,154,193,20,76);
	assert_eq!(s.as_str(), "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz");
}
