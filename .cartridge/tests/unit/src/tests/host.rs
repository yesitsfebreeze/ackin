use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use super::{built, write};
use crate::host::{Host, State};

const PROVIDER: &str = r#"return {provide={"greet"}, apply=function(ctx)
	ctx:provide("greet", function(args) return "hello " .. args.name end)
end}"#;

const CONSUMER: &str = r#"return {needs={"greet"}, provide={"welcome"}, apply=function(ctx)
	ctx:provide("welcome", function(args) return ctx.greet(args) end)
end}"#;

fn profile(dir: &Path, entries: &str) {
	write(
		dir,
		".cartridge/init.lua",
		&format!("return {{ {entries} }}"),
	);
}

async fn boot(dir: &Path) -> Arc<Host> {
	let host = Host::new(dir, dir.join(".cartridge"), false).unwrap();
	host.reconcile().await.unwrap();
	host
}

fn state(host: &Host, id: &str) -> State {
	host.status()
		.into_iter()
		.find(|s| s.id == id)
		.unwrap()
		.state
}

#[tokio::test(flavor = "multi_thread")]
async fn lua_cartridges_call_what_they_need() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "provider.lua", PROVIDER);
	write(dir.path(), "consumer.lua", CONSUMER);
	profile(
		dir.path(),
		r#"{id="consumer", path="consumer.lua"}, {id="provider", path="provider.lua"}"#,
	);
	let host = boot(dir.path()).await;
	assert_eq!(state(&host, "provider"), State::Active);
	assert_eq!(state(&host, "consumer"), State::Active);
	assert_eq!(
		host.call("welcome", json!({"name": "you"})).await.unwrap(),
		json!("hello you")
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cartridge_cannot_call_what_it_did_not_declare() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "provider.lua", PROVIDER);
	write(
		dir.path(),
		"sneaky.lua",
		r#"return {provide={"peek"}, apply=function(ctx)
			ctx:provide("peek", function() return ctx:call("greet", {name="x"}) end)
		end}"#,
	);
	profile(
		dir.path(),
		r#"{id="provider", path="provider.lua"}, {id="sneaky", path="sneaky.lua"}"#,
	);
	let host = boot(dir.path()).await;
	let error = host.call("peek", json!(null)).await.unwrap_err();
	assert!(error.contains("is not a need"), "{error}");
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn events_go_straight_to_listeners() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"listener.lua",
		r#"return {on={"ping"}, apply=function(ctx)
			ctx:on("ping", function(data) return {pong=data} end)
		end}"#,
	);
	write(
		dir.path(),
		"asker.lua",
		r#"return {provide={"ask", "undeclared"}, apply=function(ctx)
			ctx:provide("ask", function(data)
				local first = ctx:bail("ping", data)
				local all = ctx:gather("ping", data)
				return {first=first, from=all[1].from, count=#all}
			end)
			ctx:provide("undeclared", function()
				local ok, error = pcall(function() ctx:on("other", function() end) end)
				return tostring(error)
			end)
		end}"#,
	);
	profile(
		dir.path(),
		r#"{id="listener", path="listener.lua"}, {id="asker", path="asker.lua"}"#,
	);
	let host = boot(dir.path()).await;
	assert_eq!(
		host.call("ask", json!(7)).await.unwrap(),
		json!({"first": {"pong": 7}, "from": "listener", "count": 1})
	);
	let answers = host.emit("ping", json!(2)).await;
	assert_eq!(
		answers,
		vec![("listener".to_owned(), Ok(json!({"pong": 2})))]
	);
	let refusal = host.call("undeclared", json!(null)).await.unwrap();
	assert!(
		refusal.as_str().unwrap().contains("not declared in `on`"),
		"{refusal}"
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_process_cartridge_serves_calls_events_and_streams() {
	let binary = built(&["--example", "echo_fixture"]);
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"echo/cartridge.json",
		&json!({
			"name": "echo",
			"entry": "init.lua",
			"provide": ["echo", "whoami", "relay"],
			"needs": ["lua.name"],
			"on": ["ping"],
		})
		.to_string(),
	);
	write(
		dir.path(),
		"echo/init.lua",
		&format!(
			"return cartridge.process({:?})",
			binary.display().to_string()
		),
	);
	write(
		dir.path(),
		"named.lua",
		r#"return {provide={"lua.name"}, apply=function(ctx)
			ctx:provide("lua.name", function(args) return "lua saw " .. tostring(args) end)
		end}"#,
	);
	write(
		dir.path(),
		"watcher.lua",
		r#"return {needs={"echo"}, provide={"seen"}, apply=function(ctx)
			local seen = {}
			ctx:subscribe("echo", "ticks", function(envelope)
				if envelope.kind == "data" then table.insert(seen, envelope.data) end
			end)
			ctx:provide("seen", function() return seen end)
		end}"#,
	);
	profile(
		dir.path(),
		r#"{id="echo", path="echo"}, {id="named", path="named.lua"}, {id="watcher", path="watcher.lua"}"#,
	);
	let host = boot(dir.path()).await;
	let status = host.status();
	assert!(
		status.iter().all(|s| s.state == State::Active),
		"{status:?}"
	);
	assert_eq!(
		host.call("echo", json!({"a": 1})).await.unwrap(),
		json!({"a": 1})
	);
	assert_eq!(
		host.call("relay", json!(5)).await.unwrap(),
		json!("lua saw 5")
	);
	assert_eq!(
		host.call("whoami", json!(null)).await.unwrap(),
		json!({"name": "echo", "needs": ["lua.name"]})
	);
	let answers = host.emit("ping", json!("tick")).await;
	assert_eq!(
		answers,
		vec![("echo".to_owned(), Ok(json!({"pong": "tick"})))]
	);
	let mut seen = json!([]);
	for _ in 0..50 {
		seen = host.call("seen", json!(null)).await.unwrap();
		if seen == json!(["tick"]) {
			break;
		}
		tokio::time::sleep(Duration::from_millis(20)).await;
	}
	assert_eq!(seen, json!(["tick"]));
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_restart_keeps_its_dependents_working() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "provider.lua", PROVIDER);
	write(dir.path(), "consumer.lua", CONSUMER);
	profile(
		dir.path(),
		r#"{id="provider", path="provider.lua"}, {id="consumer", path="consumer.lua"}"#,
	);
	let host = boot(dir.path()).await;
	assert_eq!(
		host.call("welcome", json!({"name": "a"})).await.unwrap(),
		json!("hello a")
	);
	write(
		dir.path(),
		"provider.lua",
		&PROVIDER.replace("hello ", "hi "),
	);
	host.replace("provider").await.unwrap();
	assert_eq!(state(&host, "consumer"), State::Active);
	assert_eq!(
		host.call("welcome", json!({"name": "b"})).await.unwrap(),
		json!("hi b")
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_missing_need_waits_and_says_for_what() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "consumer.lua", CONSUMER);
	profile(dir.path(), r#"{id="consumer", path="consumer.lua"}"#);
	let host = boot(dir.path()).await;
	let status = host
		.status()
		.into_iter()
		.find(|s| s.id == "consumer")
		.unwrap();
	assert_eq!(status.state, State::Waiting);
	assert_eq!(status.waiting, vec!["greet".to_owned()]);
	assert!(host.call("welcome", json!(null)).await.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_provided_twice_fails_the_later_entry() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "first.lua", PROVIDER);
	write(dir.path(), "second.lua", PROVIDER);
	profile(
		dir.path(),
		r#"{id="first", path="first.lua"}, {id="second", path="second.lua"}"#,
	);
	let host = boot(dir.path()).await;
	assert_eq!(state(&host, "first"), State::Active);
	let second = host
		.status()
		.into_iter()
		.find(|s| s.id == "second")
		.unwrap();
	assert_eq!(second.state, State::Failed);
	assert!(second
		.error
		.unwrap()
		.contains("already provided by `first`"));
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_host_socket_answers_the_command_line() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "provider.lua", PROVIDER);
	profile(dir.path(), r#"{id="provider", path="provider.lua"}"#);
	let host = boot(dir.path()).await;
	let served = tokio::spawn(crate::host::socket::serve(host.clone()));
	let profile = host.profile().to_path_buf();
	let mut client = None;
	for _ in 0..100 {
		if let Ok(connected) = crate::host::socket::client(&profile).await {
			client = Some(connected);
			break;
		}
		tokio::time::sleep(Duration::from_millis(10)).await;
	}
	let (peer, _incoming) = client.expect("the host socket serves");
	let answer = peer
		.call("call", json!({"key": "greet", "args": {"name": "cli"}}))
		.await
		.unwrap();
	assert_eq!(answer, json!("hello cli"));
	let status = peer.call("status", json!(null)).await.unwrap();
	assert_eq!(status[0]["state"], "active");
	peer.call("stop", json!(null)).await.unwrap();
	assert!(served.await.unwrap().unwrap());
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cartridge_asks_the_host_what_only_the_host_knows() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "provider.lua", PROVIDER);
	write(
		dir.path(),
		"curious.lua",
		r#"return {provide={"ask"}, apply=function(ctx)
			ctx:provide("ask", function()
				local cartridges = ctx:host("cartridges")
				local snapshot = ctx:host("snapshot")
				local ok, refused = pcall(function() return ctx:host("bridge.status") end)
				return {count=#cartridges, entries=#snapshot.entries, bridge=ok, refused=tostring(refused)}
			end)
		end}"#,
	);
	profile(
		dir.path(),
		r#"{id="provider", path="provider.lua"}, {id="curious", path="curious.lua"}"#,
	);
	let host = boot(dir.path()).await;
	let answer = host.call("ask", json!(null)).await.unwrap();
	assert_eq!(answer["count"], 2);
	assert_eq!(answer["entries"], 2);
	assert_eq!(answer["bridge"], false);
	assert!(
		answer["refused"].as_str().unwrap().contains("not granted"),
		"{answer}"
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn verify_calls_every_declared_contract() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"checked/cartridge.json",
		&json!({"name": "checked", "entry": "init.lua", "provide": ["checked.ok"], "selftest": "checked.ok"}).to_string(),
	);
	write(
		dir.path(),
		"checked/init.lua",
		r#"return {apply=function(ctx) ctx:provide("checked.ok", function() return true end) end}"#,
	);
	profile(dir.path(), r#"{id="checked", path="checked"}"#);
	let host = Host::new(dir.path(), dir.path().join(".cartridge"), false).unwrap();
	assert_eq!(host.verify().await.unwrap(), (1, Vec::new()));
}

#[test]
fn a_document_refuses_a_duplicate_or_wildcard_listener() {
	let dir = tempfile::tempdir().unwrap();
	for on in [json!(["a", "a"]), json!(["a.*"])] {
		write(
			dir.path(),
			"cartridge.json",
			&json!({"name": "p", "entry": "init.lua", "on": on}).to_string(),
		);
		assert!(crate::loader::Cartridge::document(&dir.path().join("cartridge.json")).is_err());
	}
}
