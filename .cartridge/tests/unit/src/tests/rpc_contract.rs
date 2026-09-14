//! One behavioral corpus through the actual host, Lua bridge and process peers.
use super::{boot, next_event, process::sdk_fixture, write};
use serde_json::{json, Value};
use std::time::Duration;

fn cases() -> Vec<Value> {
	serde_json::from_str(include_str!("fixtures/rpc/cases.json")).unwrap()
}

fn peer(language: &str) -> Vec<String> {
	match language {
		"rust" => vec![sdk_fixture().to_string_lossy().into_owned()],
		_ => unreachable!(),
	}
}

async fn fixture(language: &str) -> (tempfile::TempDir, std::sync::Arc<crate::lua::Host>) {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "lua.lua", include_str!("fixtures/rpc/echo.lua"));
	let command = peer(language);
	write(
		dir.path(),
		"peer.lua",
		&format!(
			"return cartridge.process({{{}}})",
			command
				.iter()
				.map(|s| json!(s).to_string())
				.collect::<Vec<_>>()
				.join(",")
		),
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="lua",path="lua.lua"},{id="peer",path="peer.lua"}}"#,
	);
	let (host, _) = boot(dir.path()).await;
	host.fiber_of("lua").unwrap().settled().await;
	tokio::time::timeout(
		Duration::from_secs(10),
		host.fiber_of("peer").unwrap().settled(),
	)
	.await
	.unwrap();
	assert_eq!(
		host.fiber_of("peer").unwrap().state(),
		Some(crate::runtime::State::Active),
		"{language}: {}",
		host.manifest()
			.unwrap()
			.iter()
			.map(|entry| entry.entry.id.as_str())
			.collect::<Vec<_>>()
			.join(",")
	);
	(dir, host)
}

async fn call(host: &crate::lua::Host, key: &str, value: Value) -> Result<Value, String> {
	let result = tokio::time::timeout(Duration::from_secs(5), host.call(key, value))
		.await
		.expect("contract peer did not settle within the test deadline");
	result.map_err(String::from)
}

async fn roundtrips(language: &str) {
	let (_dir, host) = fixture(language).await;
	// No setup RPC consumes request IDs: the first host call and callback both
	// start at 1. Both directions remain live while the peer awaits Lua.
	let values = cases();
	let results = futures::future::join_all(
		values
			.iter()
			.map(|case| call(&host, "roundtrip", case["value"].clone())),
	)
	.await;
	for (case, result) in values.iter().zip(results) {
		assert_eq!(
			result.unwrap(),
			json!({"data":case["value"],"meta":null,"absent":null}),
			"{language}: {}",
			case["name"]
		);
		assert_eq!(
			call(&host, "lua", case["value"].clone()).await.unwrap(),
			case["value"],
			"direct Lua: {}",
			case["name"]
		);
	}
	assert!(call(&host, "roundtrip", json!("error"))
		.await
		.unwrap_err()
		.contains("contract service failed"));
	// A failed call must not poison the next ID or confuse false with failure.
	assert_eq!(
		call(&host, "roundtrip", json!(false)).await.unwrap()["data"],
		false
	);
	host.fiber_of("peer").unwrap().dispose().await;
	assert!(call(&host, "roundtrip", Value::Null).await.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn rust_sdk_and_lua_share_json_and_bidirectional_contracts() {
	roundtrips("rust").await;
}

async fn replacement(language: &str) {
	let (dir, host) = fixture(language).await;
	write(dir.path(), "config.lua", r#"return {peer={generation=1}}"#);
	host.reconcile().await.unwrap();
	assert_eq!(
		call(&host, "roundtrip", json!("config")).await.unwrap(),
		json!({"generation":1})
	);
	// The fixture rejects its apply, so the candidate never publishes and the
	// previous generation must be retained.
	write(
		dir.path(),
		"config.lua",
		r#"return {peer={generation=2,reject=true}}"#,
	);
	host.reconcile().await.unwrap();
	assert_eq!(
		call(&host, "roundtrip", json!("config")).await.unwrap(),
		json!({"generation":1})
	);
	write(dir.path(), "config.lua", r#"return {peer={generation=3}}"#);
	host.reconcile().await.unwrap();
	assert_eq!(
		call(&host, "roundtrip", json!("config")).await.unwrap(),
		json!({"generation":3})
	);
	host.fiber_of("peer").unwrap().dispose().await;
	assert!(call(&host, "roundtrip", Value::Null).await.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn rust_sdk_failed_apply_retains_generation_then_reload_and_dispose_work() {
	replacement("rust").await;
}

async fn eof(language: &str) {
	let (_dir, host) = fixture(language).await;
	let mut events = host.outbox();
	let pending = call(&host, "roundtrip", json!("pending"));
	tokio::pin!(pending);
	assert!(futures::poll!(&mut pending).is_pending());
	assert_eq!(next_event(&mut events, "pending").await, true);
	assert_eq!(
		call(&host, "roundtrip", json!("exit")).await.unwrap_err(),
		"cartridge is gone"
	);
	assert_eq!(pending.await.unwrap_err(), "cartridge is gone");
	host.fiber_of("peer").unwrap().dispose().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn rust_sdk_eof_rejects_another_inflight_call() {
	eof("rust").await;
}
