use std::time::Duration;

use super::{boot, write};
use crate::socket::{self, Client};
use serde_json::{json, Value};

/// Next frame from the client satisfying `pick`.
async fn until(client: &mut Client, pick: impl Fn(&Value) -> bool) -> Value {
	loop {
		let m = tokio::time::timeout(Duration::from_secs(2), client.next())
			.await
			.unwrap()
			.unwrap();
		if pick(&m) {
			return m;
		}
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_round_trips_through_a_cartridge() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"echo.lua",
		r#"return { apply = function(ctx) ctx:on("say", function(d) ctx:send("echo", d) end) end }"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return { { id = "echo", path = "echo.lua" } }"#,
	);
	let (host, _) = boot(dir.path()).await;
	let path = socket::path(dir.path());
	let server = tokio::spawn({
		let path = path.clone();
		async move { socket::serve(host, &path).await }
	});
	let mut client = loop {
		if let Ok(c) = Client::connect(&path).await {
			break c;
		}
		tokio::time::sleep(Duration::from_millis(20)).await;
	};
	client
		.send(json!({ "emit": "say", "data": { "text": "hi" } }))
		.await
		.unwrap();
	let reply = until(&mut client, |m| m["event"] == "echo").await;
	assert_eq!(reply["data"], json!({ "text": "hi" }));
	client.send(json!({ "status": true })).await.unwrap();
	let status = until(&mut client, |m| m.get("status").is_some()).await;
	assert!(status["status"]
		.as_array()
		.unwrap()
		.iter()
		.any(|f| f["name"] == "echo" && f["state"] == "Active"));
	server.abort();
	let _ = std::fs::remove_file(path);
}

#[tokio::test(flavor = "multi_thread")]
async fn socket_calls_a_lua_wrapped_sdk_process_with_correlated_errors() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"provider.lua",
		r#"return {provide={"lua"},apply=function(ctx)
		ctx:provide("lua", function(args) return args end)
	end}"#,
	);
	write(
		dir.path(),
		"child.lua",
		&format!(
			"return zirkle.process({})",
			json!(super::process::sdk_fixture())
		),
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="provider",path="provider.lua"},{id="child",path="child.lua"}}"#,
	);
	let (host, _) = boot(dir.path()).await;
	host.fiber_of("child").unwrap().settled().await;
	let path = socket::path(dir.path());
	let server = tokio::spawn({
		let host = host.clone();
		let path = path.clone();
		async move { socket::serve(host, &path).await }
	});
	let mut client = tokio::time::timeout(Duration::from_secs(5), async {
		loop {
			if let Ok(client) = Client::connect(&path).await {
				break client;
			}
			tokio::task::yield_now().await;
		}
	})
	.await
	.unwrap();
	client
		.send(json!({"call":"roundtrip","args":42,"id":"success"}))
		.await
		.unwrap();
	assert_eq!(
		until(&mut client, |m| m["reply"] == "success").await,
		json!({"reply":"success","data":{"data":42,"meta":null,"absent":null}})
	);
	client
		.send(json!({"call":"missing","args":null,"id":"failure"}))
		.await
		.unwrap();
	let error = until(&mut client, |m| m["reply"] == "failure").await;
	assert!(error["error"].is_string());
	host.fiber_of("child").unwrap().dispose().await;
	drop(client);
	server.abort();
	let _ = server.await;
	std::fs::remove_file(path).unwrap();
}

/// One turn id survives every hop a call makes: the socket names it, the host
/// puts it on the wire, the cartridge process reads it off its own frame, and
/// the Lua service that cartridge calls back into reports the same id.
#[tokio::test(flavor = "multi_thread")]
async fn one_turn_id_crosses_the_socket_the_host_a_cartridge_and_lua() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"provider.lua",
		r#"return {provide={"lua"},apply=function(ctx)
		ctx:provide("lua", function(args)
			if args == "turn" then return zirkle.turn() end
			return args
		end)
	end}"#,
	);
	write(
		dir.path(),
		"child.lua",
		&format!(
			"return zirkle.process({})",
			json!(super::process::sdk_fixture())
		),
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="provider",path="provider.lua"},{id="child",path="child.lua"}}"#,
	);
	let (host, _) = boot(dir.path()).await;
	host.fiber_of("child").unwrap().settled().await;
	let path = socket::path(dir.path());
	let server = tokio::spawn({
		let host = host.clone();
		let path = path.clone();
		async move { socket::serve(host, &path).await }
	});
	let mut client = tokio::time::timeout(Duration::from_secs(5), async {
		loop {
			if let Ok(client) = Client::connect(&path).await {
				break client;
			}
			tokio::task::yield_now().await;
		}
	})
	.await
	.unwrap();
	client
		.send(json!({"call":"roundtrip","args":"turn","id":"named","turn":"turn-probe-1"}))
		.await
		.unwrap();
	assert_eq!(
		until(&mut client, |m| m["reply"] == "named").await["data"],
		json!({"cartridge":"turn-probe-1","lua":"turn-probe-1"})
	);
	// A client that names no turn still gets one, and the same one everywhere.
	client
		.send(json!({"call":"roundtrip","args":"turn","id":"minted"}))
		.await
		.unwrap();
	let minted = until(&mut client, |m| m["reply"] == "minted").await;
	let seen = minted["data"]["cartridge"].as_str().unwrap().to_owned();
	assert!(!seen.is_empty() && seen != "turn-probe-1");
	assert_eq!(minted["data"]["lua"], json!(seen));
	host.fiber_of("child").unwrap().dispose().await;
	drop(client);
	server.abort();
	let _ = server.await;
	let _ = std::fs::remove_file(path);
}
