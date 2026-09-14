mod host;
mod ledger;
mod settings;

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

fn write(dir: &Path, name: &str, body: &str) {
	let path = dir.join(name);
	std::fs::create_dir_all(path.parent().unwrap()).unwrap();
	std::fs::write(path, body).unwrap();
}

/// Build a target through cargo and return the binary or library it produced.
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
