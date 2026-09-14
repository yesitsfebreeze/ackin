use std::time::Duration;

use super::{boot, next_event, settle, write};
use crate::runtime::State;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn yields_are_boundaries_and_access_is_enforced() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"a.lua",
		r#"return { provide = {"a"}, apply = function(ctx)
			ctx:provide("a", 41)
			ctx:send("step", 1)
			coroutine.yield(function() ctx:send("undo", 1) end)
			ctx:send("step", 2)
			return function() ctx:send("undo", 2) end
		end }"#,
	);
	write(
		dir.path(),
		"b.lua",
		r#"return { inject = {"a"}, apply = function(ctx)
			ctx:send("b", { a = ctx.a + 1, ok = pcall(function() return ctx.missing end) })
		end }"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return { { id = "a", path = "a.lua" }, { id = "b", path = "b.lua" } }"#,
	);
	let (host, mut rx) = boot(dir.path()).await;
	assert_eq!(next_event(&mut rx, "step").await, json!(1));
	assert_eq!(next_event(&mut rx, "step").await, json!(2));
	assert_eq!(
		next_event(&mut rx, "b").await,
		json!({ "a": 42, "ok": false })
	);
	host.fiber_of("a").unwrap().dispose().await;
	assert_eq!(next_event(&mut rx, "undo").await, json!(2));
	assert_eq!(next_event(&mut rx, "undo").await, json!(1));
	settle().await;
	assert_eq!(host.fiber_of("b").unwrap().state(), Some(State::Inactive));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_changed_file_swaps_the_fiber_and_a_broken_one_keeps_it() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"p.lua",
		r#"return { apply = function(ctx) ctx:send("version", 1) end }"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return { { id = "p", path = "p.lua" } }"#,
	);
	let (host, mut rx) = boot(dir.path()).await;
	assert_eq!(next_event(&mut rx, "version").await, json!(1));
	let first = host.fiber_of("p").unwrap().uid();
	write(
		dir.path(),
		"p.lua",
		r#"return { apply = function(ctx) ctx:send("version", 2) end }"#,
	);
	host.replace(&dir.path().join("p.lua")).await;
	assert_eq!(next_event(&mut rx, "version").await, json!(2));
	assert_ne!(host.fiber_of("p").unwrap().uid(), first);
	write(
		dir.path(),
		"p.lua",
		"return { apply = function(ctx) this is not lua end }",
	);
	let second = host.fiber_of("p").unwrap().uid();
	host.replace(&dir.path().join("p.lua")).await;
	let m = tokio::time::timeout(Duration::from_secs(2), rx.recv())
		.await
		.unwrap()
		.unwrap();
	assert_eq!(m["error"]["cartridge"], "p");
	assert_eq!(host.fiber_of("p").unwrap().uid(), second);
	assert_eq!(host.fiber_of("p").unwrap().state(), Some(State::Active));
}

#[tokio::test(flavor = "multi_thread")]
async fn reconcile_adds_removes_and_revises_by_id() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"p.lua",
		r#"return { apply = function(ctx, config) ctx:send("cfg", config) end }"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return { { id = "x", path = "p.lua", config = { n = 1 } } }"#,
	);
	let (host, mut rx) = boot(dir.path()).await;
	assert_eq!(next_event(&mut rx, "cfg").await, json!({ "n": 1 }));
	write(
		dir.path(),
		"init.lua",
		r#"return { { id = "x", path = "p.lua", config = { n = 2 } }, { id = "y", path = "p.lua", config = { n = 3 } } }"#,
	);
	host.reconcile().await.unwrap();
	let mut seen = vec![
		next_event(&mut rx, "cfg").await,
		next_event(&mut rx, "cfg").await,
	];
	seen.sort_by_key(|v| v["n"].as_i64());
	assert_eq!(seen, vec![json!({ "n": 2 }), json!({ "n": 3 })]);
	write(
		dir.path(),
		"init.lua",
		r#"return { { id = "y", path = "p.lua", config = { n = 3 }, disabled = true } }"#,
	);
	host.reconcile().await.unwrap();
	settle().await;
	assert!(host.fiber_of("x").is_none());
	assert!(host.fiber_of("y").is_none());
	assert_eq!(host.runtime().fibers().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn config_lua_overrides_entries_and_manifest_resolves_providers() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"a.lua",
		r#"return { provide = {"a"}, apply = function(ctx, config) ctx:send("cfg", config) end }"#,
	);
	write(
		dir.path(),
		"b.lua",
		r#"return { inject = {"a"}, apply = function() end }"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return { { id = "a", path = "a.lua", config = { n = 1, keep = true } }, { id = "b", path = "b.lua" } }"#,
	);
	write(dir.path(), "config.lua", r#"return { a = { n = 2 } }"#);
	let (host, mut rx) = boot(dir.path()).await;
	assert_eq!(
		next_event(&mut rx, "cfg").await,
		json!({ "n": 2, "keep": true })
	);
	let cartridges = host.manifest().unwrap();
	assert_eq!(cartridges[1].inject, vec!["a"]);
	assert!(cartridges
		.iter()
		.any(|p| p.entry.id == "a" && p.provide == vec!["a"] && p.error.is_none()));
}

#[tokio::test(flavor = "multi_thread")]
async fn wrapped_process_isolation_metadata_and_dependency_restart_compose() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"global.lua",
		r#"return {provide={"lua"},apply=function(ctx)
		ctx:provide("lua", function() return "global" end)
	end}"#,
	);
	write(
		dir.path(),
		"alternate.lua",
		&format!(
			"return cartridge.process({})",
			json!(super::process::lua_fixture())
		),
	);
	write(
		dir.path(),
		"child.lua",
		&format!(
			"return cartridge.process({})",
			json!(super::process::sdk_fixture())
		),
	);
	write(
		dir.path(),
		"parent.lua",
		r#"return {apply=function(ctx)
		local isolated = ctx:isolate("lua"):intercept("lua", {description="isolated metadata"})
		local provider = isolated:cartridge("alternate.lua", {})
		local child = isolated:cartridge("child.lua", {apply_call=true, lifecycle=true})
		ctx:on("remove", function() provider:dispose(); child:dispose(); child:wait(); ctx:send("state", child:state()) end)
		ctx:on("restore", function() provider=isolated:cartridge("alternate.lua", {}); provider:wait(); child=isolated:cartridge("child.lua", {apply_call=true, lifecycle=true}); child:wait(); ctx:send("state", child:state()) end)
	end}"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="global",path="global.lua"},{id="parent",path="parent.lua"}}"#,
	);
	let (host, mut rx) = boot(dir.path()).await;
	assert_eq!(next_event(&mut rx, "apply").await, json!({"value": 7}));
	assert_eq!(host.call("lua", json!(null)).await.unwrap(), "global");
	let response = host.call("roundtrip", json!(42)).await.unwrap();
	assert_eq!(
		response,
		json!({"data":{"value":42},"meta":{"description":"isolated metadata"},"absent":null})
	);
	let original = host.runtime().ctx().peek("roundtrip").unwrap();
	for _ in 0..2 {
		host.emit("remove", json!(null));
		assert_eq!(next_event(&mut rx, "disposed").await, true);
		assert_eq!(next_event(&mut rx, "state").await, json!(null));
		assert!(host.call("roundtrip", json!(null)).await.is_err());
		assert_eq!(
			host.invoke(original.clone(), json!(null))
				.await
				.unwrap_err(),
			"cartridge is gone"
		);
		// Await dispatch itself: a retained listener would fail against its dead link.
		host.runtime()
			.ctx()
			.parallel("probe", std::sync::Arc::new(json!("removed")))
			.await
			.unwrap();
		assert!(rx.try_recv().is_err());
		host.emit("restore", json!(null));
		assert_eq!(next_event(&mut rx, "apply").await, json!({"value": 7}));
		assert_eq!(next_event(&mut rx, "state").await, "Active");
		assert_eq!(
			host.call("roundtrip", json!(0)).await.unwrap(),
			json!({"data":{"value":0},"meta":{"description":"isolated metadata"},"absent":null})
		);
		host.runtime()
			.ctx()
			.parallel("probe", std::sync::Arc::new(json!("one")))
			.await
			.unwrap();
		assert_eq!(next_event(&mut rx, "observed").await, "one");
		assert!(rx.try_recv().is_err(), "duplicate listener after restart");
	}
	host.fiber_of("parent").unwrap().dispose().await;
	assert_eq!(next_event(&mut rx, "disposed").await, true);
	assert!(host.call("roundtrip", json!(0)).await.is_err());
	assert_eq!(
		host.runtime().fibers().len(),
		2,
		"only root and global provider remain"
	);
}

#[test]
fn local_list_resolves_wrappers_without_starting_a_daemon_or_applying_cartridges() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"provider.lua",
		r#"return {provide={"lua"},apply=function() error("list must not apply") end}"#,
	);
	write(
		dir.path(),
		"child.lua",
		&format!(
			"return cartridge.process({})",
			json!(super::process::sdk_fixture())
		),
	);
	let composition = r#"return {{id="arbitrary-provider",path="provider.lua"},{id="arbitrary-child",path="child.lua"}}"#;
	write(dir.path(), "init.lua", composition);
	// The CLI below has no profile to select: it reads the `.cartridge` beside
	// its working directory, so the same composition is written there too.
	std::fs::create_dir_all(dir.path().join(".cartridge")).unwrap();
	write(&dir.path().join(".cartridge"), "init.lua", composition);
	let host = crate::lua::Host::new(crate::runtime::Runtime::new(), dir.path(), dir.path());
	let cartridges = host.manifest().unwrap();
	assert_eq!(cartridges[1].inject, ["lua"]);
	assert_eq!(cartridges[1].provide, ["roundtrip"]);
	assert!(cartridges.iter().all(|p| p.error.is_none()));
	assert_eq!(host.runtime().fibers().len(), 1);
	let output =
		std::process::Command::new(super::built(&["-p", "cartridge", "--bin", "cartridge"]))
			.arg("--dir")
			.arg(dir.path())
			// One user profile: the host reads the `.cartridge` beside its
			// working directory, so the run happens in the fixture root.
			.current_dir(dir.path())
			.arg("list")
			.output()
			.unwrap();
	assert!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
	let stdout = String::from_utf8(output.stdout).unwrap();
	assert!(
		stdout.contains("arbitrary-child  child.lua  provides roundtrip"),
		"{stdout}"
	);
	assert!(stdout.contains("lua <- arbitrary-provider"), "{stdout}");
	assert!(!crate::socket::path(&dir.path().join(".cartridge")).exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn yolo_overrides_cartridge_config_only_for_automatic_services() {
	let dir = tempfile::tempdir().unwrap();
	for name in ["agent", "memo", "other"] {
		write(dir.path(), &format!("{name}.lua"), &format!(
			"return {{ provide = {{'{name}'}}, apply = function(ctx, config) ctx:provide('{name}', function() return config end) end }}"
		));
	}
	write(dir.path(), "init.lua", "return {{id='a',path='agent.lua',config={yolo=false}}, {id='m',path='memo.lua',config={}}, {id='o',path='other.lua',config={}}}");
	for enabled in [false, true] {
		for name in ["agent", "memo", "other"] {
			let host = crate::lua::Host::with_yolo(
				crate::runtime::Runtime::new(),
				dir.path(),
				dir.path(),
				enabled,
			);
			let expected = match name {
				"agent" => json!(enabled),
				"memo" if enabled => json!(true),
				_ => json!(null),
			};
			assert_eq!(host.run(name, json!(null)).await.unwrap()["yolo"], expected);
		}
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn a_glob_injects_every_matching_key_the_other_entries_provide() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"one.lua",
		r#"return { provide = {"tool.one", "router"}, apply = function(ctx)
			ctx:provide("tool.one", 1)
			ctx:provide("router", 100)
		end }"#,
	);
	write(
		dir.path(),
		"two.lua",
		r#"return { provide = {"tool.two"}, apply = function(ctx) ctx:provide("tool.two", 2) end }"#,
	);
	// Provides a tool of its own: a glob never injects what the entry provides,
	// which would make the entry depend on itself.
	write(
		dir.path(),
		"sink.lua",
		r#"return { provide = {"tool.sink"}, apply = function(ctx)
			ctx:provide("tool.sink", 0)
			ctx:send("saw", {
				one = ctx["tool.one"],
				two = ctx["tool.two"],
				router = pcall(function() return ctx["router"] end),
			})
		end }"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {
			{ id = "one", path = "one.lua" },
			{ id = "two", path = "two.lua" },
			{ id = "sink", path = "sink.lua", inject = { "tool.*" } },
		}"#,
	);
	let (_host, mut rx) = boot(dir.path()).await;
	// Both tools arrive from the prefix alone and `router` does not match it.
	// That this entry activates at all is the self-exclusion: a glob that
	// injected its own `tool.sink` would leave it waiting on itself forever.
	assert_eq!(
		next_event(&mut rx, "saw").await,
		json!({ "one": 1, "two": 2, "router": false })
	);
}
