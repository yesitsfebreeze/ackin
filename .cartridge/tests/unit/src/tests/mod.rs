mod host;
mod ledger;
mod settings;

use std::path::{Path, PathBuf};

use serde_json::Value;

fn write(dir: &Path, name: &str, body: &str) {
	let path = dir.join(name);
	std::fs::create_dir_all(path.parent().unwrap()).unwrap();
	std::fs::write(path, body).unwrap();
}

/// Build a target through cargo and return the executable it produced.
fn built(args: &[&str]) -> PathBuf {
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
	String::from_utf8(output.stdout)
		.unwrap()
		.lines()
		.filter_map(|line| serde_json::from_str::<Value>(line).ok())
		.find_map(|m| m["executable"].as_str().map(PathBuf::from))
		.unwrap_or_else(|| panic!("cargo build {args:?} produced no executable"))
}
