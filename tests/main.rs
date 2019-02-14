// Ensures the macro machinery works outside the scope of the crate itself...
#[test]
fn main() {
	assert_eq!(obfstr::obfstr!("Hello world"), "Hello world");

	assert_eq!(obfstr::obfstr!("This literal is very very very long to see if it correctly handles long string"),
	                           "This literal is very very very long to see if it correctly handles long string");

	// assert_eq!(obfstr::obfstr!("\0"), "\0");
	assert_eq!(obfstr::obfstr!("\"\n\t\\\'\""), "\"\n\t\\\'\"");

	assert_eq!(obfstr::obfstr!(L"ABC"), &[b'A' as u16, b'B' as u16, b'C' as u16]);
}
