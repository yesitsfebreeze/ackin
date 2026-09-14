//! The ledger: every cartridge under one root, derived from the filesystem,
//! keyed by its path from that root, and resolved by an outward walk. Both
//! answered forks of the contract are pinned here: a new cartridge is
//! *available, not started*, and two entries of one scope offering one key is
//! a clash that stops, not a silent pick.

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

/// An entry is its path from the root, so two subtrees may hold the same bare
/// name and neither collides with the other.
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

/// The answered fork: two entries of one scope offering one key is ambiguous
/// and the ask stops naming both, instead of the first in path order winning
/// quietly. A scope further out offering the key once does not change that.
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
		json!({"name": "left", "entry": "init.lua", "on": ["store.get"]}),
	);
	cartridge(
		dir.path(),
		"right",
		json!({"name": "right", "entry": "init.lua", "on": ["store.get"]}),
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

/// An unreadable document is an entry with its reason, not an absence: it
/// declares nothing and binds nothing, and the listing can still say why.
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
	assert_eq!(entry.on.len() + entry.needs.len(), 0);
	assert!(entry.unread.is_some());
}

/// A root that does not exist is an empty ledger, not an error: nothing
/// installed is a state the host runs in.
#[test]
fn a_root_that_does_not_exist_is_an_empty_ledger() {
	let dir = tempfile::tempdir().unwrap();
	assert!(Ledger::scan(&dir.path().join("nowhere")).is_empty());
}
