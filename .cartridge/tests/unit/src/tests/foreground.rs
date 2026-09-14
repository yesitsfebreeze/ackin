use super::*;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn foreground_call_returns_the_service_value_and_disposes_the_profile() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "init.lua", r#"return {{id="p",path="p.lua"}}"#);
	write(
		dir.path(),
		"p.lua",
		r#"return {provide={"example"},apply=function(ctx)
		ctx:provide("example", function(args) return args end)
		coroutine.yield(function() ctx:send("disposed", true) end)
	end}"#,
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let mut rx = host.outbox();
	assert_eq!(
		host.run("example", json!({"value":42})).await.unwrap(),
		json!({"value":42})
	);
	assert_eq!(next_event(&mut rx, "disposed").await, json!(true));
	assert!(host.fiber_of("p").is_none());
	assert!(host.call("example", json!({})).await.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn foreground_missing_or_failed_service_still_disposes() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "init.lua", r#"return {{id="p",path="p.lua"}}"#);
	write(
		dir.path(),
		"p.lua",
		r#"return {provide={"example"},apply=function(ctx)
		ctx:provide("example", function() error("service failed") end)
	end}"#,
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	assert!(host
		.run("example", json!({}))
		.await
		.unwrap_err()
		.to_string()
		.contains("service failed"));
	assert!(host.fiber_of("p").is_none());
	assert!(host
		.run("missing", json!({}))
		.await
		.unwrap_err()
		.to_string()
		.contains("unavailable"));
	assert!(host.fiber_of("p").is_none());
}
