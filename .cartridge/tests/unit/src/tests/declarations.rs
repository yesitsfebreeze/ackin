//! `declarations`: a node reads back the events its own manifest declared.
use std::path::Path;

use serde_json::{json, Value};

use super::{built, write};
use crate::host::Host;

fn cartridge(dir: &Path, id: &str, manifest: Value, init: &str) {
	write(dir, &format!("{id}/cartridge.json"), &manifest.to_string());
	write(dir, &format!("{id}/init.lua"), init);
}

fn compose(dir: &Path, ids: &[&str]) {
	let entries: Vec<String> = ids
		.iter()
		.map(|id| format!("{{id={id:?}, path={id:?}}}"))
		.collect();
	write(
		dir,
		".cartridge/init.lua",
		&format!("return {{ {} }}", entries.join(", ")),
	);
	super::home();
	crate::trust::record(dir).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cartridge_reads_its_own_declared_events_and_no_one_elses() {
	static BIN: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	let bin = BIN.get_or_init(|| built(&["--bin", "cartridge"]));
	std::env::set_var(crate::host::NODE_BIN_ENV, bin);
	let dir = tempfile::tempdir().unwrap();
	for id in ["left", "right"] {
		cartridge(
			dir.path(),
			id,
			json!({"name": id, "entry": "init.lua", "listen": [format!("{id}.ask")], "events": {
				format!("{id}.ask"): {"description": "ask", "frame": "data"},
				format!("{id}.said"): {"frame": "message", "timeout_ms": 5},
			}}),
			&format!(
				r#"cartridge.listen("{id}.ask", function() return cartridge.host("declarations", nil) end)"#
			),
		);
	}
	compose(dir.path(), &["left", "right"]);
	let host = Host::new(dir.path(), dir.path().join(".cartridge")).unwrap();
	host.reconcile().await.unwrap();
	let left = host.bail("left.ask", json!(null)).await.unwrap().unwrap();
	let right = host.bail("right.ask", json!(null)).await.unwrap().unwrap();
	assert_eq!(
		left,
		json!({
			"left.ask": {"description": "ask", "frame": "data"},
			"left.said": {"frame": "message", "timeout_ms": 5},
		})
	);
	assert_eq!(
		right.as_object().unwrap().keys().collect::<Vec<_>>(),
		["right.ask", "right.said"]
	);
	let types = host.asp(json!({"op": "types"})).await.unwrap();
	assert_eq!(types["events"]["left.said"]["frame"], "message");
	assert_eq!(types["events"]["right.ask"]["frame"], "data");
	host.stop().await;
}
