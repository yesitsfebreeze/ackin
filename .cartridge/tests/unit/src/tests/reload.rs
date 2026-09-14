use super::{boot, next_event, settle, write};
use crate::runtime::{Component, State};
use futures::StreamExt;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::Receiver;

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

/// Wait for exactly the generation under test, then distinguish successful
/// readiness from a failed or retired fiber (which can also be settled).
async fn ready_generation(host: &Arc<super::Host>, uid: crate::runtime::Uid) {
	let fiber = host.runtime().handle(uid);
	tokio::time::timeout(Duration::from_secs(5), fiber.settled())
		.await
		.expect("generation did not settle within 5s");
	assert_eq!(fiber.state(), Some(State::Active), "{:?}", fiber.error());
	assert!(fiber.error().is_none());
}

/// A nested cartridge is swapped under the running tree: the parent composes
/// it and hands its uid out, the host addresses the node by that uid. The
/// parent is the dependent — it follows the replacement while a sibling entry
/// beside it never re-applies.
#[tokio::test(flavor = "multi_thread")]
async fn a_nested_node_is_swapped_and_its_dependent_follows() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"inner.lua",
		&provider(1, false).replacen("return {", "return {inject={\"build_ready\"},", 1),
	);
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
	write(
		dir.path(),
		"other.lua",
		r#"return {provide={"mark"},apply=function(ctx)
		ctx:provide("mark", function() return "mark" end)
		ctx:send("other", "started")
	end}"#,
	);
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
	// Force the old ordering: announcing the parent and waiting 60 ms does
	// not make the nested provider ready while its build dependency is held.
	settle().await;
	assert_ne!(host.runtime().handle(inner).state(), Some(State::Active));
	assert!(host.call("read", json!(null)).await.is_err());
	let build_ready = host.runtime().ctx().cartridge(
		Component::new(
			"build-ready",
			Arc::new(|ctx| {
				futures::stream::once(async move {
					ctx.provide("build_ready", Arc::new(json!(true)))?;
					let dispose: crate::runtime::Disposer = Box::new(|| Box::pin(async {}));
					Ok(dispose)
				})
				.boxed()
			}),
		)
		.provide(["build_ready"]),
	);
	build_ready.settled().await;
	ready_generation(&host, inner).await;
	assert_eq!(host.call("read", json!(null)).await.unwrap()["version"], 1);
	let outer = host.fiber_of("o").unwrap().uid();
	let sibling = host.fiber_of("x").unwrap().uid();
	// The inner source changes on disk; the nested rebuild re-reads it.
	write(dir.path(), "inner.lua", &provider(2, false));
	let generation = host
		.replace_node(inner)
		.await
		.unwrap_or_else(|e| panic!("swap failed: {e}"));
	assert_ne!(generation, inner);
	ready_generation(&host, generation).await;
	assert_eq!(host.call("read", json!(null)).await.unwrap()["version"], 2);
	// The dependent kept its generation; the sibling beside it never noticed.
	assert_eq!(host.fiber_of("o").unwrap().uid(), outer);
	assert_eq!(host.fiber_of("x").unwrap().uid(), sibling);
	// A candidate that raises never publishes; the live generation stays.
	write(dir.path(), "inner.lua", &provider(3, true));
	assert!(host.replace_node(generation).await.is_err());
	ready_generation(&host, generation).await;
	assert_eq!(host.call("read", json!(null)).await.unwrap()["version"], 2);
	// And the transaction it failed in is finished, so the next one can begin.
	write(dir.path(), "inner.lua", &provider(4, false));
	let third = host.replace_node(generation).await.unwrap();
	ready_generation(&host, third).await;
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
	assert_eq!(
		host.call("mark", json!(null)).await.unwrap(),
		json!("mark2")
	);
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

/// A swap runs behind the ask that fired it, so the observable the reading
/// service reports arrives a beat later; the poll keeps the test off a fixed
/// sleep while holding the 5s budget a missing swap would take to blow.
async fn version_of(host: &std::sync::Arc<super::Host>, want: u64) {
	for _ in 0..250 {
		if let Ok(v) = host.call("read", json!(null)).await {
			if v["version"].as_u64() == Some(want) {
				return;
			}
		}
		tokio::time::sleep(Duration::from_millis(20)).await;
	}
	panic!("the reading service never reported version {want}");
}

/// The next error frame on the outbox: a refused reload ask lands on the
/// report channel, named by the uid that fired it.
async fn error_of(rx: &mut Receiver<serde_json::Value>) -> serde_json::Value {
	loop {
		let m = tokio::time::timeout(Duration::from_secs(5), rx.recv())
			.await
			.unwrap_or_else(|_| panic!("no error frame within 5s"))
			.unwrap();
		if m.get("error").is_some() {
			return m["error"].clone();
		}
	}
}

/// The Lua handle addresses the node, not the generation: the second reload
/// fired from the same handle lands on the generation the first swap
/// published, because what persists across a swap is the node's shared
/// reload transaction, and the reading service returns each generation's
/// value in turn.
#[tokio::test(flavor = "multi_thread")]
async fn a_composed_handle_reloads_its_node_again_after_the_first_swap() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "inner.lua", &provider(1, false));
	write(
		dir.path(),
		"outer.lua",
		r#"return {provide={"read","kick"},apply=function(ctx)
		local inner = ctx:cartridge("inner.lua")
		ctx:provide("read", function() return ctx:peek("counter")() end)
		ctx:provide("kick", function() inner:reload() return true end)
		ctx:send("outer", "started")
	end}"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="o",path="outer.lua"}}"#,
	);
	let (host, mut events) = boot(dir.path()).await;
	next_event(&mut events, "outer").await;
	version_of(&host, 1).await;
	// The source changes; the handle fires the first swap.
	write(dir.path(), "inner.lua", &provider(2, false));
	host.call("kick", json!(null)).await.unwrap();
	version_of(&host, 2).await;
	// The same handle — its uid was retired by the first swap — fires the
	// second swap, and it lands on the generation the first published.
	write(dir.path(), "inner.lua", &provider(3, false));
	host.call("kick", json!(null)).await.unwrap();
	version_of(&host, 3).await;
}

/// A raw reload ask at a uid the tree already replaced is refused on the
/// report channel rather than dropped, and the node still reloads from its
/// live generation afterwards; the entry ask `{"reload": true}` dispatches
/// to, `host.request_reload(uid)` on a profile entry, still swaps it.
#[tokio::test(flavor = "multi_thread")]
async fn an_ask_at_a_retired_uid_is_refused_and_the_node_still_reloads() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "inner.lua", &provider(1, false));
	write(
		dir.path(),
		"outer.lua",
		r#"return {provide={"read","kick"},apply=function(ctx)
		local inner = ctx:cartridge("inner.lua")
		ctx:send("inner", {uid=inner:uid()})
		ctx:provide("read", function() return ctx:peek("counter")() end)
		ctx:provide("kick", function() inner:reload() return true end)
		ctx:send("outer", "started")
	end}"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="o",path="outer.lua"}}"#,
	);
	let (host, mut events) = boot(dir.path()).await;
	let mut inner = None;
	let mut started = false;
	while !started {
		let m = tokio::time::timeout(Duration::from_secs(5), events.recv())
			.await
			.unwrap()
			.unwrap();
		if m["event"] == "inner" {
			inner = Some(m["data"]["uid"].as_u64().unwrap());
		} else if m["event"] == "outer" {
			started = true;
		}
	}
	let retired = inner.unwrap();
	version_of(&host, 1).await;
	// The handle swaps the node; the retired uid stops carrying anything.
	write(dir.path(), "inner.lua", &provider(2, false));
	host.call("kick", json!(null)).await.unwrap();
	version_of(&host, 2).await;
	// The raw ask at the retired uid is refused, by name.
	host.request_reload(retired);
	let refused = error_of(&mut events).await;
	assert!(
		refused["message"]
			.as_str()
			.unwrap()
			.contains("no running node carries uid"),
		"{refused}"
	);
	assert_eq!(
		refused["cartridge"].as_str().unwrap().parse::<u64>().ok(),
		Some(retired)
	);
	// And the node still reloads from its live generation.
	write(dir.path(), "inner.lua", &provider(3, false));
	host.call("kick", json!(null)).await.unwrap();
	version_of(&host, 3).await;
	// The entry ask `{"reload": true}` answers through the same dispatch,
	// without change: the profile entry re-applies on the ask.
	host.request_reload(host.fiber_of("o").unwrap().uid());
	assert_eq!(next_event(&mut events, "outer").await, json!("started"));
}

/// A node composed without a rebuild refuses the ask by name instead of
/// guessing at one, and a uid nothing in the tree carries is refused rather
/// than ignored.
#[tokio::test(flavor = "multi_thread")]
async fn an_uncomposed_node_and_an_unknown_uid_refuse_the_ask() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "init.lua", r#"return {}"#);
	let (host, _events) = boot(dir.path()).await;
	let plain = host
		.runtime()
		.ctx()
		.cartridge(crate::runtime::Component::new(
			"plain",
			Arc::new(|_| futures::stream::empty().boxed()),
		));
	plain.settled().await;
	let refused = host.replace_node(plain.uid()).await.unwrap_err();
	assert!(matches!(refused, crate::Error::Reload(_)), "{refused:?}");
	assert_eq!(
		refused.to_string(),
		"this node was not composed to be replaceable"
	);
	let refused = host.replace_node(4242).await.unwrap_err();
	assert_eq!(refused.to_string(), "no running node carries uid 4242");
}
