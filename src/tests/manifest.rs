//! The document declares the whole cartridge: what it provides, what it needs,
//! what it passes outward from inside it, and what it asks the machine for.
//! One document, read twice — by the resolver and by the sandbox.

use super::*;
use serde_json::json;

fn cartridge(dir: &Path, folder: &str, manifest: Value, entry: &str) {
	std::fs::create_dir_all(dir.join(folder)).unwrap();
	write(
		dir,
		&format!("{folder}/cartridge.json"),
		&manifest.to_string(),
	);
	write(dir, &format!("{folder}/init.lua"), entry);
}

const INERT: &str = r#"return {apply=function(ctx) end}"#;

/// `provide` and `needs` in the document reach the component without the entry
/// repeating them, so the declaration has one home.
#[tokio::test(flavor = "multi_thread")]
async fn the_document_declares_what_it_provides_and_needs() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"p",
		json!({
			"name": "p",
			"entry": "init.lua",
			"provide": ["p.key"],
			"needs": ["other.key"],
		}),
		r#"return {apply=function(ctx) ctx:provide("p.key", function() return 1 end) end}"#,
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let component = host.component(&dir.path().join("p"), json!({})).unwrap();
	assert_eq!(component.provide, vec!["p.key".to_owned()]);
	assert_eq!(component.inject, vec!["other.key".to_owned()]);
}

/// The capability request rides the same document and is readable as data,
/// without the entry being evaluated.
#[tokio::test(flavor = "multi_thread")]
async fn the_capability_request_is_on_the_same_document() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"p",
		json!({
			"name": "p",
			"entry": "init.lua",
			"grant": {
				"read": ["data", "/usr/share/dict"],
				"write": ["cache"],
				"net": ["api.example.com"],
				"exec": ["rg"],
			},
		}),
		"error('entry evaluated')",
	);
	let (manifest, _) =
		crate::loader::Cartridge::read(&dir.path().join("p/cartridge.json")).unwrap();
	assert_eq!(manifest.grant.read, vec!["data", "/usr/share/dict"]);
	assert_eq!(manifest.grant.write, vec!["cache"]);
	assert_eq!(manifest.grant.net, vec!["api.example.com"]);
	assert_eq!(manifest.grant.exec, vec!["rg"]);
}

/// A cartridge that declares nothing asks for nothing.
#[tokio::test(flavor = "multi_thread")]
async fn an_empty_document_requests_nothing() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"p",
		json!({"name": "p", "entry": "init.lua"}),
		INERT,
	);
	let (manifest, _) =
		crate::loader::Cartridge::read(&dir.path().join("p/cartridge.json")).unwrap();
	assert!(manifest.provide.is_empty());
	assert!(manifest.needs.is_empty());
	assert!(manifest.export.is_empty());
	assert!(manifest.grant.read.is_empty());
	assert!(manifest.grant.write.is_empty());
	assert!(manifest.grant.net.is_empty());
	assert!(manifest.grant.exec.is_empty());
}

/// Every malformed declaration is refused by reading the document, before the
/// entry is evaluated.
#[tokio::test(flavor = "multi_thread")]
async fn malformed_declarations_are_refused_without_evaluating_the_entry() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"p",
		json!({"name": "p", "entry": "init.lua"}),
		"error('entry evaluated')",
	);
	for manifest in [
		json!({"name":"p","entry":"init.lua","provide":["a","a"]}),
		json!({"name":"p","entry":"init.lua","provide":["tool.*"]}),
		json!({"name":"p","entry":"init.lua","provide":[" "]}),
		json!({"name":"p","entry":"init.lua","needs":["b","b"]}),
		json!({"name":"p","entry":"init.lua","provide":["a"],"needs":["a"]}),
		json!({"name":"p","entry":"init.lua","provide":["a"],"export":["a"]}),
		json!({"name":"p","entry":"init.lua","export":["c","c"]}),
		json!({"name":"p","entry":"init.lua","provide":["a"],"selftest":"z"}),
		json!({"name":"p","entry":"init.lua","grant":{"read":["../outside"]}}),
		json!({"name":"p","entry":"init.lua","grant":{"write":[""]}}),
		json!({"name":"p","entry":"init.lua","grant":{"net":["*.example.com"]}}),
		json!({"name":"p","entry":"init.lua","grant":{"exec":[" "]}}),
		json!({"name":"p","entry":"init.lua","grant":{"sudo":true}}),
	] {
		write(dir.path(), "p/cartridge.json", &manifest.to_string());
		let error = crate::loader::Cartridge::read(&dir.path().join("p/cartridge.json"))
			.err()
			.unwrap_or_else(|| panic!("accepted {manifest}"));
		assert!(!error.contains("entry evaluated"), "{error}");
		assert!(error.contains("cartridge.json"), "{error}");
	}
}

/// Two cartridges provide the same key without colliding, so long as neither
/// subtree passes it into the other.
#[tokio::test(flavor = "multi_thread")]
async fn two_subtrees_may_provide_the_same_key() {
	let dir = tempfile::tempdir().unwrap();
	for folder in ["left", "right"] {
		cartridge(
			dir.path(),
			folder,
			json!({"name": folder, "entry": "init.lua", "provide": ["store"]}),
			r#"return {apply=function(ctx) ctx:provide("store", function() return 1 end) end}"#,
		);
	}
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	for folder in ["left", "right"] {
		let component = host.component(&dir.path().join(folder), json!({})).unwrap();
		assert_eq!(component.provide, vec!["store".to_owned()]);
	}
}

/// An inner key is visible outward only where the parent passes it on.
#[tokio::test(flavor = "multi_thread")]
async fn a_parent_passes_an_inner_key_outward_by_naming_it() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"outer",
		json!({"name": "outer", "entry": "init.lua", "export": ["inner.store"]}),
		INERT,
	);
	cartridge(
		dir.path(),
		"outer/inner",
		json!({"name": "inner", "entry": "init.lua", "provide": ["inner.store"]}),
		INERT,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="outer",path="outer"}}"#,
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let listed = host.manifest().unwrap();
	let outer = listed.iter().find(|c| c.entry.id == "outer").unwrap();
	assert_eq!(outer.export, vec!["inner.store".to_owned()]);
	// The inner cartridge is not an entry of the profile and is not visible as one.
	assert_eq!(listed.len(), 1);
}

/// The document and the Lua entry are two homes for one fact, and the document
/// wins where it speaks. An entry-only cartridge is unchanged.
#[tokio::test(flavor = "multi_thread")]
async fn the_document_overrides_what_the_entry_declares() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"p",
		json!({"name": "p", "entry": "init.lua", "provide": ["declared"], "needs": ["asked"]}),
		r#"return {provide={"undeclared"}, inject={"unasked"},
			apply=function(ctx) ctx:provide("undeclared", function() return 1 end) end}"#,
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let component = host.component(&dir.path().join("p"), json!({})).unwrap();
	// What a cartridge takes away when it goes is exactly what it declared on
	// the way in: the entry's `undeclared` and `unasked` do not survive a
	// document that names neither.
	assert_eq!(component.provide, vec!["declared".to_owned()]);
	assert_eq!(component.inject, vec!["asked".to_owned()]);
}

/// An export that names a key the subtree does not offer is refused: the second
/// naming was not paid.
#[tokio::test(flavor = "multi_thread")]
async fn an_export_that_names_nothing_inside_is_refused() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"outer",
		json!({"name": "outer", "entry": "init.lua", "export": ["nobody.provides.this"]}),
		INERT,
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let error = host
		.component(&dir.path().join("outer"), json!({}))
		.err()
		.unwrap()
		.to_string();
	assert!(error.contains("nothing inside this cartridge offers it"), "{error}");
}

/// A grandchild's keys are hidden from the grandparent unless the child passes
/// them on, so the subtree is walked one level at a time.
#[tokio::test(flavor = "multi_thread")]
async fn a_grandchild_key_reaches_the_top_only_through_every_level() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"outer/inner/deep",
		json!({"name": "deep", "entry": "init.lua", "provide": ["deep.key"]}),
		INERT,
	);
	cartridge(
		dir.path(),
		"outer/inner",
		json!({"name": "inner", "entry": "init.lua"}),
		INERT,
	);
	cartridge(
		dir.path(),
		"outer",
		json!({"name": "outer", "entry": "init.lua", "export": ["deep.key"]}),
		INERT,
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let error = host
		.component(&dir.path().join("outer"), json!({}))
		.err()
		.unwrap()
		.to_string();
	assert!(error.contains("nothing inside this cartridge offers it"), "{error}");

	// `inner` passes it on, and now `outer` may too.
	write(
		dir.path(),
		"outer/inner/cartridge.json",
		&json!({"name": "inner", "entry": "init.lua", "export": ["deep.key"]}).to_string(),
	);
	assert!(host.component(&dir.path().join("outer"), json!({})).is_ok());
}

/// PROBE: a nested cartridge is not discovered at all. The profile is still the
/// only way a cartridge enters the graph.
#[tokio::test(flavor = "multi_thread")]
async fn probe_nesting_is_not_discovered() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"outer",
		json!({"name": "outer", "entry": "init.lua", "needs": ["inner.store"], "export": ["inner.store"]}),
		INERT,
	);
	cartridge(
		dir.path(),
		"outer/inner",
		json!({"name": "inner", "entry": "init.lua", "provide": ["inner.store"]}),
		r#"return {apply=function(ctx) ctx:provide("inner.store", function() return 1 end) end}"#,
	);
	write(dir.path(), "init.lua", r#"return {{id="outer",path="outer"}}"#);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let listed = host.manifest().unwrap();
	let outer = listed.iter().find(|c| c.entry.id == "outer").unwrap();
	// `outer` needs a key its own child provides, and the need reads as unmet:
	// nothing loaded the child, so the subtree the answer describes has no
	// mechanism behind it yet.
	assert_eq!(outer.inject, vec!["inner.store".to_owned()]);
	assert!(!listed
		.iter()
		.any(|c| c.provide.iter().any(|k| k == "inner.store")));
}

/// A path is blank on the same terms a key is. `Grant::check` tested
/// `is_empty()` where the key check tested `trim().is_empty()`, so a grant of
/// `"   "` was accepted where a `provide` of `"   "` was refused — one document
/// with two ideas of what nothing is.
#[tokio::test(flavor = "multi_thread")]
async fn a_blank_grant_path_is_refused_like_a_blank_key() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"p",
		json!({"name": "p", "entry": "init.lua"}),
		"error('entry evaluated')",
	);
	let refused = |manifest: Value| -> String {
		write(dir.path(), "p/cartridge.json", &manifest.to_string());
		crate::loader::Cartridge::read(&dir.path().join("p/cartridge.json"))
			.err()
			.unwrap_or_else(|| panic!("accepted {manifest}"))
	};
	let key = refused(json!({"name":"p","entry":"init.lua","provide":["   "]}));
	assert!(key.contains("must be a nonempty exact key"), "{key}");
	for field in ["read", "write"] {
		let path = refused(json!({"name":"p","entry":"init.lua","grant":{field:["   "]}}));
		assert!(path.contains("must be a nonempty exact path"), "{path}");
		assert!(!path.contains("entry evaluated"), "{path}");
	}
}

/// The layout rule is one level and no deeper: a `cartridge.json` two
/// directories down is nested in the directory above it, not in this one. Make
/// `nested` recursive and this test fails, which is what pins the rule.
#[tokio::test(flavor = "multi_thread")]
async fn a_cartridge_two_levels_down_is_not_offered() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"outer/vendor/inner",
		json!({"name": "inner", "entry": "init.lua", "provide": ["inner.key"]}),
		INERT,
	);
	// `vendor` is a plain directory, so no level between can pass the key on and
	// `inner.key` is unreachable from `outer` by any amount of re-exporting.
	assert!(crate::loader::Cartridge::offered(&dir.path().join("outer"))
		.unwrap()
		.is_empty());
	// One level down is offered, which is the same walk seen from the inside.
	assert_eq!(
		crate::loader::Cartridge::offered(&dir.path().join("outer/vendor")).unwrap(),
		vec!["inner.key".to_owned()]
	);
	cartridge(
		dir.path(),
		"outer",
		json!({"name": "outer", "entry": "init.lua", "export": ["inner.key"]}),
		INERT,
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let error = host
		.component(&dir.path().join("outer"), json!({}))
		.err()
		.unwrap()
		.to_string();
	assert!(error.contains("nothing inside this cartridge offers it"), "{error}");
}

/// The document is data, so a disabled cartridge has still declared what it
/// asks for and the listing says so — that is what makes the request readable
/// before anything of the cartridge runs.
#[tokio::test(flavor = "multi_thread")]
async fn a_disabled_cartridge_still_declares_what_it_asked_for() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"inner",
		json!({"name": "inner", "entry": "init.lua", "provide": ["inner.key"]}),
		INERT,
	);
	cartridge(
		dir.path(),
		"outer",
		json!({
			"name": "outer",
			"entry": "init.lua",
			"export": ["inner.key"],
			"grant": {"read": ["data"], "exec": ["rg"]},
		}),
		INERT,
	);
	std::fs::rename(dir.path().join("inner"), dir.path().join("outer/inner")).unwrap();
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="outer",path="outer",disabled=true}}"#,
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let listed = host.manifest().unwrap();
	let outer = listed.iter().find(|c| c.entry.id == "outer").unwrap();
	assert!(outer.entry.disabled);
	assert_eq!(outer.export, vec!["inner.key".to_owned()]);
	let grant = outer.grant.as_ref().expect("a readable document declares");
	assert_eq!(grant.read, vec!["data".to_owned()]);
	assert_eq!(grant.exec, vec!["rg".to_owned()]);
}

/// An empty grant is the tightest policy, so a document that could not be read
/// must not produce one. It reports the error and no grant at all.
#[tokio::test(flavor = "multi_thread")]
async fn an_unreadable_document_asks_for_nothing_knowable_not_for_nothing() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"good",
		json!({"name": "good", "entry": "init.lua", "grant": {"net": ["api.example.com"]}}),
		INERT,
	);
	cartridge(
		dir.path(),
		"bad",
		json!({"name": "bad", "entry": "init.lua", "export": ["nobody.provides.this"]}),
		INERT,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="good",path="good"},{id="bad",path="bad"},{id="off",path="bad",disabled=true}}"#,
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let listed = host.manifest().unwrap();
	for id in ["bad", "off"] {
		let bad = listed.iter().find(|c| c.entry.id == id).unwrap();
		// Absent, not empty: an empty grant is the tightest policy, and a document
		// nobody could read has not asked for nothing — it has not been read.
		assert!(bad.grant.is_none(), "{id}: an unreadable document declared a grant");
		assert!(bad.export.is_empty(), "{id}");
		// Why it would not read is said, and said for the disabled entry too: the
		// document is data, so nothing had to try to load it for this to be known.
		let unread = bad.unread.as_deref().unwrap_or_default();
		assert!(
			unread.contains("nothing inside this cartridge offers it"),
			"{id}: {unread}"
		);
		// And said once. Evaluating the entry hits the same wall, and that second
		// telling is suppressed rather than printed beside the first.
		assert!(bad.error.is_none(), "{id}: {:?}", bad.error);
	}
	let good = listed.iter().find(|c| c.entry.id == "good").unwrap();
	assert_eq!(
		good.grant.as_ref().expect("readable").net,
		vec!["api.example.com".to_owned()]
	);
	assert!(good.error.is_none(), "{:?}", good.error);
	assert!(good.unread.is_none(), "{:?}", good.unread);
}

/// A file the document names being missing is a fact about the tree, not about
/// the document. The declarations still read, and the listing still reports
/// them — otherwise a cartridge whose UI has not been built yet would look like
/// a cartridge that asked for nothing.
#[tokio::test(flavor = "multi_thread")]
async fn a_missing_file_the_document_names_does_not_blank_its_declarations() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"p",
		json!({
			"name": "p",
			"entry": "init.lua",
			"ui": "ui/index.ts",
			"provide": ["p.key"],
			"grant": {"read": ["data"]},
		}),
		INERT,
	);
	write(dir.path(), "init.lua", r#"return {{id="p",path="p"}}"#);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let listed = host.manifest().unwrap();
	let p = listed.iter().find(|c| c.entry.id == "p").unwrap();
	// The document read. The `ui:` file did not exist, so loading the cartridge
	// failed — and those are reported as the two different things they are.
	assert!(p.unread.is_none(), "{:?}", p.unread);
	assert_eq!(
		p.grant.as_ref().expect("the document read").read,
		vec!["data".to_owned()]
	);
	let error = p.error.as_deref().unwrap_or_default();
	assert!(error.contains("index.ts"), "{error}");
	// And the document alone still refuses a `ui` that is not a module at all,
	// because that is a fact about the document.
	write(
		dir.path(),
		"p/cartridge.json",
		&json!({"name": "p", "entry": "init.lua", "ui": "../escape.ts"}).to_string(),
	);
	let error = crate::loader::Cartridge::document(&dir.path().join("p/cartridge.json"))
		.err()
		.unwrap();
	assert!(error.contains("ui must be a relative"), "{error}");
}
