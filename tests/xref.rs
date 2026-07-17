#![allow(dead_code)]

struct Cleanup<'a>(&'a std::path::Path);

impl Drop for Cleanup<'_> {
	fn drop(&mut self) {
		let _ = std::fs::remove_dir_all(self.0);
	}
}

#[cfg(all(target_arch = "x86_64", target_os = "linux", not(miri)))]
#[test]
fn xref_reference_uses_relocation_addend() {
	use std::{env, fs, process::Command};

	let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
	let target_dir = env::temp_dir().join(format!("obfstr-xref-{}", std::process::id()));
	let _cleanup = Cleanup(&target_dir);

	let output = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
		.args([
			"rustc",
			"--quiet",
			"--release",
			"--example",
			"obfuscation",
			"--target-dir",
		])
		.arg(&target_dir)
		.args(["--", "--emit=asm", "-Cllvm-args=-x86-asm-syntax=intel"])
		.current_dir(manifest_dir)
		.output()
		.expect("failed to compile the obfuscation example");
	assert!(output.status.success(), "build failed:\n{}", String::from_utf8_lossy(&output.stderr));

	let examples_dir = target_dir.join("release/examples");
	let asm_path = fs::read_dir(&examples_dir).unwrap()
		.map(|entry| entry.unwrap().path())
		.find(|path| {
			path.extension().is_some_and(|extension| extension == "s") &&
			path.file_stem().is_some_and(|stem| stem.to_string_lossy().starts_with("obfuscation-"))
		})
		.expect("build did not emit assembly");
	let asm = fs::read_to_string(asm_path).unwrap();
	let reference = asm.lines()
		.find(|line| line.contains("[rip") && line.contains("XREF_ADDEND_TARGET"))
		.expect("could not find the xref target reference in the assembly");
	// A plain reference is functionally correct too, so only codegen inspection can distinguish
	// `XREF_ADDEND_TARGET` from the intended `XREF_ADDEND_TARGET-N` reference.
	let reference = reference.replace(' ', "");
	let addend = reference.split_once("XREF_ADDEND_TARGET-")
		.and_then(|(_, addend)| addend.trim_end_matches(']').parse::<usize>().ok());
	assert!(addend.is_some_and(|addend| addend != 0), "xref target reference: {reference}");
}
