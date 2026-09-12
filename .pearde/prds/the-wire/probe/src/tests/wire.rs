//! Recursion over the process wire: a cartridge that hosts cartridges over the
//! same wire it is itself hosted on. The daemon above a sub-host never learns
//! that it nested — every frame it sees still names only the sub-host.
use super::{boot, next_event, settle, write};
use serde_json::json;

/// A cartridge that hosts `rpc_fixture` as its own child.
fn nested_fixture() -> std::path::PathBuf {
	static BINARY: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	BINARY
		.get_or_init(|| super::built(&["-p", "zirkle", "--example", "nested_fixture"]))
		.clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cartridge_hosts_a_cartridge_over_the_same_wire() {
	let parent = nested_fixture();
	let child = super::process::sdk_fixture();
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"lua.lua",
		r#"return { provide = {"lua"}, apply = function(ctx)
		ctx:provide("lua", function(args)
			if args == "error" then error("lua service failed") end
			return { value = args }
		end)
	end }"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{ id = "lua", path = "lua.lua" }}"#,
	);
	let (host, mut rx) = boot(dir.path()).await;
	settle().await;
	let component = crate::cartridge::component(
		host.clone(),
		"parent".into(),
		vec![parent.to_string_lossy().into_owned()],
		json!({ "child": [child.to_string_lossy()], "lifecycle": true }),
	)
	.unwrap();
	let fiber = host
		.runtime()
		.ctx()
		.intercept("lua", json!({"description": "Lua service"}))
		.cartridge(component);
	fiber.settled().await;
	let call = |args| {
		tokio::time::timeout(
			std::time::Duration::from_secs(5),
			host.call("roundtrip", args),
		)
	};
	// Three wire hops on one protocol: the daemon calls the parent, the parent
	// forwards to its child, the child calls the injected key back up through
	// the parent. The answer is the child's, assembled from keys the daemon
	// injected into the parent.
	assert_eq!(
		call(json!(21)).await.unwrap().unwrap(),
		json!({"data": {"value": 21}, "meta": {"description": "Lua service"}, "absent": null})
	);
	// An event the child listens for arrives through the parent, and the
	// child's `send` leaves through the parent to the socket audience.
	host.emit("probe", json!(1));
	assert_eq!(next_event(&mut rx, "observed").await, json!(1));
	// Disposing the sub-host disposes the child: the child's own finalizer
	// runs, and its farewell still reaches the socket through the parent.
	fiber.dispose().await;
	assert_eq!(next_event(&mut rx, "disposed").await, json!(true));
	assert!(call(json!(21)).await.unwrap().is_err());
	settle().await;
}