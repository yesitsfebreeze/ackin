//! Actions run through their tool, and never around policy.

use serde_json::json;

use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn an_action_runs_through_its_tool_and_a_denial_reaches_the_caller_unchanged() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	cartridge(
		dir.path(),
		"guard",
		json!({"name": "guard", "entry": "init.lua", "events": {"tool.open": {}}, "listen": ["tool.open"]}),
		r#"cartridge.listen("tool.open", function(args)
			if args.input.path == "secret.rs" then error("policy: tool.open is denied for secret.rs", 0) end
			return "opened " .. args.input.path
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
		.bail(
			"tool.open",
			json!({"op": "call", "input": {"path": "secret.rs"}}),
		)
		.await
		.unwrap_err()
		.to_string();
	assert!(direct.contains("policy: tool.open is denied"), "{direct}");
	assert_eq!(through_asp, direct);
	host.stop().await;
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
