use core::ptr;

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
