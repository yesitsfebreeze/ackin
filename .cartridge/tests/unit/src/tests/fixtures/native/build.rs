// The Lua C API comes from the base binary that loads this module.
fn main() {
	if cfg!(target_os = "macos") {
		println!("cargo:rustc-cdylib-link-arg=-undefined");
		println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
	}
}
