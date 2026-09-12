use super::{boot, next_event, settle, write};
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

/// A nested cartridge is swapped under the running tree: the parent composes
/// it and hands its uid out, the host addresses the node by that uid. The
/// parent is the dependent — it follows the replacement while a sibling entry
/// beside it never re-applies.
#[tokio::test(flavor = "multi_thread")]
async fn a_nested_node_is_swapped_and_its_dependent_follows() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "inner.lua", &provider(1, false));
	write(
		dir.path(),
		"outer.lua",
		r#"return {provide={"read"},apply=function(ctx)
		local inner = ctx:cartridge("inner.lua")
		ctx:send("inner", {uid=inner:uid()})
		ctx:provide("read", function() return ctx:peek("counter")() end)
		ctx:send("outer", "started")
	end}"#,
	);
	write(dir.path(), "other.lua", r#"return {provide={"mark"},apply=function(ctx)
		ctx:provide("mark", function() return "mark" end)
		ctx:send("other", "started")
	end}"#);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="o",path="outer.lua"},{id="x",path="other.lua"}}"#,
	);
	let (host, mut events) = boot(dir.path()).await;
	// Entry composition order is racy, so the events are collected by name.
	let mut inner = None;
	let mut started = 0;
	while started < 2 {
		let m = tokio::time::timeout(std::time::Duration::from_secs(5), events.recv())
			.await
			.unwrap()
			.unwrap();
		if m["event"] == "inner" {
			inner = Some(m["data"]["uid"].as_u64().unwrap());
		} else if m["event"] == "outer" || m["event"] == "other" {
			started += 1;
		}
	}
	let inner = inner.unwrap();
	// The uid is handed out at composition, before the first generation is up.
	settle().await;
	assert_eq!(host.call("read", json!(null)).await.unwrap()["version"], 1);
	let outer = host.fiber_of("o").unwrap().uid();
	let sibling = host.fiber_of("x").unwrap().uid();
	// The inner source changes on disk; the nested rebuild re-reads it.
	write(dir.path(), "inner.lua", &provider(2, false));
	let generation = host.replace_node(inner).await.unwrap_or_else(|e| panic!("swap failed: {e}"));
	assert_ne!(generation, inner);
	assert_eq!(host.call("read", json!(null)).await.unwrap()["version"], 2);
	// The dependent kept its generation; the sibling beside it never noticed.
	assert_eq!(host.fiber_of("o").unwrap().uid(), outer);
	assert_eq!(host.fiber_of("x").unwrap().uid(), sibling);
	// A candidate that raises never publishes; the live generation stays.
	write(dir.path(), "inner.lua", &provider(3, true));
	assert!(host.replace_node(generation).await.is_err());
	assert_eq!(host.call("read", json!(null)).await.unwrap()["version"], 2);
	// And the transaction it failed in is finished, so the next one can begin.
	write(dir.path(), "inner.lua", &provider(4, false));
	let third = host.replace_node(generation).await.unwrap();
	assert_eq!(host.call("read", json!(null)).await.unwrap()["version"], 4);
	assert_ne!(third, generation);
	// The same ask answered for a profile entry goes through the entry path.
	write(
		dir.path(),
		"other.lua",
		r#"return {provide={"mark"},apply=function(ctx)
		ctx:provide("mark", function() return "mark2" end)
		ctx:send("other", "swapped")
	end}"#,
	);
	host.replace_node(sibling).await.unwrap();
	assert_eq!(host.call("mark", json!(null)).await.unwrap(), json!("mark2"));
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
