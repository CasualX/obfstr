String Obfuscation
==================

[![MIT License](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![crates.io](https://img.shields.io/crates/v/obfstr.svg)](https://crates.io/crates/obfstr)
[![docs.rs](https://docs.rs/obfstr/badge.svg)](https://docs.rs/obfstr)
[![Build Status](https://travis-ci.org/CasualX/obfstr.svg?branch=master)](https://travis-ci.org/CasualX/obfstr)

Compiletime string literal obfuscation for Rust.

Examples
--------

The `obfstr!` macro returns a borrowed temporary and may not escape the statement it was used in:

```rust
assert_eq!(obfstr::obfstr!("Hello 🌍"), "Hello 🌍");
```

The `obflocal!` macro returns the `ObfBuffer` with the deobfuscated string and is more flexible but less ergonomic:

```rust
let str_buf = obfstr::obflocal!("Hello 🌍");
assert_eq!(str_buf.as_str(), "Hello 🌍");
```

The `obfconst!` macro returns the encrypted `ObfString` for use in constant expressions:

```rust
static GSTR: obfstr::ObfString<[u8; 10]> = obfstr::obfconst!("Hello 🌍");
assert_eq!(GSTR.deobfuscate(0).as_str(), "Hello 🌍");
```

The `wide!` macro provides compile time utf16 string literals:

```rust
let expected = &['W' as u16, 'i' as u16, 'd' as u16, 'e' as u16, 0];
assert_eq!(obfstr::wide!("Wide\0"), expected);
```

The `random!` macro provides compile time random values:

```rust
const RND: i32 = obfstr::random!(u8) as i32;
assert!(RND >= 0 && RND <= 255);
```

License
-------

Licensed under [MIT License](https://opensource.org/licenses/MIT), see [license.txt](license.txt).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any additional terms or conditions.
