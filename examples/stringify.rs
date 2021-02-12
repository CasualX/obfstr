/*!
This example demonstrates various ways to obfuscate the string literals in more complex scenarios.
The example presented here uses an enum because it can be challenging to return a string representation of it.
 */

use std::fmt;
use obfstr::{obfstr, position};

// Let's try to obfuscate the string representation of this enum.
pub enum Example {
	Foo,
	Bar,
	Baz,
}

impl Example {
	// Returns an owned String but this allocates memory.
	pub fn to_str1(&self) -> String {
		match self {
			Example::Foo => String::from(obfstr!("Foo")),
			Example::Bar => String::from(obfstr!("Bar")),
			Example::Baz => String::from(obfstr!("Baz")),
		}
	}

	// Use a callback to keep the string slice allocated on the stack but this gets annoying in more complex scenarios.
	pub fn to_str2<R, F: FnMut(&str) -> R>(&self, mut f: F) -> R {
		match self {
			Example::Foo => f(obfstr!("Foo")),
			Example::Bar => f(obfstr!("Bar")),
			Example::Baz => f(obfstr!("Baz"))
		}
	}

	// Use a buffer to hold the deobfuscated string. Panics if the buffer is too small.
	pub fn to_str3<'a>(&self, buf: &'a mut [u8; 4]) -> &'a str {
		match self {
			Example::Foo => obfstr!(buf <- "Foo"),
			Example::Bar => obfstr!(buf <- "Bar"),
			Example::Baz => obfstr!(buf <- "Baz"),
		}
	}

	// Allocate the string literals via concatenation.
	pub const POOL: &'static str = concat!("Foo", "Bar", "Baz");

	// Deobfuscate the POOL constant and pass it here as the pool argument.
	// This to string implementation will slice the right substring.
	pub fn to_str4<'a>(&self, pool: &'a str) -> &'a str {
		match self {
			Example::Foo => &pool[position!(Example::POOL, "Foo")],
			Example::Bar => &pool[position!(Example::POOL, "Bar")],
			Example::Baz => &pool[position!(Example::POOL, "Baz")],
		}
	}
}
impl fmt::Display for Example {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.to_str2(|s| f.write_str(s))
	}
}

fn main() {}
