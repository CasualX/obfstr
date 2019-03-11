String Obfuscation
==================

Compiletime string literal obfuscation for Rust.

Examples
--------

Comes with a simple `wide!` macro to provide compile time utf16 string literals:

```rust
let expected = &['W' as u16, 'i' as u16, 'd' as u16, 'e' as u16];
assert_eq!(obfstr::wide!("Wide"), expected);
```

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
const RND: i32 = obfstr::random!(u8) as i32;
assert!(RND >= 0 && RND <= 255);
```

License
-------

Licensed under [MIT License](https://opensource.org/licenses/MIT), see [license.txt](license.txt).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any additional terms or conditions.
