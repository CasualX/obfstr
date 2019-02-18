String Obfuscation
==================

Compiletime string literal obfuscation for Rust.

Examples
--------

The `obfstr!` macro returns a borrowed temporary and may not escape the statement it was used in:

```rust
assert_eq!(obfstr::obfstr!("Hello 🌍"), "Hello 🌍");
```

The `local` modifier returns the `ObfBuffer` with the decrypted string and is more flexible but less ergonomic:

```rust
let str_buf = obfstr::obfstr!(local "Hello 🌍");
assert_eq!(str_buf.as_str(), "Hello 🌍");
```

The `const` modifier returns the encrypted `ObfString` for use in constant expressions:

```rust
static GSTR: obfstr::ObfString<[u8; 10]> = obfstr::obfstr!(const "Hello 🌍");
assert_eq!(GSTR.decrypt(0).as_str(), "Hello 🌍");
```

We're already depending on `rand`, why not throw in a compiletime random number generator:

```rust
let r = obfstr::random!(u8);
assert!((r as i32) >= 0 && (r as i32) <= 255);
```

License
-------

Licensed under [MIT License](https://opensource.org/licenses/MIT), see [license.txt](license.txt).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any additional terms or conditions.
