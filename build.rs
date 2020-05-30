use std::env;

fn main() {
	// Accept external source of randomness
	println!("cargo:rerun-if-changed=OBFSTR_SEED");

	// Ensure there's always a valid seed
	if let Err(_) = env::var("OBFSTR_SEED") {
		println!("cargo:rustc-env=OBFSTR_SEED=FIXED");
	}
}
