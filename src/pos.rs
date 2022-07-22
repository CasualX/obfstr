use core::{ops, str};

/// Finds the position of the needle in the haystack at compiletime.
///
/// Produces a const-eval error if the needle is not a substring of the haystack.
///
/// # Examples
///
/// ```
/// assert_eq!(obfstr::position!("haystack", "st"), 3..5);
///# assert_eq!(obfstr::position!("haystack", "haystack"), 0..8);
///# assert_eq!(obfstr::position!("haystack", "ck"), 6..8);
/// ```
///
/// Use this API when pooling strings in a single obfstr:
///
/// ```
/// const POOL: &str = concat!("Foo", "Bar", "Baz");
///
/// obfstr::obfstr! { let pool = POOL; }
///
/// // Later, read strings from the pool
/// let foo = &pool[obfstr::position!(POOL, "Foo")];
/// let bar = &pool[obfstr::position!(POOL, "Bar")];
/// let baz = &pool[obfstr::position!(POOL, "Baz")];
/// ```
#[macro_export]
macro_rules! position {
	($haystack:expr, $needle:expr) => {{ const _POSITION_RANGE: ::core::ops::Range<usize> = $crate::position($haystack, $needle); _POSITION_RANGE }};
}

/// Finds the position of the needle in the haystack at compiletime.
///
/// Produces a const-eval error if the needle is not a substring of the haystack.
///
/// ```
/// const POSITION: std::ops::Range<usize> = obfstr::position("haystack", "st");
/// assert_eq!(POSITION, 3..5);
/// ```
#[doc(hidden)]
#[inline(always)]
pub const fn position(haystack: &str, needle: &str) -> ops::Range<usize> {
	let start = search(haystack, needle);
	// Panic if substring not found
	if start < 0 {
		panic!("Needle not found in the haystack");
	}
	let start = start as usize;
	start..start + needle.len()
}

const fn search(haystack: &str, needle: &str) -> isize {
	// Short-circuit empty needles
	if needle.len() == 0 {
		return 0;
	}

	let haystack = haystack.as_bytes();
	let needle = needle.as_bytes();

	// Avoid overflow checks later
	if needle.len() <= haystack.len() {
		// Special case for needle length of 1
		if needle.len() == 1 {
			let needle = needle[0];
			let mut offset = 0;
			while offset <= haystack.len() {
				if haystack[offset] == needle {
					return offset as isize;
				}
				offset += 1;
			}
		}
		// Full blown quicksearch
		else {
			// assumed:
			// needle.len() >= 2
			// needle.len() <= haystack.len()

			// Initialize the jump table
			let mut jumps = [max(needle.len()); 256];
			let tail = needle.len() - 1;
			let mut i = 0;
			while i < tail {
				jumps[needle[i] as usize] = max(tail - i);
				i += 1;
			}
			// Find the needle
			let sentinel = needle[tail];
			let mut offset = 0;
			while offset < haystack.len() - tail {
				let chr = haystack[offset + tail];
				if chr == sentinel && check(haystack, needle, offset) {
					return offset as isize;
				}
				offset += jumps[chr as usize] as usize;
			}
		}
	}
	return -1;
}


#[inline(always)]
const fn check(haystack: &[u8], needle: &[u8], offset: usize) -> bool {
	let mut i = 0;
	while i < needle.len() {
		if haystack[offset + i] != needle[i] {
			return false;
		}
		i += 1;
	}
	return true;
}
#[inline(always)]
const fn max(a: usize) -> u8 {
	if a > 255 { 255 } else { a as u8 }
}

#[test]
fn test_position() {
	assert_eq!(position("ABCBC", "CBC"), 2..5);
	assert_eq!(position("ABCBC", "ABCBC"), 0..5);
}

#[test]
#[should_panic]
fn test_position_needle_longer_than_haystack() {
	let _ = position("haystack", "needleneedleneedle");
}
