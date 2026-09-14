mod host;
mod ledger;
mod settings;

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

pub(crate) fn write(dir: &Path, name: &str, body: &str) {
	let path = dir.join(name);
	std::fs::create_dir_all(path.parent().unwrap()).unwrap();
	std::fs::write(path, body).unwrap();
}

/// This test binary's `$CARTRIDGE_HOME`, set once: every test records trust
/// here, never in the developer's, and no test switches it under another.
pub(crate) fn home() -> &'static Path {
	static HOME: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
	HOME.get_or_init(|| {
		let home = tempfile::tempdir().unwrap();
		std::env::set_var("CARTRIDGE_HOME", home.path());
		home
	})
	.path()
}

/// Build a target through cargo and return the binary or library it produced.
pub(crate) fn built(args: &[&str]) -> PathBuf {
	let output = std::process::Command::new(env!("CARGO"))
		.arg("build")
		.args(args)
		.arg("--message-format=json")
		.current_dir(env!("CARGO_MANIFEST_DIR"))
		.output()
		.unwrap();
	assert!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
	let messages: Vec<Value> = String::from_utf8(output.stdout)
		.unwrap()
		.lines()
		.filter_map(|line| serde_json::from_str::<Value>(line).ok())
		.filter(|m| m["reason"] == "compiler-artifact")
		.collect();
	messages
		.iter()
		.find_map(|m| m["executable"].as_str())
		.or_else(|| {
			messages
				.iter()
				.rev()
				.find(|m| m["target"]["kind"] != json!(["custom-build"]))
				.and_then(|m| m["filenames"].as_array()?.first()?.as_str())
		})
		.map(PathBuf::from)
		.unwrap_or_else(|| panic!("cargo build {args:?} produced no executable"))
}
