use std::path::Path;
use std::sync::Arc;

use serde_json::{json, Value};

use crate::host::{Host, State};
use crate::tests::{built, home, write};

fn cartridge(dir: &Path, id: &str, manifest: Value, init: &str) {
	write(dir, &format!("{id}/cartridge.json"), &manifest.to_string());
	write(dir, &format!("{id}/init.lua"), init);
}

fn descriptor(dir: &Path, ids: &[&str]) {
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

async fn boot(dir: &Path, ids: &[&str]) -> Arc<Host> {
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
fn files(dir: &Path) {
	cartridge(
		dir,
		"files",
		json!({
			"name": "files", "entry": "init.lua",
			"events": {"asp.files": {}, "bump": {}},
			"listen": ["asp.files", "bump"],
			"asp": {
				"schemes": {"file": {"description": "a workspace file", "owner": true}},
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
				return { nodes = { { id = "file:src/zeta.rs", name = "zeta.rs" }, { id = "file:src/alpha.rs", name = "alpha.rs" } } }
			end
			return { nodes = { { id = request.entity, revision = revision, name = "a.rs", attributes = { ["files.bytes"] = 12 } } } }
		end)"#,
	);
}

/// Owns `range:` and links files to ranges. It always answers against `r1`.
fn search(dir: &Path) {
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

fn ids(answer: &Value) -> Vec<&str> {
	answer["nodes"]
		.as_array()
		.unwrap()
		.iter()
		.map(|n| n["id"].as_str().unwrap())
		.collect()
}

async fn expand(host: &Host, entity: &str) -> Value {
	host.asp(json!({"op": "expand", "entity": entity}))
		.await
		.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn unloading_a_provider_removes_its_types_and_facts_and_keeps_the_rest() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	search(dir.path());
	let host = boot(dir.path(), &["files", "search"]).await;

	let types = host.asp(json!({"op": "types"})).await.unwrap();
	assert_eq!(types["schemes"]["file"]["owner"], "files");
	assert_eq!(
		types["schemes"]["file"]["contributors"],
		json!(["files", "search"])
	);
	assert_eq!(types["schemes"]["range"]["owner"], "search");
	assert_eq!(types["edges"]["matches"], json!(["search"]));

	let both = expand(&host, "file:src/a.rs").await;
	assert_eq!(ids(&both), ["file:src/a.rs", "range:src/a.rs:4:1-4:9"]);
	assert_eq!(both["edges"][0]["kind"], "matches");
	assert_eq!(both["edges"][0]["contributor"], "search");
	assert_eq!(both["nodes"][0]["attributes"]["files.bytes"], 12);

	descriptor(dir.path(), &["files"]);
	host.reconcile().await.unwrap();

	let types = host.asp(json!({"op": "types"})).await.unwrap();
	assert!(types["schemes"].get("range").is_none(), "{types}");
	assert!(types["edges"].get("matches").is_none(), "{types}");
	let alone = expand(&host, "file:src/a.rs").await;
	assert_eq!(ids(&alone), ["file:src/a.rs"]);
	assert_eq!(alone["edges"], json!([]));
	let refused = host
		.asp(json!({"op": "expand", "entity": "range:src/a.rs:4:1-4:9"}))
		.await
		.unwrap_err();
	assert!(
		refused.contains("no loaded cartridge declares"),
		"{refused}"
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_node_two_providers_assert_survives_one_of_them() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	cartridge(
		dir.path(),
		"mirror",
		json!({
			"name": "mirror", "entry": "init.lua",
			"events": {"asp.mirror": {}}, "listen": ["asp.mirror"],
			"asp": {"schemes": {"file": {}}},
		}),
		r#"cartridge.listen("asp.mirror", function(request) return { nodes = { { id = request.entity, name = "mirrored" } } } end)"#,
	);
	let host = boot(dir.path(), &["files", "mirror"]).await;
	let contributors = |answer: &Value| -> Vec<String> {
		answer["nodes"][0]["contributors"]
			.as_array()
			.unwrap()
			.iter()
			.map(|c| c["contributor"].as_str().unwrap().to_owned())
			.collect()
	};
	let both = expand(&host, "file:src/a.rs").await;
	assert_eq!(contributors(&both), ["files", "mirror"]);
	assert_eq!(
		both["nodes"][0]["name"], "a.rs",
		"the owner names the entity"
	);

	descriptor(dir.path(), &["mirror"]);
	host.reconcile().await.unwrap();
	let one = expand(&host, "file:src/a.rs").await;
	assert_eq!(ids(&one), ["file:src/a.rs"]);
	assert_eq!(contributors(&one), ["mirror"]);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fact_behind_the_owners_revision_is_marked_stale() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	search(dir.path());
	let host = boot(dir.path(), &["files", "search"]).await;
	let fresh = expand(&host, "file:src/a.rs").await;
	assert!(fresh["edges"][0].get("stale").is_none(), "{fresh}");

	host.bail("bump", json!(null)).await.unwrap();
	let moved = expand(&host, "file:src/a.rs").await;
	assert_eq!(moved["edges"][0]["stale"], true, "{moved}");
	assert_eq!(moved["edges"][0]["revision"], "r1");
	let range = &moved["nodes"][1];
	assert_eq!(range["contributors"][0]["stale"], true, "{moved}");
	let file = &moved["nodes"][0];
	assert_eq!(file["contributors"][0]["revision"], "r2");
	assert!(file["contributors"][0].get("stale").is_none(), "{moved}");
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_undeclared_edge_kind_refuses_the_provider_by_name() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	cartridge(
		dir.path(),
		"rogue",
		json!({
			"name": "rogue", "entry": "init.lua",
			"events": {"asp.rogue": {}}, "listen": ["asp.rogue"],
			"asp": {"schemes": {"file": {}}, "edges": {"declared": {}}},
		}),
		r#"cartridge.listen("asp.rogue", function(request)
			return { nodes = { { id = "file:src/b.rs" } }, edges = { { from = request.entity, to = "file:src/b.rs", kind = "imports" } } }
		end)"#,
	);
	let host = boot(dir.path(), &["files", "rogue"]).await;
	let answer = expand(&host, "file:src/a.rs").await;
	assert_eq!(
		ids(&answer),
		["file:src/a.rs"],
		"nothing of rogue's is kept"
	);
	let rogue = answer["sources"]
		.as_array()
		.unwrap()
		.iter()
		.find(|s| s["contributor"] == "rogue")
		.unwrap();
	assert_eq!(rogue["state"], "refused");
	let error = rogue["error"].as_str().unwrap();
	assert!(
		error.contains("`rogue`") && error.contains("`imports`"),
		"{error}"
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_action_runs_through_its_tool_and_a_denial_reaches_the_caller_unchanged() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	cartridge(
		dir.path(),
		"guard",
		json!({"name": "guard", "entry": "init.lua", "events": {"tool.open": {}}, "listen": ["tool.open"]}),
		r#"cartridge.listen("tool.open", function(args)
			if args.path == "secret.rs" then error("policy: tool.open is denied for secret.rs", 0) end
			return "opened " .. args.path
		end)"#,
	);
	let host = boot(dir.path(), &["files", "guard"]).await;
	let listed = host
		.asp(json!({"op": "actions", "entity": "file:src/a.rs"}))
		.await
		.unwrap();
	assert_eq!(
		listed["actions"],
		json!([{"name": "open", "contributor": "files", "effect": "read",
			"tool": "tool.open", "args": {"path": "src/a.rs"}}])
	);
	let ran = host
		.asp(json!({"op": "act", "entity": "file:src/a.rs", "action": "open"}))
		.await
		.unwrap();
	assert_eq!(ran, json!("opened src/a.rs"));

	let through_asp = host
		.asp(json!({"op": "act", "entity": "file:secret.rs", "action": "open"}))
		.await
		.unwrap_err()
		.to_string();
	let direct = host
		.bail("tool.open", json!({"path": "secret.rs"}))
		.await
		.unwrap_err()
		.to_string();
	assert!(direct.contains("policy: tool.open is denied"), "{direct}");
	assert_eq!(through_asp, direct);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn search_ranks_what_the_providers_found_with_the_fabrics_formula() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	let host = boot(dir.path(), &["files"]).await;
	let answer = host
		.asp(json!({"op": "search", "query": "alpha"}))
		.await
		.unwrap();
	let hits = answer["hits"].as_array().unwrap();
	assert_eq!(hits[0]["node"]["id"], "file:src/alpha.rs", "{answer}");
	assert!(hits[0]["score"].as_f64().unwrap() > 0.0);
	assert_eq!(hits[1]["node"]["id"], "file:src/zeta.rs");
	assert_eq!(hits[1]["why"], "provider match");
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cartridge_reaches_asp_through_the_host() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	cartridge(
		dir.path(),
		"reader",
		json!({"name": "reader", "entry": "init.lua", "events": {"look": {}}, "listen": ["look"]}),
		r#"cartridge.listen("look", function() return cartridge.host("asp", { op = "expand", entity = "file:src/a.rs" }) end)"#,
	);
	let host = boot(dir.path(), &["files", "reader"]).await;
	let served = tokio::spawn(crate::host::socket::serve(host.clone()));
	let answer = host.bail("look", json!(null)).await.unwrap().unwrap();
	assert_eq!(ids(&answer), ["file:src/a.rs"]);
	let on_the_service_key = host
		.bail(crate::asp::SERVICE, json!({"op": "types"}))
		.await
		.unwrap()
		.unwrap();
	assert_eq!(on_the_service_key["schemes"]["file"]["owner"], "files");
	host.stop().await;
	served.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cartridge_that_needs_tools_finds_asp_among_them() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	cartridge(
		dir.path(),
		"agent",
		json!({"name": "agent", "entry": "init.lua", "events": {"go": {}}, "listen": ["go"], "needs": ["tool.*"]}),
		r#"cartridge.listen("go", function()
			return {
				needs = cartridge.needs(),
				owner = cartridge.events()["tool.asp"].owner,
				described = cartridge.bail("tool.asp", { op = "describe" }),
				expanded = cartridge.bail("tool.asp", { op = "call", input = { op = "expand", entity = "file:src/a.rs" } }),
			}
		end)"#,
	);
	let host = boot(dir.path(), &["files", "agent"]).await;
	let served = tokio::spawn(crate::host::socket::serve(host.clone()));
	let answer = host.bail("go", json!(null)).await.unwrap().unwrap();
	assert_eq!(answer["needs"], json!(["tool.asp"]));
	assert_eq!(answer["owner"], "host");
	assert_eq!(answer["described"]["name"], "asp");
	assert_eq!(answer["expanded"]["error"], false, "{answer}");
	let world: Value =
		serde_json::from_str(answer["expanded"]["content"].as_str().unwrap()).unwrap();
	assert_eq!(ids(&world), ["file:src/a.rs"]);
	assert_eq!(world["actions"][0]["tool"], "tool.open");
	host.stop().await;
	served.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cartridge_cannot_run_an_action_through_asp() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	cartridge(
		dir.path(),
		"guard",
		json!({"name": "guard", "entry": "init.lua", "events": {"tool.open": {}}, "listen": ["tool.open"]}),
		r#"opened = 0
		cartridge.listen("tool.open", function() opened = opened + 1 return opened end)"#,
	);
	cartridge(
		dir.path(),
		"agent",
		json!({"name": "agent", "entry": "init.lua", "events": {"go": {}}, "listen": ["go"], "needs": ["tool.asp"]}),
		r#"cartridge.listen("go", function()
			local act = { op = "act", entity = "file:src/a.rs", action = "open" }
			local ok, refused = pcall(cartridge.host, "asp", act)
			return { host_ok = ok, host_refused = tostring(refused), tool = cartridge.bail("tool.asp", { op = "call", input = act }) }
		end)"#,
	);
	let host = boot(dir.path(), &["files", "guard", "agent"]).await;
	let served = tokio::spawn(crate::host::socket::serve(host.clone()));
	let answer = host.bail("go", json!(null)).await.unwrap().unwrap();
	assert_eq!(answer["host_ok"], false, "{answer}");
	assert!(
		answer["host_refused"]
			.as_str()
			.unwrap()
			.contains("not granted"),
		"{answer}"
	);
	assert_eq!(answer["tool"]["error"], true, "{answer}");
	let count = host.bail("tool.open", json!({})).await.unwrap().unwrap();
	assert_eq!(count, 1, "neither refused path reached the tool");
	host.stop().await;
	served.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_base_contributes_every_tool_and_cartridge_and_lists_every_event_as_a_type() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"reader",
		json!({
			"name": "reader", "entry": "init.lua",
			"events": {"tool.read": {"description": "read a file from the workspace", "schema": {"type": "object"}}},
			"listen": ["tool.read"],
		}),
		r#"cartridge.listen("tool.read", function() return "" end)"#,
	);
	let host = boot(dir.path(), &["reader"]).await;

	let types = host.asp(json!({"op": "types"})).await.unwrap();
	assert_eq!(types["schemes"]["tool"]["owner"], "host");
	assert_eq!(types["events"]["tool.read"]["owner"], "reader");
	assert_eq!(
		types["events"]["tool.read"]["schema"],
		json!({"type": "object"})
	);
	assert_eq!(types["events"]["tool.asp"]["owner"], "host");

	let tool = expand(&host, "tool:read").await;
	assert_eq!(ids(&tool), ["cartridge:reader", "tool:read"]);
	assert_eq!(tool["edges"][0]["kind"], "provides");
	assert_eq!(tool["edges"][0]["contributor"], "host");
	let owner = expand(&host, "cartridge:reader").await;
	assert_eq!(owner["edges"][0]["to"], "tool:read");

	let found = host
		.asp(json!({"op": "search", "query": "read a workspace file"}))
		.await
		.unwrap();
	assert_eq!(found["hits"][0]["node"]["id"], "tool:read", "{found}");
	assert_eq!(
		found["hits"].as_array().unwrap().len(),
		1,
		"asp itself does not match"
	);
	host.stop().await;
}

#[test]
fn a_manifest_refuses_an_asp_block_it_cannot_serve() {
	let dir = tempfile::tempdir().unwrap();
	let base = |asp: Value, listen: Value| json!({"name": "p", "entry": "init.lua", "events": {"asp.p": {}, "asp.q": {}}, "listen": listen, "asp": asp});
	let refused = [
		base(json!({"schemes": {"file": {}}}), json!([])),
		base(json!({"schemes": {"file": {}}}), json!(["asp.p", "asp.q"])),
		base(json!({"schemes": {"File:": {}}}), json!(["asp.p"])),
		base(json!({"attributes": {"other.size": {}}}), json!(["asp.p"])),
		base(json!({"colours": {}}), json!(["asp.p"])),
		json!({"name": "p", "entry": "init.lua", "events": {"asp": {}}}),
	];
	for manifest in refused {
		write(dir.path(), "cartridge.json", &manifest.to_string());
		assert!(
			crate::loader::Cartridge::document(&dir.path().join("cartridge.json")).is_err(),
			"{manifest}"
		);
	}
	let accepted = base(
		json!({"schemes": {"file": {}}, "attributes": {"p.size": {}}}),
		json!(["asp.p"]),
	);
	write(dir.path(), "cartridge.json", &accepted.to_string());
	write(dir.path(), "init.lua", "");
	crate::loader::Cartridge::document(&dir.path().join("cartridge.json")).unwrap();
}
