use obfstr::*;

// Ensures the macro machinery works outside the scope of the crate itself...
#[test]
fn main() {
	let (a, b) = (random!(u64), random!(u64));
	assert_ne!(a, b);

	assert_eq!(
		obfstr!("This literal is very very very long to see if it correctly handles long string"),
		        "This literal is very very very long to see if it correctly handles long string");

	assert_eq!(obfstr!("\u{20}\0"), " \0");
	assert_eq!(obfstr!("\"\n\t\\\'\""), "\"\n\t\\\'\"");

	assert_eq!(obfstr!(L"ABC"), &[b'A' as u16, b'B' as u16, b'C' as u16]);
	assert_eq!(obfstr!(L"🌍"), &[0xd83c, 0xdf0d]);

	assert!(obfeq!(wide!("ABC"), L"ABC"));
	assert!(obfeq!(wide!("🌍"), L"🌍"));
}
