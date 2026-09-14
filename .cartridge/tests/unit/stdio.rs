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

/// Minting is a decision, not an overwrite, and it is separate from installing
/// so `main` can `set_var` before the runtime exists.
#[test]
fn a_proxy_key_is_minted_only_when_the_environment_has_none() {
	let key = minted_proxy_key()
		.unwrap()
		.expect("no CARTRIDGE_PROXY_KEY in the test env");
	assert_eq!(key.len(), cartridge::settings::host().proxy_key_bytes * 2);
	assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
}
