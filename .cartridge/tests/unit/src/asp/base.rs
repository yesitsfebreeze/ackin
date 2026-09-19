//! What the base itself contributes.

use serde_json::json;

use super::*;

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
	assert!(owner["edges"]
		.as_array()
		.unwrap()
		.iter()
		.any(|edge| edge["to"] == "tool:read"));
	assert!(owner["edges"]
		.as_array()
		.unwrap()
		.iter()
		.any(|edge| edge["to"] == "event:tool.read"));
	let reader = &owner["nodes"][0];
	assert_eq!(reader["id"], "cartridge:reader");
	assert_eq!(reader["attributes"]["host.state"], "active");
	let started = reader["contributors"][0]["revision"].clone();
	assert_eq!(started, "1", "the first start is generation 1");

	host.replace("reader").await.unwrap();
	let restarted = expand(&host, "cartridge:reader").await;
	assert_ne!(
		restarted["nodes"][0]["contributors"][0]["revision"], started,
		"a restart moves the cartridge's revision"
	);

	let found = host
		.asp(json!({"op": "search", "query": "read a workspace file"}))
		.await
		.unwrap();
	assert!(
		found["hits"]
			.as_array()
			.unwrap()
			.iter()
			.any(|hit| hit["node"]["id"] == "tool:read"),
		"{found}"
	);
	assert_eq!(
		found["hits"].as_array().unwrap().len(),
		2,
		"asp itself does not match"
	);
	host.stop().await;
}
