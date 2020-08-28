use std::env;

fn main() {
	// "Forgive me, Father, for I have sinned"
	println!("cargo:rustc-env=RUSTC_BOOTSTRAP=1");

	// Accept external source of randomness
	println!("cargo:rerun-if-env-changed=OBFSTR_SEED");

	// Ensure there's always a valid seed
	if let Err(_) = env::var("OBFSTR_SEED") {
		println!("cargo:rustc-env=OBFSTR_SEED=FIXED");
	}
}
