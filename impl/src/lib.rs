#![allow(bare_trait_objects, ellipsis_inclusive_range_patterns)]

extern crate proc_macro;
use proc_macro::*;

//----------------------------------------------------------------

/// Strips any outer `Delimiter::None` groups from the input,
/// returning a `TokenStream` consisting of the innermost
/// non-empty-group `TokenTree`.
/// This is used to handle a proc macro being invoked
/// by a `macro_rules!` expansion.
/// See https://github.com/rust-lang/rust/issues/72545 for background
fn ignore_groups(mut input: TokenStream) -> TokenStream {
	loop {
		let mut tokens = input.clone().into_iter();
		if let Some(TokenTree::Group(group)) = tokens.next() {
			if group.delimiter() == Delimiter::None {
				input = group.stream();
				continue;
			}
		}
		return input;
	}
}

fn strip_groups(token: TokenTree) -> TokenTree {
	ignore_groups(vec![token].into_iter().collect()).into_iter().next().unwrap()
}

#[cfg(feature = "rand")]
#[proc_macro_attribute]
pub fn obfstr_attribute(args: TokenStream, input: TokenStream) -> TokenStream {
	drop(args);
	replace_macro(replace_macro(input, "_obfstr_", obfstr_impl), "_strlen_", strlen_impl)
}

#[cfg(feature = "rand")]
fn strlen_impl(mut input: TokenStream) -> TokenStream {
	input = ignore_groups(input);
	if let Some(TokenTree::Literal(literal)) = input.into_iter().next() {
		let s = string_parse(literal);
		TokenStream::from(TokenTree::Literal(Literal::usize_suffixed(s.len())))
	}
	else {
		panic!("expected a string literal")
	}
}
#[cfg(feature = "rand")]
fn obfstr_impl(mut input: TokenStream) -> TokenStream {
	input = ignore_groups(input);
	let mut tt = input.into_iter();
	let mut token = tt.next();

	// Optional L ident prefix to indicate wide strings
	let mut wide = false;
	if let Some(TokenTree::Ident(ident)) = &token.clone().map(strip_groups) {
		if ident.to_string() == "L" {
			wide = true;
			token = tt.next();
		}
	}

	// Followed by a string literal
	let string = match token.map(strip_groups)  {
		Some(TokenTree::Literal(lit)) => string_parse(lit),
		Some(tt) => panic!("expected a string literal: `{:?}`", tt),
		None => panic!("expected a string literal"),
	};

	// End of macro arguments
	token = tt.next();
	if let Some(tt) = token {
		panic!("unexpected token: `{}`", tt);
	}

	// Generate a random key
	let key = rand::random::<u32>();
	// Obfuscate the string itself
	let array = if wide {
		let mut words = {string}.encode_utf16().collect::<Vec<u16>>();
		wencrypt(&mut words, key)
	}
	else {
		let mut bytes = string.into_bytes();
		encrypt(&mut bytes, key)
	}.into_iter().collect();

	// Generate `key, [array]` to be passed to ObfString constructor
	vec![
		TokenTree::Literal(Literal::u32_suffixed(key)),
		TokenTree::Punct(Punct::new(',', Spacing::Alone)),
		TokenTree::Group(Group::new(Delimiter::Bracket, array)),
	].into_iter().collect()
}

#[cfg(feature = "rand")]
fn next_round(mut x: u32) -> u32 {
	x ^= x << 13;
	x ^= x >> 17;
	x ^= x << 5;
	x
}

#[cfg(feature = "rand")]
fn encrypt(bytes: &mut [u8], mut key: u32) -> Vec<TokenTree> {
	for byte in bytes.iter_mut() {
		key = next_round(key);
		*byte = (*byte).wrapping_sub(key as u8);
	}
	let mut array = Vec::new();
	for &byte in bytes.iter() {
		array.push(TokenTree::Literal(Literal::u8_suffixed(byte)));
		array.push(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
	}
	array
}

#[cfg(feature = "rand")]
fn wencrypt(words: &mut [u16], mut key: u32) -> Vec<TokenTree> {
	for word in words.iter_mut() {
		key = next_round(key);
		*word = (*word).wrapping_sub(key as u16);
	}
	let mut array = Vec::new();
	for &word in words.iter() {
		array.push(TokenTree::Literal(Literal::u16_suffixed(word)));
		array.push(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
	}
	array
}

//----------------------------------------------------------------

fn string_parse(input: Literal) -> String {
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

//----------------------------------------------------------------

/// Wide string literal, returns an array of words.
///
/// The type of the returned literal is `&'static [u16; LEN]`.
///
/// ```
/// # use obfstr_impl as obfstr;
/// let expected = &['W' as u16, 'i' as u16, 'd' as u16, 'e' as u16, 0];
/// assert_eq!(obfstr::wide!("Wide\0"), expected);
/// ```
#[proc_macro]
pub fn wide(input: TokenStream) -> TokenStream {
	let input = ignore_groups(input);
	// Parse the input as a single string literal
	let mut iter = input.into_iter();
	let string = match iter.next() {
		Some(TokenTree::Literal(lit)) => string_parse(lit),
		Some(tt) => panic!("expected a string literal: `{}`", tt),
		None => panic!("expected a string literal"),
	};
	if let Some(tt) = iter.next() {
		panic!("unexpected token: `{}`", tt);
	}
	// Encode the string literal as an array of words
	let mut array = Vec::new();
	for word in string.encode_utf16() {
		array.push(TokenTree::Literal(Literal::u16_suffixed(word)));
		array.push(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
	}
	let elements = array.into_iter().collect();
	// Wrap the array of words in a reference
	vec![
		TokenTree::Punct(Punct::new('&', Spacing::Alone)),
		TokenTree::Group(Group::new(Delimiter::Bracket, elements)),
	].into_iter().collect()
}

//----------------------------------------------------------------

/// Compiletime random number generator.
///
/// Every time the code is compiled, a new random number literal is generated.
/// Recompilation (and thus regeneration of the number) is not triggered automatically.
///
/// Supported types are `u8`, `u16`, `u32`, `u64`, `usize`, `i8`, `i16`, `i32`, `i64`, `isize`, `bool`, `f32` and `f64`.
///
/// ```
/// # use obfstr_impl as obfstr;
/// const RND: i32 = obfstr::random!(u8) as i32;
/// assert!(RND >= 0 && RND <= 255);
/// ```
#[cfg(feature = "rand")]
#[proc_macro]
pub fn random(input: TokenStream) -> TokenStream {
	let input = ignore_groups(input);
	let mut tt = input.into_iter();
	match tt.next() {
		Some(TokenTree::Ident(ident)) => {
			if let Some(tt) = tt.next() {
				panic!("unexpected token: `{}`", tt);
			}
			TokenTree::Literal(match &*ident.to_string() {
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
			}).into()
		},
		Some(tt) => panic!("expected a primitive name: `{}`", tt),
		None => panic!("expected a primitive name"),
	}
}

//----------------------------------------------------------------
// Implements a tt muncher for proc-macros

fn replace<F>(input: TokenStream, mut f: F) -> TokenStream
	where F: FnMut(&[TokenTree]) -> Option<(usize, TokenStream)>
{
	let input: Vec<TokenTree> = input.into_iter().collect();
	let mut output = Vec::new();
	replace_rec(input, &mut output, &mut f);
	output.into_iter().collect()
}
fn replace_rec(input: Vec<TokenTree>, output: &mut Vec<TokenTree>, f: &mut FnMut(&[TokenTree]) -> Option<(usize, TokenStream)>) {
	let mut into_iter = input.into_iter();
	loop {
		// If tokens are matched, insert the replacement and skip some tokens
		if let Some((mut skip, replace)) = f(into_iter.as_slice()) {
			output.extend(replace);
			while skip > 0 {
				let _ = into_iter.next();
				skip -= 1;
			}
		}
		match into_iter.next() {
			// Recursively process into groups
			Some(TokenTree::Group(group)) => {
				let group_input = group.stream().into_iter().collect();
				let mut group_output = Vec::new();
				replace_rec(group_input, &mut group_output, f);
				let group_stream = group_output.into_iter().collect();
				output.push(TokenTree::Group(Group::new(group.delimiter(), group_stream)));
			},
			Some(tt) => output.push(tt),
			None => break,
		}
	}
}
// Replaces invocations of `$name!($tokens)` with the output of the callable given the `$tokens`.
fn replace_macro(input: TokenStream, name: &str, f: fn(TokenStream) -> TokenStream) -> TokenStream {
	replace(input, |tokens| {
		if tokens.len() >= 3 {
			if let (
				TokenTree::Ident(ident),
				TokenTree::Punct(punct),
				TokenTree::Group(group),
			) = (
				&tokens[0],
				&tokens[1],
				&tokens[2],
			) {
				if punct.as_char() == '!' && ident.to_string() == name {
					return Some((3, f(group.stream())));
				}
			}
		}
		None
	})
}
