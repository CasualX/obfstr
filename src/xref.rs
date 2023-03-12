use core::hint;

/// Obfuscates the xref to data reference.
///
/// ```
/// static FOO: i32 = 42;
/// let foo = obfstr::xref!(&FOO);
///
/// // When looking at the disassembly the reference to `FOO` has been obfuscated.
/// assert_eq!(foo as *const _, &FOO as *const _);
/// ```
///
/// This can be used for a more lightweight obfuscation that keeps that `'static` nature of string constants:
///
/// ```
/// assert_eq!(obfstr::xref!("Hello world!"), "Hello world!");
/// assert_eq!(obfstr::xref!(b"Byte array"), b"Byte array");
/// ```
#[macro_export]
macro_rules! xref {
	($e:expr) => {
		$crate::xref::xref::<_,
			{$crate::random!(u32, stringify!($e), "OFFSET")},
			{$crate::random!(u64, stringify!($e), "SEED")}>($e)
	};
}

#[inline(always)]
const fn non_zero(rand: u32) -> u32 {
	if rand == 0 { 1 } else { rand }
}

#[inline(always)]
const fn obfchoice(v: u32, seed: u64) -> u32 {
	let rand = (seed >> 32) as u32;
	match seed & 7 {
		0 => v.wrapping_add(rand),
		1 => rand.wrapping_sub(v),
		2 => v ^ rand,
		3 => v ^ v.rotate_left(non_zero(rand & 7)),
		4 => !v,
		5 => v ^ (v >> non_zero(rand & 31)),
		6 => v.wrapping_mul(non_zero(rand)),
		7 => v.wrapping_neg(),
		_ => unsafe { hint::unreachable_unchecked() }
	}
}

#[inline(always)]
const fn obfuscate<const SEED: u64>(mut v: u32) -> usize {
	let mut seed = SEED;
	use crate::splitmix;
	seed = splitmix(seed);
	v = obfchoice(v, seed);
	seed = splitmix(seed);
	v = obfchoice(v, seed);
	seed = splitmix(seed);
	v = obfchoice(v, seed);
	seed = splitmix(seed);
	v = obfchoice(v, seed);
	seed = splitmix(seed);
	v = obfchoice(v, seed);
	return (v & 0xffff) as usize
}

#[inline(never)]
fn inner<const SEED: u64>(p: *const u8, offset: u32) -> *const u8 {
	p.wrapping_add(obfuscate::<SEED>(offset))
}

/// Obfuscates the xref to data reference.
#[inline(always)]
pub fn xref<T: ?Sized, const OFFSET: u32, const SEED: u64>(p: &'static T) -> &'static T {
	unsafe {
		let mut p: *const T = p;
		// Launder the values through black_box to prevent LLVM from optimizing away the obfuscation
		let val = inner::<SEED>(hint::black_box((p as *const u8).wrapping_sub(obfuscate::<SEED>(OFFSET))), hint::black_box(OFFSET));
		// set_ptr_value
		*(&mut p as *mut *const T as *mut *const u8) = val;
		&*p
	}
}

/// Obfuscates the xref to data reference.
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
	($e:expr) => {
		$crate::xref::xref_mut::<_,
			{$crate::random!(u32, stringify!($e), "OFFSET")},
			{$crate::random!(u64, stringify!($e), "SEED")}>($e)
	};
}

#[inline(never)]
fn inner_mut<const SEED: u64>(p: *mut u8, offset: u32) -> *mut u8 {
	p.wrapping_add(obfuscate::<SEED>(offset))
}

/// Obfuscates the xref to data reference.
#[inline(always)]
pub fn xref_mut<T: ?Sized, const OFFSET: u32, const SEED: u64>(p: &'static mut T) -> &'static mut T {
	unsafe {
		let mut p: *mut T = p;
		// Launder the values through black_box to prevent LLVM from optimizing away the obfuscation
		let val = inner_mut::<SEED>(hint::black_box((p as *mut u8).wrapping_sub(obfuscate::<SEED>(OFFSET))), hint::black_box(OFFSET));
		// set_ptr_value
		*(&mut p as *mut *mut T as *mut *mut u8) = val;
		&mut *p
	}
}

#[test]
fn test_xref_slice() {
	static FOO: [i32; 42] = [13; 42];
	let foo = xref!(&FOO[..]);
	assert_eq!(foo as *const _, &FOO as *const _);
	assert_eq!(xref!("Hello world!"), "Hello world!");
	assert_eq!(xref!(b"Byte array"), b"Byte array");
}

#[test]
fn regression1() {
	// Caused by `v = v ^ (v >> RNG)` when RNG is zero to always be zero
	let v = obfuscate::<0xC4B3B4F3D986EFBEu64>(0x3C236765u32);
	assert_ne!(v, 0);
}
