extern crate proc_macro;

use proc_macro::*;
use proc_macro_hack::proc_macro_hack;

#[proc_macro_hack]
pub fn obfstr_impl(input: TokenStream) -> TokenStream {
	let mut tt = input.into_iter();
	let mut token = tt.next();

	// Optional L ident prefix to indicate wide strings
	let mut wide = false;
	if let Some(TokenTree::Ident(ident)) = &token {
		if ident.to_string() == "L" {
			wide = true;
			token = tt.next();
		}
	}

	// Followed by a string literal
	let string = match token {
		Some(TokenTree::Literal(lit)) => obfstr_parse(lit),
		Some(tt) => panic!("expected a string literal: `{}`", tt),
		None => panic!("expected a string literal"),
	};

	// End of macro arguments
	token = tt.next();
	if let Some(tt) = token {
		panic!("unexpected token: `{}`", tt);
	}

	// Generate a random key
	let key = rand::random::<u32>();
	let result = if wide {
		let mut words = {string}.encode_utf16().collect::<Vec<u16>>();
		wencrypt(&mut words, key)
	}
	else {
		let mut bytes = string.into_bytes();
		encrypt(&mut bytes, key)
	};

	result.parse().unwrap()
}

fn obfstr_parse(input: Literal) -> String {
	let string = input.to_string();
	let mut bytes = string.as_bytes();

	// Trim the string from its outer quotes
	if bytes.len() < 2 || bytes[0] != b'"' || bytes[bytes.len() - 1] != b'"' {
		panic!("expected a string literal: `{}`", input);
	}
	bytes = &bytes[1..bytes.len() - 1];
	let string: &str = unsafe { &*(bytes as *const _ as *const str) };

	// Parse escape sequences
	let mut unescaped = String::new();
	let mut chars = string.chars();
	while let Some(mut chr) = chars.next() {
		if chr == '\\' {
			chr = match chars.next() {
				Some('t') => '\t',
				Some('n') => '\n',
				Some('r') => '\r',
				Some('0') => '\0',
				Some('\\') => '\\',
				Some('\'') => '\'',
				Some('\"') => '\"',
				Some('u') => {
					match chars.next() {
						Some('{') => (),
						Some(chr) => panic!("invalid unicode escape character: `{}`", chr),
						None => panic!("invalid unicode escape at end of string"),
					}
					let mut u = 0;
					loop {
						match chars.next() {
							Some(chr @ '0'...'9') => {
								u = u * 16 + (chr as u32 - '0' as u32);
							},
							Some(chr @ 'a'...'f') => {
								u = u * 16 + (chr as u32 - 'a' as u32) + 10;
							},
							Some(chr @ 'A'...'F') => {
								u = u * 16 + (chr as u32 - 'A' as u32) + 10;
							},
							Some('}') => break,
							Some(chr) => panic!("invalid unicode escape character: `{}`", chr),
							None => panic!("invalid unicode escape at end of string"),
						};
					}
					match std::char::from_u32(u) {
						Some(chr) => chr,
						None => panic!("invalid unicode escape character: `\\u{{{}}}`", u),
					}
				},
				Some(chr) => panic!("invalid escape character: `{}`", chr),
				None => panic!("invalid escape at end of string"),
			}
		}
		unescaped.push(chr);
	}
	unescaped
}

fn next_round(mut x: u32) -> u32 {
	x ^= x << 13;
	x ^= x >> 17;
	x ^= x << 5;
	x
}

fn encrypt(bytes: &mut [u8], mut key: u32) -> String {
	let mut result = format!("$crate::ObfString {{ key: {}, data: [", key);
	for byte in bytes.iter_mut() {
		key = next_round(key);
		*byte = (*byte).wrapping_sub(key as u8);
	}
	for byte in bytes.iter() {
		use std::fmt::Write;
		let _ = write!(result, "{},", byte);
	}
	result.push_str("] }");
	result
}

fn wencrypt(words: &mut [u16], mut key: u32) -> String {
	let mut result = format!("$crate::WObfString {{ key: {}, data: [", key);
	for word in words.iter_mut() {
		key = next_round(key);
		*word = (*word).wrapping_sub(key as u16);
	}
	for word in words.iter() {
		use std::fmt::Write;
		let _ = write!(result, "{},", word);
	}
	result.push_str("] }");
	result
}

//----------------------------------------------------------------

#[proc_macro_hack]
pub fn random_impl(input: TokenStream) -> TokenStream {
	let mut tt = input.into_iter();
	match tt.next() {
		Some(TokenTree::Ident(ident)) => {
			if let Some(tt) = tt.next() {
				panic!("unexpected token: `{}`", tt);
			}
			random_parse(ident).into()
		},
		Some(tt) => panic!("expected a primitive name: `{}`", tt),
		None => panic!("expected a primitive name"),
	}
}

fn random_parse(input: Ident) -> TokenTree {
	match &*input.to_string() {
		"u8" => Literal::u8_suffixed(rand::random::<u8>()),
		"u16" => Literal::u16_suffixed(rand::random::<u16>()),
		"u32" => Literal::u32_suffixed(rand::random::<u32>()),
		"u64" => Literal::u64_suffixed(rand::random::<u64>()),
		"usize" => Literal::usize_suffixed(rand::random::<usize>()),
		"i8" => Literal::i8_suffixed(rand::random::<i8>()),
		"i16" => Literal::i16_suffixed(rand::random::<i16>()),
		"i32" => Literal::i32_suffixed(rand::random::<i32>()),
		"i64" => Literal::i64_suffixed(rand::random::<i64>()),
		"isize" => Literal::isize_suffixed(rand::random::<isize>()),
		"f32" => Literal::f32_suffixed(rand::random::<f32>()),
		"f64" => Literal::f64_suffixed(rand::random::<f64>()),
		s => panic!("unsupported type: `{}`", s),
	}.into()
}
