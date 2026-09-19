//! The doors to ASP: the host method, the service key and tool.asp.

use serde_json::{json, Value};

use super::*;

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
