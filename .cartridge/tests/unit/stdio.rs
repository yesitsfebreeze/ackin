use super::*;

#[test]
fn mcp_bridge_failure_preserves_request_ids_without_replay() {
	for id in [json!(7), json!("request-a")] {
		let line = json!({"jsonrpc":"2.0","id":id,"method":"tools/list"}).to_string();
		let reply =
			mcp_bridge_reply(&line, Err(cartridge::Error::NotProvided("mcp".into()))).unwrap();
		assert_eq!(reply["id"], id);
		assert_eq!(reply["jsonrpc"], "2.0");
		assert_eq!(reply["error"]["code"], -32603);
		assert_eq!(reply["error"]["message"], "`mcp` is not provided");
	}
}

#[test]
fn mcp_bridge_keeps_notifications_silent_and_successes_intact() {
	for value in [
		json!({"method":"notifications/initialized"}),
		json!({"method":"notifications/cancelled","id":null}),
		json!({"id":7,"result":{}}),
	] {
		assert!(mcp_bridge_reply(
			&value.to_string(),
			Err(cartridge::Error::Remote("unavailable".into()))
		)
		.is_none());
	}
	let success = json!({"jsonrpc":"2.0","id":7,"result":{"tools":[]}});
	assert_eq!(
		mcp_bridge_reply("request", Ok(success.clone())),
		Some(success)
	);
	assert!(mcp_bridge_reply("notification", Ok(Value::Null)).is_none());
	let parse = mcp_bridge_reply(
		"not-json",
		Err(cartridge::Error::Remote("unavailable".into())),
	)
	.unwrap();
	assert_eq!(parse["error"]["code"], -32700);
	assert!(parse["id"].is_null());
}

#[test]
fn only_trust_refusals_prompt_an_attach_reload() {
	let status = json!([
		{"id":"proxy","state":"failed","error":"/p/init.lua: has changed since it was trusted; review it"},
		{"id":"broken","state":"failed","error":"init.lua:3: syntax error"},
		{"id":"memo","state":"active"}
	]);
	let ids: Vec<_> = untrusted(&status).iter().map(|s| s["id"].clone()).collect();
	assert_eq!(ids, [json!("proxy")]);
	assert!(untrusted(&json!([{"id":"memo","state":"active"}])).is_empty());
}

#[test]
fn settled_when_the_key_is_active() {
	let status = json!([
		{"id":"memo","state":"active","listen":["memo"]},
		{"id":"mcp","state":"active","listen":["mcp","tools"]},
		{"id":"slow","state":"starting","listen":[]}
	]);
	assert!(settled(&status, "mcp"));
}

#[test]
fn settled_not_while_the_key_is_starting() {
	// The defect: the composition holds one picture while `mcp` is still
	// starting. The decision is a pure function of that picture, so seeing it
	// again is the same answer — there is no count that could end the wait.
	let status = json!([
		{"id":"memo","state":"active","listen":["memo"]},
		{"id":"mcp","state":"starting","listen":["mcp"]},
		{"id":"prd","state":"waiting","listen":["prd"]}
	]);
	assert!(!settled(&status, "mcp"));
	assert!(!settled(&status, "mcp"));
}

#[test]
fn settled_when_nothing_is_left_starting() {
	// `mcp` is served by nobody and nothing can change that, so the caller
	// fails fast and `explain` names the cause.
	let status = json!([
		{"id":"memo","state":"active","listen":["memo"]},
		{"id":"broken","state":"failed","error":"init.lua:3: syntax error"}
	]);
	assert!(settled(&status, "mcp"));
}

#[test]
fn settled_not_on_an_empty_composition() {
	assert!(!settled(&json!([]), "mcp"));
	assert!(!settled(&Value::Null, "mcp"));
	assert!(!settled(&json!({"id":"mcp","state":"active"}), "mcp"));
}
