use super::*;
use crate::ledger::Ledger;
use serde_json::json;

fn cartridge(dir: &Path, folder: &str, manifest: Value) {
	std::fs::create_dir_all(dir.join(folder)).unwrap();
	write(
		dir,
		&format!("{folder}/cartridge.json"),
		&manifest.to_string(),
	);
	write(dir, &format!("{folder}/init.lua"), "return {}");
}

#[test]
fn an_entry_is_its_path_from_the_root() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"left",
		json!({"name": "same", "entry": "init.lua"}),
	);
	cartridge(
		dir.path(),
		"left/nested",
		json!({"name": "same", "entry": "init.lua"}),
	);
	cartridge(
		dir.path(),
		"right",
		json!({"name": "same", "entry": "init.lua"}),
	);
	let ledger = Ledger::scan(dir.path());
	assert_eq!(ledger.len(), 3);
	assert_eq!(
		ledger
			.entries()
			.map(|e| e.path.as_str())
			.collect::<Vec<_>>(),
		vec!["left", "left/nested", "right"]
	);
	assert_eq!(ledger.get("left/nested").unwrap().name, "same");
}

#[test]
fn two_entries_of_one_scope_offering_one_key_is_a_clash() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"asker",
		json!({"name": "asker", "entry": "init.lua", "needs": ["store.get"]}),
	);
	cartridge(
		dir.path(),
		"left",
		json!({"name": "left", "entry": "init.lua", "listen": ["store.get"]}),
	);
	cartridge(
		dir.path(),
		"right",
		json!({"name": "right", "entry": "init.lua", "listen": ["store.get"]}),
	);
	let ledger = Ledger::scan(dir.path());
	let bound = ledger.resolve("asker", "store.get");
	assert!(
		bound.is_clashed(),
		"expected a clash, got {:?}",
		bound.paths()
	);
	assert_eq!(
		bound.paths(),
		vec!["left", "right"],
		"a clash names both offers, in path order"
	);
	assert!(ledger
		.bindings()
		.iter()
		.any(|(e, key, b)| e.path == "asker" && *key == "store.get" && b.is_clashed()));
}

#[test]
fn an_unreadable_document_is_an_entry_with_its_reason() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"broken",
		json!({"name": "broken", "entry": "init.lua"}),
	);
	write(dir.path(), "broken/cartridge.json", "{ this is not json");
	let ledger = Ledger::scan(dir.path());
	assert_eq!(ledger.len(), 1);
	let entry = ledger.get("broken").unwrap();
	assert_eq!(entry.listen.len() + entry.needs.len(), 0);
	assert!(entry.unread.is_some());
}

#[test]
fn a_root_that_does_not_exist_is_an_empty_ledger() {
	let dir = tempfile::tempdir().unwrap();
	assert!(Ledger::scan(&dir.path().join("nowhere")).is_empty());
}
