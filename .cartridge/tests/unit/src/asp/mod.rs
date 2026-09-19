//! Fixture providers every ASP test composes, and one file per behaviour.

mod actions;
mod activity_tool;
mod base;
mod composition;
mod doors;
mod manifest;
mod merge;
mod search;
mod tree;

use std::path::Path;
use std::sync::Arc;

use serde_json::{json, Value};

use crate::host::{Host, State};
use crate::tests::{built, home, write};

pub(super) fn cartridge(dir: &Path, id: &str, manifest: Value, init: &str) {
	write(dir, &format!("{id}/cartridge.json"), &manifest.to_string());
	write(dir, &format!("{id}/init.lua"), init);
}

pub(super) fn descriptor(dir: &Path, ids: &[&str]) {
	let entries: Vec<String> = ids
		.iter()
		.map(|id| format!("{{id={id:?}, path={id:?}}}"))
		.collect();
	write(
		dir,
		".cartridge/init.lua",
		&format!("return {{ {} }}", entries.join(", ")),
	);
	home();
	crate::trust::record(dir).unwrap();
}

pub(super) async fn boot(dir: &Path, ids: &[&str]) -> Arc<Host> {
	static BIN: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	let bin = BIN.get_or_init(|| built(&["--bin", "cartridge"]));
	std::env::set_var(crate::host::NODE_BIN_ENV, bin);
	descriptor(dir, ids);
	let host = Host::new(dir, dir.join(".cartridge")).unwrap();
	host.reconcile().await.unwrap();
	for id in ids {
		let status = host.status().into_iter().find(|s| s.id == *id).unwrap();
		assert_eq!(status.state, State::Active, "{id}: {:?}", status.error);
	}
	host
}

/// Owns `file:`. Its revision of every file moves when `bump` is sent, and it
/// offers one action, which runs the `tool.open` event.
pub(super) fn files(dir: &Path) {
	cartridge(
		dir,
		"files",
		json!({
			"name": "files", "entry": "init.lua",
			"events": {"asp.files": {}, "bump": {}},
			"listen": ["asp.files", "bump"],
			"asp": {
				"schemes": {"file": {"description": "a workspace file", "owner": true}},
				"edges": {"imports": {}},
				"attributes": {"files.bytes": {}},
				"actions": [{"name": "open", "applies_to": "file", "effect": "read",
					"tool": "tool.open", "args": {"path": "${key}"}}],
				"search": true,
			},
		}),
		r#"local revision = "r1"
		cartridge.listen("bump", function() revision = "r2" return revision end)
		cartridge.listen("asp.files", function(request)
			if request.op == "search" then
				return {
					nodes = { { id = "file:src/zeta.rs", name = "zeta.rs" }, { id = "file:src/alpha.rs", name = "alpha.rs" } },
					edges = { { from = "file:src/zeta.rs", to = "file:src/alpha.rs", kind = "imports" } },
				}
			end
			return { nodes = { { id = request.entity, revision = revision, name = "a.rs", attributes = { ["files.bytes"] = 12 } } } }
		end)"#,
	);
}

/// Owns `range:` and links files to ranges. It always answers against `r1`.
pub(super) fn search(dir: &Path) {
	cartridge(
		dir,
		"search",
		json!({
			"name": "search", "entry": "init.lua",
			"events": {"asp.search": {}}, "listen": ["asp.search"],
			"asp": {
				"schemes": {"file": {}, "range": {"owner": true}},
				"edges": {"matches": {"description": "the file holds a match at the range"}},
			},
		}),
		r#"cartridge.listen("asp.search", function(request)
			local range = "range:src/a.rs:4:1-4:9"
			return {
				nodes = { { id = range, revision = "r1" } },
				edges = { { from = request.entity, to = range, kind = "matches", revision = "r1" } },
			}
		end)"#,
	);
}

pub(super) fn ids(answer: &Value) -> Vec<&str> {
	answer["nodes"]
		.as_array()
		.unwrap()
		.iter()
		.map(|n| n["id"].as_str().unwrap())
		.collect()
}

pub(super) async fn expand(host: &Host, entity: &str) -> Value {
	host.asp(json!({"op": "expand", "entity": entity}))
		.await
		.unwrap()
}
