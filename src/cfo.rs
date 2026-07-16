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
///
/// An explicit constant `u32` seed can be provided when otherwise-identical macro invocations
/// need different control flow. The seed is mixed with the stringified statements:
///
/// ```
/// let mut tmp = 0;
/// obfstr::obfstmt! {
/// 	@seed obfstr::random!(u32, "example");
/// 	tmp = 2;
/// 	tmp *= 5;
/// }
/// assert_eq!(tmp, 10);
/// ```
#[macro_export]
macro_rules! obfstmt {
	(@seed $seed:expr; $($stmt:stmt;)*) => {{
		// Count the number of statements
		const _OBFSTMT_LEN: usize = <[&'static str]>::len(&[$(stringify!($stmt)),*]);
		// Generate one key for every statement and one final exit key
		const _OBFSTMT_KEY_LEN: usize = _OBFSTMT_LEN + 1;
		// Seed might be a generic const (from xref obfuscate) which is not allowed in inner const item...
		let _obfstmt_keys: [u32; _OBFSTMT_KEY_LEN] = const {
			$crate::cfo::generate::<_OBFSTMT_KEY_LEN>($seed, &[$(stringify!($stmt)),*])
		};
		// Initialize the key value
		let mut key = _obfstmt_keys[0];
		#[allow(unused_mut)]
		let mut xor = 0u32;
		loop {
			#[allow(unused_mut)]
			let mut index = 0usize;
			match key {
				$(
					// Guards are evaluated in source order, so each arm consumes one key
					key if key == {
						let expected = _obfstmt_keys[index];
						index += 1;
						expected
					} => {
						$stmt
						xor = _obfstmt_keys[index - 1] ^ _obfstmt_keys[index];
					},
				)*
				key if key == { _obfstmt_keys[index] } => break,
				_ => (),
			}
			key ^= xor;
		}
	}};
	($($stmt:stmt;)*) => {{
		$crate::obfstmt!(@seed $crate::entropy("obfstmt") as u32; $($stmt;)*)
	}};
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

#[test]
fn test_generate_seed_and_statements_change_keys() {
	let stmts = &["i += 1", "i *= 2", "i -= 3"];
	let lhs = generate::<4>(0x12345678, stmts);
	let rhs = generate::<4>(0x87654321, stmts);
	let different_stmts = generate::<4>(0x12345678, &["i += 1", "i *= 2", "i -= 4"]);
	assert_ne!(lhs, rhs);
	assert_ne!(lhs, different_stmts);
}
