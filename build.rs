fn main() {
	if cfg!(target_os = "macos") {
		println!("cargo:rustc-link-arg-bins=-Wl,-export_dynamic");
	} else if cfg!(target_os = "linux") {
		println!("cargo:rustc-link-arg-bins=-rdynamic");
	}
}
