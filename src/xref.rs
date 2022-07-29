use core::{hint, ptr};

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
	($e:expr) => {
		$crate::__xref!($crate::__entropy!(stringify!($e), 1) as usize, $crate::__entropy!(stringify!($e), 2), $e)
	};
}
#[doc(hidden)]
#[macro_export]
macro_rules! __xref {
	($offset:expr, $seed:expr, $e:expr) => {{
		const _XREF_OFFSET: usize = $offset;
		static mut _XREF_STATIC_MUT_OFFSET: usize = _XREF_OFFSET;
		$crate::xref::xref::<_, _XREF_OFFSET, {$seed}>($e, unsafe { &mut _XREF_STATIC_MUT_OFFSET })
	}};
}

#[inline(always)]
const fn non_zero(rand: usize) -> usize {
	if rand == 0 { 1 } else { rand }
}

#[inline(always)]
const fn obfchoice(v: usize, seed: u64) -> usize {
	let rand = (seed >> 32) as i32 as usize;
	match seed & 7 {
		0 => v.wrapping_add(rand),
		1 => rand.wrapping_sub(v),
		2 => v ^ rand,
		3 => v.rotate_left(non_zero(rand & 7) as u32),
		4 => !v,
		5 => v ^ (v >> non_zero(rand & 31)),
		6 => v.wrapping_mul(non_zero(rand)),
		7 => v.wrapping_neg(),
		_ => unsafe { hint::unreachable_unchecked() }
	}
}
#[inline(always)]
const fn obfuscate(mut v: usize, mut seed: u64) -> usize {
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
	return obfchoice(v, seed & 0xffffffff00000000 | 3) & 0xffff
}

/// Obfuscates the xref to static data.
#[inline(always)]
pub fn xref<T: ?Sized, const OFFSET: usize, const SEED: u64>(p: &'static T, offset: &'static usize) -> &'static T {
	unsafe {
		let mut p: *const T = p;
		// To avoid LLMV optimizing away the obfuscation, launder it through read_volatile
		let val = ptr::read_volatile(&(p as *const u8).wrapping_sub(obfuscate(OFFSET, SEED))).wrapping_add(obfuscate(ptr::read_volatile(offset), SEED));
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
	($e:expr) => { $crate::__xref_mut!($crate::__entropy!(stringify!($e), 1) as usize, $crate::__entropy!(stringify!($e), 2), $e) };
}
#[doc(hidden)]
#[macro_export]
macro_rules! __xref_mut {
	($offset:expr, $seed:expr, $e:expr) => {{
		const _XREF_OFFSET: usize = $offset;
		static mut _XREF_STATIC_MUT_OFFSET: usize = _XREF_OFFSET;
		$crate::xref::xref_mut::<_, _XREF_OFFSET, {$seed}>($e, unsafe { &mut _XREF_STATIC_MUT_OFFSET })
	}};
}

/// Obfuscates the xref to static data.
#[inline(always)]
pub fn xref_mut<T: ?Sized, const OFFSET: usize, const SEED: u64>(p: &'static mut T, offset: &'static usize) -> &'static mut T {
	unsafe {
		let mut p: *mut T = p;
		// To avoid LLMV optimizing away the obfuscation, launder it through read_volatile
		let val = ptr::read_volatile(&(p as *mut u8).wrapping_sub(obfuscate(OFFSET, SEED))).wrapping_add(obfuscate(ptr::read_volatile(offset), SEED));
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
}

#[test]
fn regression1() {
	// Caused by `v = v ^ (v >> RNG)` when RNG is zero to always be zero
	let v = obfuscate(4898264233338431333usize, (-4272872662024917058i64) as u64);
	assert_ne!(v, 0);
}
