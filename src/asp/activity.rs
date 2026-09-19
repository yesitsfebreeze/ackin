//! Observed ASP use and host dispatches, retained for late-joining viewers.
//! This is telemetry, not a cache of provider answers or a claim of importance.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Component, Path, PathBuf};

use parking_lot::Mutex;
use serde_json::{json, Value};

const RETAINED: usize = 1024;

#[derive(Default)]
pub(crate) struct Activity(Mutex<Journal>);

#[derive(Default)]
struct Journal {
	serial: u64,
	records: VecDeque<Value>,
	nodes: BTreeMap<String, Value>,
	edges: VecDeque<Value>,
}

fn now_ms() -> u64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_default()
		.as_millis() as u64
}

fn file_id(root: &Path, request: &Value, path: &str) -> Option<String> {
	if path.is_empty() || path.len() > 4096 {
		return None;
	}
	let cwd = request["context"]["cwd"]
		.as_str()
		.map(Path::new)
		.unwrap_or(root);
	let joined = cwd.join(path);
	let mut clean = PathBuf::new();
	for part in joined.components() {
		match part {
			Component::ParentDir => {
				clean.pop();
			}
			Component::CurDir => {}
			other => clean.push(other.as_os_str()),
		}
	}
	let relative = clean.strip_prefix(root).ok()?;
	Some(format!("file:{}", relative.to_str()?))
}

impl Activity {
	pub(crate) fn record(&self, operation: &str, entities: Vec<String>, answer: Option<&Value>) {
		let mut journal = self.0.lock();
		let nodes = answer.into_iter().flat_map(|a| {
			a["nodes"].as_array().into_iter().flatten().chain(
				a["hits"]
					.as_array()
					.into_iter()
					.flatten()
					.map(|h| &h["node"]),
			)
		});
		for node in nodes {
			if let Some(id) = node["id"].as_str() {
				journal.nodes.insert(
					id.to_owned(),
					json!({"id": id, "name": node["name"], "tags": node["tags"]}),
				);
			}
		}
		for edge in answer
			.into_iter()
			.flat_map(|a| a["edges"].as_array().into_iter().flatten())
		{
			let edge = json!({"from": edge["from"], "to": edge["to"], "kind": edge["kind"]});
			if !journal.edges.contains(&edge) {
				journal.edges.push_back(edge);
			}
		}
		journal.serial += 1;
		let seq = journal.serial;
		journal.records.push_back(
			json!({"seq": seq, "ts_ms": now_ms(), "operation": operation, "entities": entities}),
		);
		while journal.records.len() > RETAINED {
			journal.records.pop_front();
		}
		while journal.edges.len() > RETAINED {
			journal.edges.pop_front();
		}
		let referenced: BTreeSet<String> = journal
			.records
			.iter()
			.flat_map(|r| r["entities"].as_array().into_iter().flatten())
			.filter_map(Value::as_str)
			.map(str::to_owned)
			.collect();
		if journal.nodes.len() > RETAINED {
			journal.nodes.retain(|id, _| referenced.contains(id));
			while journal.nodes.len() > RETAINED {
				journal.nodes.pop_first();
			}
		}
	}

	pub(crate) fn dispatch(&self, root: &Path, name: &str, request: &Value) {
		if name == "trace"
			|| name.starts_with("scope.detail.")
			|| name.starts_with("asp.")
			|| matches!(
				request["op"].as_str(),
				Some("describe" | "status" | "health" | "ready")
			) || (name == "memory"
			&& request["op"] == "trace"
			&& matches!(request["action"].as_str(), Some("cycles" | "status")))
		{
			return;
		}
		let mut entities = vec![format!("event:{name}")];
		let input = request.get("input").unwrap_or(request);
		if let Some(id) = input["path"]
			.as_str()
			.and_then(|p| file_id(root, request, p))
		{
			entities.push(id);
		}
		if let Some(entity) = input["entity"]
			.as_str()
			.filter(|s| super::protocol::scheme_of(s).is_some())
		{
			entities.push(entity.to_owned());
		}
		self.record("dispatch", entities, None);
	}

	pub(crate) fn snapshot(&self) -> Value {
		let journal = self.0.lock();
		json!({"generation": std::process::id(), "capacity": RETAINED,
			"records": journal.records, "nodes": journal.nodes.values().collect::<Vec<_>>(), "edges": journal.edges})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn retains_1024_uses_and_reading_does_not_heat_the_graph() {
		let activity = Activity::default();
		for _ in 0..1100 {
			activity.dispatch(
				Path::new("/workspace"),
				"tool.read",
				&json!({"input": {"path": "src/a.rs"}}),
			);
		}
		let before = activity.snapshot();
		activity.dispatch(
			Path::new("/workspace"),
			"scope.detail.memo",
			&json!({"version": 1, "item": {"id": "memo:example"}}),
		);
		activity.dispatch(
			Path::new("/workspace"),
			"memory",
			&json!({"op": "trace", "action": "cycles"}),
		);
		assert_eq!(before, activity.snapshot());
		assert_eq!(before["records"].as_array().unwrap().len(), 1024);
		assert_eq!(before["records"][0]["seq"], 77);
		assert_eq!(before["records"][0]["entities"][1], "file:src/a.rs");
	}

	#[test]
	fn observes_graph_edges_without_retaining_content() {
		let activity = Activity::default();
		activity.record(
			"expand",
			vec!["file:a".into()],
			Some(&json!({
				"nodes": [{"id": "file:a", "name": "a", "description": "private contents"}],
				"edges": [{"from": "file:a", "to": "symbol:a#main", "kind": "contains"}]
			})),
		);
		let snapshot = activity.snapshot();
		assert_eq!(snapshot["edges"][0]["kind"], "contains");
		assert!(!snapshot.to_string().contains("private contents"));
	}
}
