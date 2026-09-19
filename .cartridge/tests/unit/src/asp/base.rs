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
