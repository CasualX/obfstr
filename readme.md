String Obfuscation
==================

Simple string obfuscation for Rust.

Examples
--------

Visit [this page](https://casualhacks.net/obfstr/index.html) to generate your obfuscated strings.

```rust
let s = obfstr::obfstr!(/*Hello 🌍*/ 2803150042,11,63,105,38,140,200,70,29,83,200);
assert_eq!(s.as_str(), "Hello 🌍");
```

License
-------

Licensed under [MIT License](https://opensource.org/licenses/MIT), see [license.txt](license.txt).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any additional terms or conditions.
