String Obfuscation
==================

[![MIT License](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![crates.io](https://img.shields.io/crates/v/obfstr.svg)](https://crates.io/crates/obfstr)
[![docs.rs](https://docs.rs/obfstr/badge.svg)](https://docs.rs/obfstr)
[![Build status](https://github.com/CasualX/obfstr/workflows/CI/badge.svg)](https://github.com/CasualX/obfstr/actions)

Compiletime string constant obfuscation for Rust.

The string constant itself is embedded in obfuscated form and deobfuscated locally.
This reference to a temporary value must be used in the same statement it was generated.
See the documentation for more advanced use cases.

If you're looking for obfuscating format strings (`format!`, `println!`, etc.) I have another crate [`fmtools`](https://crates.io/crates/fmtools) with the optional dependency `obfstr` enabled to automatically apply string obfuscation to your formatting strings.

Examples
--------

The `obfstr!` macro returns the deobfuscated string as a temporary value:

```rust
assert_eq!(obfstr::obfstr!("Hello 🌍"), "Hello 🌍");
```

The `wide!` macro provides compiletime utf16 string constants:

```rust
let expected = &['W' as u16, 'i' as u16, 'd' as u16, 'e' as u16, 0];
assert_eq!(obfstr::wide!("Wide\0"), expected);
```

The `random!` macro provides compiletime random values:

```rust
const RND: i32 = obfstr::random!(u8) as i32;
assert!(RND >= 0 && RND <= 255);
```

Compiletime random values are based on `file!()`, `line!()`, `column!()` and a fixed seed to ensure reproducibility.
This fixed seed is stored as text in the environment variable `OBFSTR_SEED` and can be changed as desired.

License
-------

Licensed under [MIT License](https://opensource.org/licenses/MIT), see [license.txt](license.txt).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any additional terms or conditions.
