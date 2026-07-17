/*!
Try out the obfuscation live!

```
cargo rustc --release --example obfuscation -- --emit asm -C "llvm-args=-x86-asm-syntax=intel"
```

Inspect `target/release/examples/obfuscation.s` to see the compiled code.

These examples are #[inline(never)] to aid inspection of the generated code.
In practice the generated code is inlined and mixed with their surrounding code.
*/

#[inline(never)]
fn obfstmt() -> i32 {
	let mut i = 0;
	// trace_macros!(true);
	obfstr::obfstmt! {
		i = 5;
		i *= 24;
		i -= 10;
		i += 8;
		i *= 28;
		i -= 18;
		i += 1;
		i *= 21;
		i -= 11;
	}
	// trace_macros!(false);
	assert_eq!(i, 69016);
	i
}

#[inline(never)]
fn obfstr() {
	print(obfstr::obfstr!("Hello world!"));
	print(obfstr::obfstr!("AB"));
	print(obfstr::obfstr!("This literal is very very very long to see if it correctly handles long strings"));
}

#[inline(never)]
fn xref() -> &'static i32 {
	static XREF_ADDEND_TARGET: i32 = 3141592;
	// Fixed generics keep the addend deterministic for the codegen regression test.
	obfstr::xref::xref::<_, 0xD22D8787, 0x2CA5A9509425F502>(&XREF_ADDEND_TARGET)
}

fn main() {
	println!("obfstmt: {}", obfstmt());
	obfstr();
	println!("xref: {}", xref());
}

#[inline(never)]
fn print(s: &str) {
	println!("obfstr: {}", s);
}
