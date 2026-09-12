use super::{boot, next_event, write};
use serde_json::json;

/// A provider whose only observable is the generation it belongs to, so a
/// switch is visible without any host-resident data behind it.
fn provider(version: u64, fail: bool) -> String {
	format!(
		r#"return {{provide={{"counter"}},apply=function(ctx)
		ctx:provide("counter", function() return {{version={version}}} end)
		{failure}
	end}}"#,
		failure = if fail {
			"error('migration rejected')"
		} else {
			""
		}
	)
}

#[tokio::test(flavor = "multi_thread")]
async fn switching_keeps_consumers_bound_and_rejects_bad_migrations() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "p.lua", &provider(1, false));
	write(
		dir.path(),
		"consumer.lua",
		r#"return {inject={"counter"},provide={"read"},apply=function(ctx)
		local counter = ctx.counter
		ctx:provide("read", function() return counter() end)
		ctx:send("consumer", "started")
	end}"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="p",path="p.lua"},{id="c",path="consumer.lua"}}"#,
	);
	let (host, mut events) = boot(dir.path()).await;
	assert_eq!(next_event(&mut events, "consumer").await, json!("started"));
	let consumer = host.fiber_of("c").unwrap().uid();
	assert_eq!(host.call("read", json!(null)).await.unwrap()["version"], 1);
	// The consumer captured the service reference, not the generation behind it.
	write(dir.path(), "p.lua", &provider(2, false));
	host.replace(&dir.path().join("p.lua")).await;
	assert_eq!(host.fiber_of("c").unwrap().uid(), consumer);
	assert_eq!(host.call("read", json!(null)).await.unwrap()["version"], 2);
	// A candidate that raises never publishes; the live generation stays.
	write(dir.path(), "p.lua", &provider(3, true));
	host.replace(&dir.path().join("p.lua")).await;
	assert_eq!(host.call("read", json!(null)).await.unwrap()["version"], 2);
	// And the transaction it failed in is finished, so the next one can begin.
	write(dir.path(), "p.lua", &provider(4, false));
	host.replace(&dir.path().join("p.lua")).await;
	assert_eq!(host.call("read", json!(null)).await.unwrap()["version"], 4);
}

#[tokio::test(flavor = "multi_thread")]
async fn corrected_code_can_recover_a_failed_initial_generation() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "p.lua", &provider(1, true));
	write(dir.path(), "init.lua", r#"return {{id="p",path="p.lua"}}"#);
	let (host, _) = boot(dir.path()).await;
	host.fiber_of("p").unwrap().settled().await;
	assert!(host.fiber_of("p").unwrap().error().is_some());
	write(dir.path(), "p.lua", &provider(2, false));
	host.replace(&dir.path().join("p.lua")).await;
	host.fiber_of("p").unwrap().settled().await;
	assert_eq!(
		host.call("counter", json!(null)).await.unwrap()["version"],
		2
	);
}

#[tokio::test(flavor = "multi_thread")]
async fn two_entries_of_one_file_each_get_their_own_switch() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"p.lua",
		r#"return {apply=function(ctx,config)
		ctx:send("entry", {id=config.id})
	end}"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="a",path="p.lua",config={id="a"}},{id="b",path="p.lua",config={id="b"}}}"#,
	);
	let (host, mut events) = boot(dir.path()).await;
	let mut ids = Vec::new();
	for _ in 0..2 {
		ids.push(
			next_event(&mut events, "entry").await["id"]
				.as_str()
				.unwrap()
				.to_owned(),
		);
	}
	ids.sort();
	assert_eq!(ids, vec!["a", "b"]);
	let original = std::fs::read_to_string(dir.path().join("p.lua")).unwrap();
	write(dir.path(), "p.lua", &(original + "\n-- next generation\n"));
	host.replace(&dir.path().join("p.lua")).await;
	let mut again = Vec::new();
	for _ in 0..2 {
		again.push(
			next_event(&mut events, "entry").await["id"]
				.as_str()
				.unwrap()
				.to_owned(),
		);
	}
	again.sort();
	assert_eq!(again, vec!["a", "b"]);
}
