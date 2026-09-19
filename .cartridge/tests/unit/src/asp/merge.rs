//! Merging: staleness against the owner, and refusal of undeclared facts.

use serde_json::json;

use super::*;

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
