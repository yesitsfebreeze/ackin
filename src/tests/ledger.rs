//! The ledger: every cartridge under one root, derived from the filesystem,
//! keyed by its path from that root, and resolved by an outward walk. Both
//! answered forks of the contract are pinned here: a new cartridge is
//! *available, not started*, and two entries of one scope offering one key is
//! a clash that stops, not a silent pick.

use super::*;
use crate::ledger::{Bound, Ledger};
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
	cartridge(dir.path(), "left", json!({"name": "same", "entry": "init.lua"}));
	cartridge(
		dir.path(),
		"left/nested",
		json!({"name": "same", "entry": "init.lua"}),
	);
	cartridge(dir.path(), "right", json!({"name": "same", "entry": "init.lua"}));
	let ledger = Ledger::scan(dir.path());
	assert_eq!(ledger.len(), 3);
	assert_eq!(
		ledger.entries().map(|e| e.path.as_str()).collect::<Vec<_>>(),
		vec!["left", "left/nested", "right"]
	);
	assert_eq!(ledger.get("left/nested").unwrap().name, "same");
}

/// A key is bound from the asker's own subtree first, then steps outward one
/// scope at a time. The identical key one subtree over never wins the walk,
/// and a key deeper than one level is invisible until every level passes it on.
#[test]
fn a_walk_answers_from_the_asker_subtree_then_steps_outward() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"outer",
		json!({"name": "outer", "entry": "init.lua", "needs": ["store.get"]}),
	);
	cartridge(
		dir.path(),
		"outer/inner",
		json!({"name": "inner", "entry": "init.lua", "provide": ["store.get"]}),
	);
	cartridge(
		dir.path(),
		"other",
		json!({"name": "other", "entry": "init.lua", "provide": ["store.get"]}),
	);
	let ledger = Ledger::scan(dir.path());
	match ledger.resolve("outer", "store.get") {
		Bound::One(e) => assert_eq!(e.path, "outer/inner"),
		other => panic!("expected the own-subtree binding, got {:?}", other.paths()),
	}
	// A root-level ask cannot see into `outer`'s subtree: `other` is the only
	// entry of the root scope offering the key.
	match ledger.resolve("", "store.get") {
		Bound::One(e) => assert_eq!(e.path, "other"),
		other => panic!("expected the root-scope binding, got {:?}", other.paths()),
	}
	// The asker itself is not its own answer.
	assert!(matches!(
		ledger.resolve("other", "store.get"),
		Bound::None
	));
}

/// A request made *from* a nested scope steps outward through more than one
/// scope. The asker's own subtree is silent, its parent's scope answers with
/// the asker alone — excluded, because nothing is its own answer — and only the
/// top of the chain has the offer, re-exported from the depth that provides it.
/// Delete the walk's later scopes and this reads `None`: every step past the
/// first is load bearing, and the asker-exclusion here is the case pass two
/// never ran, where the asker sits inside the scope that would otherwise bind.
#[test]
fn a_walk_from_a_nested_scope_steps_outward() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"outer",
		json!({"name": "outer", "entry": "init.lua", "export": ["store.get"]}),
	);
	cartridge(
		dir.path(),
		"outer/inner",
		json!({"name": "inner", "entry": "init.lua"}),
	);
	cartridge(
		dir.path(),
		"outer/inner/deep",
		json!({"name": "deep", "entry": "init.lua", "provide": ["store.get"]}),
	);
	let ledger = Ledger::scan(dir.path());
	match ledger.resolve("outer/inner/deep", "store.get") {
		Bound::One(e) => assert_eq!(e.path, "outer", "the walk ends at the top of the chain"),
		other => panic!("expected the outward binding, got {:?}", other.paths()),
	}
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
		json!({"name": "left", "entry": "init.lua", "provide": ["store.get"]}),
	);
	cartridge(
		dir.path(),
		"right",
		json!({"name": "right", "entry": "init.lua", "provide": ["store.get"]}),
	);
	let ledger = Ledger::scan(dir.path());
	let bound = ledger.resolve("asker", "store.get");
	assert!(bound.is_clashed(), "expected a clash, got {:?}", bound.paths());
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
	cartridge(dir.path(), "broken", json!({"name": "broken", "entry": "init.lua"}));
	write(dir.path(), "broken/cartridge.json", "{ this is not json");
	let ledger = Ledger::scan(dir.path());
	assert_eq!(ledger.len(), 1);
	let entry = ledger.get("broken").unwrap();
	assert_eq!(entry.provide.len() + entry.export.len() + entry.needs.len(), 0);
	assert!(entry.unread.is_some());
}

/// A root that does not exist is an empty ledger, not an error: nothing
/// installed is a state the host runs in.
#[test]
fn a_root_that_does_not_exist_is_an_empty_ledger() {
	let dir = tempfile::tempdir().unwrap();
	assert!(Ledger::scan(&dir.path().join("nowhere")).is_empty());
}

/// The answered fork, as a host fact: a cartridge dropped into the root is an
/// entry the moment it is there — listed, its declarations carried, and
/// `disabled`, so nothing of it runs until something needs it. Taking the
/// folder away is the uninstall, and no list was edited either way.
#[tokio::test(flavor = "multi_thread")]
async fn a_new_cartridge_is_available_and_not_started() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"fresh",
		json!({"name": "fresh", "entry": "init.lua", "provide": ["fresh.key"]}),
	);
	// Nothing of it runs: applying the entry would evaluate it.
	write(dir.path(), "fresh/init.lua", r#"error("a fresh cartridge must not run")"#);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let listed = host.manifest().unwrap();
	let fresh = listed.iter().find(|c| c.entry.id == "fresh").unwrap();
	assert!(fresh.entry.disabled, "available, not started");
	assert!(fresh.provide.is_empty() && fresh.inject.is_empty());
	assert!(host.fiber_of("fresh").is_none(), "nothing of it runs");
	// Uninstalling is taking it away: the entry goes with the folder.
	std::fs::remove_dir_all(dir.path().join("fresh")).unwrap();
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	assert!(!host.manifest().unwrap().iter().any(|c| c.entry.id == "fresh"));
}