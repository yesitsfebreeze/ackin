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
	assert!(matches!(ledger.resolve("other", "store.get"), Bound::None));
}

/// A request made *from* a nested scope steps outward through more than one
/// scope, on a tree every valid document passes: `deep` declares the need, the
/// two scopes between it and the top are silent, and `outer` provides. The
/// asker's own subtree is silent, its parent's scope is silent, and only the
/// root scope answers — cut the walk to the asker's own subtree and the need
/// reads `?`, so every step past the first is load bearing. The ask is a
/// declared need the registry's own `bindings()` can make; the asker-exclusion
/// the doc comment here once claimed to pin is not reachable from a valid
/// document (the format refuses a key that is both provided here and needed
/// from outside) and is pinned synthetically in
/// `a_walk_answers_from_the_asker_subtree_then_steps_outward` instead.
#[test]
fn a_walk_from_a_nested_scope_steps_outward() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"outer",
		json!({"name": "outer", "entry": "init.lua", "provide": ["outer.top"]}),
	);
	cartridge(
		dir.path(),
		"outer/inner",
		json!({"name": "inner", "entry": "init.lua"}),
	);
	cartridge(
		dir.path(),
		"outer/inner/deep",
		json!({"name": "deep", "entry": "init.lua", "needs": ["outer.top"]}),
	);
	let ledger = Ledger::scan(dir.path());
	assert!(
		ledger.entries().all(|e| e.unread.is_none()),
		"every document on the walk test's tree reads"
	);
	match ledger.resolve("outer/inner/deep", "outer.top") {
		Bound::One(e) => assert_eq!(e.path, "outer", "the walk ends at the top of the chain"),
		other => panic!("expected the outward binding, got {:?}", other.paths()),
	}
	// The same ask as the registry makes it: a declared need, bound outward.
	assert!(ledger.bindings().iter().any(|(e, key, b)| {
		e.path == "outer/inner/deep" && *key == "outer.top" && b.paths() == vec!["outer"]
	}));
}

/// The sibling case, both sides of it. A nested cartridge's `provide` is seen
/// by every lookup from inside its parent's subtree — the walk passes the
/// parent's scope — so `outer/sib` binds `store.get` straight to
/// `outer/inner`'s provide with no re-export anywhere. And the graph outside
/// the parent does not see it: a top-level asker binds to whatever the root
/// scope offers, and here that is nothing. This is the settled reading of
/// "inner cartridges are hidden until passed on" — hidden from outside the
/// parent, not from the siblings within it.
#[test]
fn a_child_provide_is_seen_by_its_parent_subtree_and_not_by_the_graph_outside() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"outer",
		json!({"name": "outer", "entry": "init.lua"}),
	);
	cartridge(
		dir.path(),
		"outer/inner",
		json!({"name": "inner", "entry": "init.lua", "provide": ["store.get"]}),
	);
	cartridge(
		dir.path(),
		"outer/sib",
		json!({"name": "sib", "entry": "init.lua", "needs": ["store.get"]}),
	);
	cartridge(
		dir.path(),
		"stranger",
		json!({"name": "stranger", "entry": "init.lua", "needs": ["store.get"]}),
	);
	let ledger = Ledger::scan(dir.path());
	match ledger.resolve("outer/sib", "store.get") {
		Bound::One(e) => assert_eq!(
			e.path, "outer/inner",
			"the sibling binds the nested provide"
		),
		other => panic!("expected the sibling binding, got {:?}", other.paths()),
	}
	// A lookup from the parent's scope binds the same key — the parent is
	// inside its own subtree, the walk passes its scope too.
	match ledger.resolve("outer", "store.get") {
		Bound::One(e) => assert_eq!(e.path, "outer/inner"),
		other => panic!("expected the parent binding, got {:?}", other.paths()),
	}
	// Outside the parent's subtree the key never arrived: the root scope
	// offers nothing, so both the unrelated top-level asker and a lookup at
	// the root itself read `?`.
	assert!(matches!(
		ledger.resolve("stranger", "store.get"),
		Bound::None
	));
	assert!(matches!(ledger.resolve("", "store.get"), Bound::None));
	// The registry says the same thing on the same tree: the sibling's need is
	// bound, the stranger's is named with `?`.
	let bound = |path: &str| {
		ledger
			.bindings()
			.iter()
			.find(|(e, key, _)| e.path == path && *key == "store.get")
			.map(|(_, _, b)| {
				b.paths()
					.iter()
					.map(|p| p.to_string())
					.collect::<Vec<String>>()
			})
	};
	assert_eq!(bound("outer/sib"), Some(vec!["outer/inner".to_string()]));
	assert_eq!(bound("stranger"), Some(Vec::<String>::new()));
}

/// A document the host refuses — a re-export nothing inside the cartridge
/// offers — is unread on the ledger's read too, with the host's reason. The
/// ledger's verdict and the host's are the same verdict, so `cartridge ledger`
/// exits non-zero on a tree the host refuses to load.
#[test]
fn a_dangling_re_export_is_unread_on_the_ledger_read_too() {
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
	let ledger = Ledger::scan(dir.path());
	let outer = ledger.get("outer").unwrap();
	assert!(
		outer
			.unread
			.as_deref()
			.unwrap()
			.contains("`store.get` is passed on, but nothing inside this cartridge offers it"),
		"the host's reason, carried verbatim"
	);
	assert!(outer.provide.is_empty() && outer.export.is_empty() && outer.needs.is_empty());
	assert_eq!(outer.name, "");
	// The child below it still reads: the refusal is about `outer`'s document.
	assert!(ledger.get("outer/inner").unwrap().unread.is_none());
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
	assert_eq!(
		entry.provide.len() + entry.export.len() + entry.needs.len(),
		0
	);
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
	write(
		dir.path(),
		"fresh/init.lua",
		r#"error("a fresh cartridge must not run")"#,
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let listed = host.manifest().unwrap();
	let fresh = listed.iter().find(|c| c.entry.id == "fresh").unwrap();
	assert!(fresh.entry.disabled, "available, not started");
	assert!(fresh.provide.is_empty() && fresh.inject.is_empty());
	assert!(host.fiber_of("fresh").is_none(), "nothing of it runs");
	// Uninstalling is taking it away: the entry goes with the folder.
	std::fs::remove_dir_all(dir.path().join("fresh")).unwrap();
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	assert!(!host
		.manifest()
		.unwrap()
		.iter()
		.any(|c| c.entry.id == "fresh"));
}
