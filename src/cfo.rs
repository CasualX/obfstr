/*!
Control Flow Obfuscation
========================
*/

#[inline(always)]
const fn mix32(mut value: u32) -> u32 {
	value ^= value >> 16;
	value = value.wrapping_mul(0x7feb352d);
	value ^= value >> 15;
	value = value.wrapping_mul(0x846ca68b);
	value ^= value >> 16;
	return value;
}

/// Generates a collision-free key table for a sequence of statements.
///
/// The returned table contains one key per statement plus the final exit key.
/// The odd-increment sequence is full-period over `u32`, and the mixer is
/// bijective, so keys do not repeat until the 32-bit period wraps.
#[inline(always)]
pub const fn generate<const LEN: usize>(seed: u32, stmts: &[&'static str]) -> [u32; LEN] {
	if LEN == 0 || LEN - 1 != stmts.len() {
		panic!("invalid control flow key table length");
	}

	let mut digest = seed ^ (LEN as u32).wrapping_mul(0x9e3779b9);
	let mut i = 0;
	while i < stmts.len() {
		digest = crate::murmur3(stmts[i].as_bytes(), digest ^ (i as u32).wrapping_mul(0x85ebca6b));
		digest ^= (stmts[i].len() as u32).rotate_left((i & 31) as u32);
		i += 1;
	}

	let increment = mix32(digest ^ 0x9e3779b9) | 1;
	let mut state = mix32(seed ^ digest ^ 0x85ebca6b);

	let mut keys = [0; LEN];
	let mut i = 0;
	while i < LEN {
		state = state.wrapping_add(increment);
		keys[i] = mix32(state ^ digest);
		i += 1;
	}
	return keys;
}

/// Statement control flow obfuscation.
///
/// Given a sequence of statements obfuscates the relationship between each statement.
///
/// # Limitations
///
/// Variables cannot be declared inside the obfuscated statements, declare and initialize any variables needed beforehand.
/// Control flow analysis will fail. The declared variables will need to be mutable and have an initial value.
///
/// # Examples
///
/// ```
/// let mut tmp = 0;
/// obfstr::obfstmt! {
/// 	tmp = 2;
/// 	tmp *= 22;
/// 	tmp -= 12;
/// 	tmp /= 3;
/// }
///# obfstr::obfstmt! {}
/// assert_eq!(tmp, 10);
/// ```
#[macro_export]
macro_rules! obfstmt {
	($($stmt:stmt;)*) => {{
		// Initial seed value
		const _OBFSTMT_SEED: u32 = $crate::random!(u32, stringify!($($stmt;)*));
		// Count the number of statements
		const _OBFSTMT_LEN: usize = <[&'static str]>::len(&[$(stringify!($stmt)),*]);
		// Generate one key for every statement and one final exit key
		const _OBFSTMT_KEY_LEN: usize = _OBFSTMT_LEN + 1;
		const _OBFSTMT_KEYS: [u32; _OBFSTMT_KEY_LEN] =
			$crate::cfo::generate::<_OBFSTMT_KEY_LEN>(_OBFSTMT_SEED, &[$(stringify!($stmt)),*]);
		// Initialize the key value
		let mut key = _OBFSTMT_KEYS[0];
		#[allow(unused_mut)]
		let mut xor = 0u32;
		loop {
			$crate::__obfstmt_match!(key, xor, 0usize, [$($stmt;)*], []);
			key ^= xor;
		}
	}};
}

/// Generates the match statement for [`obfstmt!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __obfstmt_match {
	// Terminating case, generate the code
	($key:expr, $xor:expr, $x:expr, [], [$($i:expr, $stmt:stmt;)*]) => {
		match $key {
			// Have to use match guard here because an expression isn't allowed in pattern position
			// The result is still optimized to a binary search for the right key per block
			$(
				key if key == { _OBFSTMT_KEYS[$i] } => {
					$stmt
					$xor = _OBFSTMT_KEYS[$i] ^ _OBFSTMT_KEYS[$i + 1usize];
				},
			)*
			key if key == { _OBFSTMT_KEYS[_OBFSTMT_LEN] } => break,
			_ => (),
		}
	};
	// Generate increasing indices for every stmt
	($key:expr, $xor:expr, $x:expr, [$stmt1:stmt; $($tail:stmt;)*], [$($i:expr, $stmt2:stmt;)*]) => {
		$crate::__obfstmt_match!($key, $xor, $x + 1usize, [$($tail;)*], [$($i, $stmt2;)* $x, $stmt1; ])
	};
}

#[test]
fn test_identical_stmt() {
	let mut i: u8 = 0;
	obfstmt! {
		i += 1;
		i += 1;
		i += 1;
		i += 1;
	}
	obfstmt! {}
	assert_eq!(i, 4);
}

#[test]
fn test_generate_known_collision_is_unique() {
	const STMT_LEN: usize = 34289;
	const KEY_LEN: usize = STMT_LEN + 1;

	let keys = generate::<KEY_LEN>(0x12345678, &["i += 1"; STMT_LEN]);
	assert_ne!(keys[9681], keys[34288]);
	assert_ne!(keys[34288], keys[KEY_LEN - 1]);
}

#[test]
fn test_generate_unique_keys() {
	const STMT_LEN: usize = 512;
	const KEY_LEN: usize = STMT_LEN + 1;

	let keys = generate::<KEY_LEN>(0x12345678, &["i += 1"; STMT_LEN]);
	let mut i = 0;
	while i < KEY_LEN {
		let mut j = i + 1;
		while j < KEY_LEN {
			assert_ne!(keys[i], keys[j]);
			j += 1;
		}
		i += 1;
	}
}
