use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use super::{built, write};
use crate::host::{Host, State};
use crate::transport::cartridge::Outcome;

/// A cartridge folder: its manifest and its `init.lua`.
fn cartridge(dir: &Path, id: &str, manifest: Value, init: &str) {
	write(dir, &format!("{id}/cartridge.json"), &manifest.to_string());
	write(dir, &format!("{id}/init.lua"), init);
}

fn profile(dir: &Path, ids: &[&str]) {
	let entries: Vec<String> = ids
		.iter()
		.map(|id| format!("{{id={id:?}, path={id:?}}}"))
		.collect();
	write(
		dir,
		".cartridge/init.lua",
		&format!("return {{ {} }}", entries.join(", ")),
	);
	trust(dir);
}

fn greeter(dir: &Path) {
	cartridge(
		dir,
		"greeter",
		json!({
			"name": "greeter", "entry": "init.lua",
			"events": {"greet": {"description": "a greeting", "schema": {"type": "object", "required": ["name"], "properties": {"name": {"type": "string"}}}}},
			"listen": ["greet"],
		}),
		r#"cartridge.listen("greet", function(args) return "hello " .. args.name end)"#,
	);
}

/// The test binary is not the base; nodes run the built `cartridge`.
fn node_binary() {
	static BIN: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	let bin = BIN.get_or_init(|| built(&["--bin", "cartridge"]));
	std::env::set_var(crate::host::NODE_BIN_ENV, bin);
}

/// Approve what the test wrote, as `cartridge trust` would.
fn trust(dir: &Path) {
	super::home();
	crate::trust::record(dir).unwrap();
}

async fn boot(dir: &Path) -> Arc<Host> {
	node_binary();
	let host = Host::new(dir, dir.join(".cartridge")).unwrap();
	host.reconcile().await.unwrap();
	host
}

fn status(host: &Host, id: &str) -> crate::host::Status {
	host.status().into_iter().find(|s| s.id == id).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cartridge_answers_the_events_it_listens_to() {
	let dir = tempfile::tempdir().unwrap();
	greeter(dir.path());
	cartridge(
		dir.path(),
		"welcome",
		json!({
			"name": "welcome", "entry": "init.lua",
			"events": {"welcome": {}}, "needs": ["greet"], "listen": ["welcome"],
		}),
		r#"cartridge.listen("welcome", function(args) return cartridge.bail("greet", args) end)"#,
	);
	profile(dir.path(), &["welcome", "greeter"]);
	let host = boot(dir.path()).await;
	assert_eq!(status(&host, "greeter").state, State::Active);
	assert_eq!(status(&host, "welcome").state, State::Active);
	assert_eq!(
		host.bail("welcome", json!({"name": "you"})).await.unwrap(),
		Some(json!("hello you"))
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_payload_the_schema_rejects_never_reaches_the_listener() {
	let dir = tempfile::tempdir().unwrap();
	greeter(dir.path());
	profile(dir.path(), &["greeter"]);
	let host = boot(dir.path()).await;
	let error = host.bail("greet", json!({"name": 7})).await.unwrap_err();
	assert!(error.contains("rejected by its schema"), "{error}");
	let error = host.bail("nobody.declares", json!(null)).await.unwrap_err();
	assert!(error.contains("not provided"), "{error}");
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_schema_only_change_reaches_a_sender_that_already_validated() {
	let dir = tempfile::tempdir().unwrap();
	greeter(dir.path());
	profile(dir.path(), &["greeter"]);
	let host = boot(dir.path()).await;
	// Warm the validator: the good payload passes, the bad one is refused.
	assert_eq!(
		host.bail("greet", json!({"name": "you"})).await.unwrap(),
		Some(json!("hello you"))
	);
	let error = host.bail("greet", json!({"name": 7})).await.unwrap_err();
	assert!(error.contains("rejected by its schema"), "{error}");
	// Only the schema changes: the event names, needs and listen stay the same.
	cartridge(
		dir.path(),
		"greeter",
		json!({
			"name": "greeter", "entry": "init.lua",
			"events": {"greet": {"description": "a greeting", "schema": {"type": "object", "required": ["other"], "properties": {"other": {"type": "string"}}}}},
			"listen": ["greet"],
		}),
		r#"cartridge.listen("greet", function(args) return "hello " .. args.other end)"#,
	);
	trust(dir.path());
	host.replace("greeter").await.unwrap();
	// The warmed sender must validate against the new schema, not the old one.
	let error = host
		.bail("greet", json!({"name": "you"}))
		.await
		.unwrap_err();
	assert!(error.contains("rejected by its schema"), "{error}");
	assert_eq!(
		host.bail("greet", json!({"other": "you"})).await.unwrap(),
		Some(json!("hello you"))
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_undeclared_event_fails_the_cartridge_before_it_starts() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"stray",
		json!({"name": "stray", "entry": "init.lua", "listen": ["nobody.declares"]}),
		"",
	);
	cartridge(
		dir.path(),
		"needy",
		json!({"name": "needy", "entry": "init.lua", "needs": ["also.undeclared"]}),
		"",
	);
	profile(dir.path(), &["stray", "needy"]);
	let host = boot(dir.path()).await;
	let stray = status(&host, "stray");
	assert_eq!(stray.state, State::Failed);
	assert!(stray
		.error
		.unwrap()
		.contains("listens to `nobody.declares`, which no cartridge declares"));
	let needy = status(&host, "needy");
	assert_eq!(needy.state, State::Failed);
	assert!(needy.error.unwrap().contains("needs `also.undeclared`"));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_event_declared_twice_fails_the_later_entry() {
	let dir = tempfile::tempdir().unwrap();
	greeter(dir.path());
	cartridge(
		dir.path(),
		"twin",
		json!({"name": "twin", "entry": "init.lua", "events": {"greet": {}}}),
		"",
	);
	profile(dir.path(), &["greeter", "twin"]);
	let host = boot(dir.path()).await;
	assert_eq!(status(&host, "greeter").state, State::Active);
	let twin = status(&host, "twin");
	assert_eq!(twin.state, State::Failed);
	assert!(twin
		.error
		.unwrap()
		.contains("already declared by `greeter`"));
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_node_refuses_to_listen_to_what_it_did_not_declare() {
	let dir = tempfile::tempdir().unwrap();
	greeter(dir.path());
	cartridge(
		dir.path(),
		"sneaky",
		json!({"name": "sneaky", "entry": "init.lua", "events": {"sneak": {}}, "listen": ["sneak"]}),
		r#"cartridge.listen("sneak", function()
			local ok, error = pcall(cartridge.listen, "greet", function() end)
			return tostring(error)
		end)"#,
	);
	profile(dir.path(), &["greeter", "sneaky"]);
	let host = boot(dir.path()).await;
	let answer = host.bail("sneak", json!(null)).await.unwrap().unwrap();
	assert!(
		answer
			.as_str()
			.unwrap()
			.contains("not declared in `listen`"),
		"{answer}"
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn gather_and_emit_reach_every_listener() {
	let dir = tempfile::tempdir().unwrap();
	for id in ["a", "b"] {
		cartridge(
			dir.path(),
			id,
			json!({"name": id, "entry": "init.lua", "listen": ["ping"]}),
			&format!(
				r#"cartridge.listen("ping", function(data) return {{ from = {id:?}, data = data }} end)"#
			),
		);
	}
	cartridge(
		dir.path(),
		"asker",
		json!({"name": "asker", "entry": "init.lua", "events": {"ping": {}, "ask": {}}, "listen": ["ask"]}),
		r#"cartridge.listen("ask", function(data)
			local all = cartridge.gather("ping", data)
			cartridge.emit("ping", data)
			return { count = #all, first = all[1].from, second = all[2].from, outcome = all[1].outcome }
		end)"#,
	);
	profile(dir.path(), &["a", "b", "asker"]);
	let host = boot(dir.path()).await;
	assert_eq!(
		host.bail("ask", json!(1)).await.unwrap(),
		Some(json!({"count": 2, "first": "a", "second": "b", "outcome": "answered"}))
	);
	let answers = host.gather("ping", json!(2)).await.unwrap();
	assert_eq!(answers.len(), 2);
	assert_eq!(
		answers[0],
		Outcome::Answered {
			from: "a".into(),
			data: json!({"from": "a", "data": 2})
		}
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cartridge_sends_only_what_it_defines_or_needs() {
	let dir = tempfile::tempdir().unwrap();
	greeter(dir.path());
	cartridge(
		dir.path(),
		"stranger",
		json!({"name": "stranger", "entry": "init.lua", "events": {"try": {}}, "listen": ["try"]}),
		r#"cartridge.listen("try", function(args)
			local ok, error = pcall(cartridge.bail, "greet", args)
			return tostring(error)
		end)"#,
	);
	profile(dir.path(), &["greeter", "stranger"]);
	let host = boot(dir.path()).await;
	let answer = host
		.bail("try", json!({"name": "x"}))
		.await
		.unwrap()
		.unwrap();
	assert!(
		answer.as_str().unwrap().contains("defines or needs"),
		"{answer}"
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_that_comes_back_into_its_sender_is_answered() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"left",
		json!({"name": "left", "entry": "init.lua",
			"events": {"go": {}, "ping": {}}, "listen": ["go", "pong"]}),
		r#"cartridge.listen("go", function() return cartridge.bail("ping", {}) end)
		cartridge.listen("pong", function() return "pong from left" end)"#,
	);
	cartridge(
		dir.path(),
		"right",
		json!({"name": "right", "entry": "init.lua",
			"events": {"pong": {}}, "listen": ["ping"]}),
		r#"cartridge.listen("ping", function() return cartridge.bail("pong", {}) end)"#,
	);
	profile(dir.path(), &["left", "right"]);
	let host = boot(dir.path()).await;
	let answer = tokio::time::timeout(Duration::from_secs(5), host.bail("go", json!(null)))
		.await
		.expect("left serves pong while its go handler waits");
	assert_eq!(answer.unwrap(), Some(json!("pong from left")));
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_hung_listener_times_out_and_its_node_keeps_serving() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"stuck",
		json!({"name": "stuck", "entry": "init.lua",
			"events": {"hang": {"timeout_ms": 300}, "quick": {}},
			"listen": ["hang", "quick"],
			"grant": {"exec": ["/bin/sleep"]}}),
		r#"local sleeper = cartridge.spawn({"/bin/sleep", "60"})
		cartridge.listen("hang", function() return sleeper:request({}) end)
		cartridge.listen("quick", function() return "here" end)"#,
	);
	profile(dir.path(), &["stuck"]);
	let host = boot(dir.path()).await;
	let hung = tokio::spawn({
		let host = host.clone();
		async move { host.gather("hang", json!(null)).await }
	});
	tokio::time::sleep(Duration::from_millis(50)).await;
	let quick = tokio::time::timeout(Duration::from_millis(200), host.bail("quick", json!(null)))
		.await
		.expect("the node answers while another handler waits");
	assert_eq!(quick.unwrap(), Some(json!("here")));
	assert_eq!(
		hung.await.unwrap().unwrap(),
		vec![Outcome::TimedOut {
			from: "stuck".into()
		}]
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_waiting_cartridge_starts_when_the_profile_adds_its_listener() {
	let dir = tempfile::tempdir().unwrap();
	greeter(dir.path());
	cartridge(
		dir.path(),
		"welcome",
		json!({"name": "welcome", "entry": "init.lua", "events": {"welcome": {}}, "needs": ["greet"], "listen": ["welcome"]}),
		r#"cartridge.listen("welcome", function(args) return cartridge.bail("greet", args) end)"#,
	);
	cartridge(
		dir.path(),
		"shadow",
		json!({"name": "shadow", "entry": "init.lua", "events": {"greet": {}}}),
		"",
	);
	profile(dir.path(), &["shadow", "welcome"]);
	let host = boot(dir.path()).await;
	assert_eq!(status(&host, "welcome").state, State::Waiting);

	profile(dir.path(), &["greeter", "welcome"]);
	host.reconcile().await.unwrap();
	assert_eq!(status(&host, "shadow").state, State::Disabled);
	assert_eq!(status(&host, "welcome").state, State::Active);
	assert_eq!(
		host.bail("welcome", json!({"name": "late"})).await.unwrap(),
		Some(json!("hello late"))
	);

	profile(dir.path(), &["welcome"]);
	host.reconcile().await.unwrap();
	assert_eq!(status(&host, "greeter").state, State::Disabled);
	let error = host
		.bail("welcome", json!({"name": "gone"}))
		.await
		.unwrap_err();
	assert!(
		error.contains("defines or needs") || error.contains("not an event"),
		"{error}"
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_native_module_reaches_the_base_through_the_global() {
	let module = built(&[
		"--manifest-path",
		".cartridge/tests/unit/src/tests/fixtures/native/Cargo.toml",
	]);
	let dir = tempfile::tempdir().unwrap();
	greeter(dir.path());
	cartridge(
		dir.path(),
		"native",
		json!({
			"name": "native", "entry": "init.lua",
			"events": {"twice": {"schema": {"type": "integer"}}, "relay": {}},
			"needs": ["greet"], "listen": ["twice", "relay"],
		}),
		r#"local native = cartridge.load("native_fixture")
		cartridge.listen("twice", function(n) return native.twice(n) end)
		cartridge.listen("relay", function(args) return native.ask("greet", args) end)"#,
	);
	std::fs::copy(
		&module,
		dir.path().join("native").join(module.file_name().unwrap()),
	)
	.unwrap();
	profile(dir.path(), &["greeter", "native"]);
	let host = boot(dir.path()).await;
	let native = status(&host, "native");
	assert_eq!(native.state, State::Active, "{:?}", native.error);
	assert_eq!(
		host.bail("twice", json!(21)).await.unwrap(),
		Some(json!(42))
	);
	assert_eq!(
		host.bail("relay", json!({"name": "rust"})).await.unwrap(),
		Some(json!("hello rust"))
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_spawned_program_answers_requests_by_id() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"echoer",
		json!({
			"name": "echoer", "entry": "init.lua",
			"events": {"echo": {}}, "listen": ["echo"],
			"grant": {"exec": ["/bin/cat"]},
		}),
		r#"local cat = cartridge.spawn({"/bin/cat"})
		cartridge.listen("echo", function(data) return cat:request({ data = data }) end)"#,
	);
	profile(dir.path(), &["echoer"]);
	let host = boot(dir.path()).await;
	let answer = host.bail("echo", json!("hi")).await.unwrap().unwrap();
	assert_eq!(answer["data"], "hi");
	assert!(answer["id"].is_u64());
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn streams_replay_and_then_deliver_live() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"source",
		json!({"name": "source", "entry": "init.lua", "events": {"tick": {}}, "listen": ["tick"]}),
		r#"cartridge.listen("tick", function(n) cartridge.publish("ticks", n) return true end)"#,
	);
	cartridge(
		dir.path(),
		"watcher",
		json!({"name": "watcher", "entry": "init.lua", "events": {"seen": {}}, "needs": ["tick"], "listen": ["seen"]}),
		r#"local seen = {}
		cartridge.subscribe("source", "ticks", function(envelope)
			if envelope.kind == "data" then table.insert(seen, envelope.data) end
		end)
		cartridge.listen("seen", function() return seen end)"#,
	);
	profile(dir.path(), &["source", "watcher"]);
	let host = boot(dir.path()).await;
	host.bail("tick", json!(1)).await.unwrap();
	host.bail("tick", json!(2)).await.unwrap();
	let mut seen = json!([]);
	for _ in 0..50 {
		seen = host
			.bail("seen", json!(null))
			.await
			.unwrap()
			.unwrap_or_default();
		if seen == json!([1, 2]) {
			break;
		}
		tokio::time::sleep(Duration::from_millis(20)).await;
	}
	assert_eq!(seen, json!([1, 2]));
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_restart_keeps_its_dependents_working() {
	let dir = tempfile::tempdir().unwrap();
	greeter(dir.path());
	cartridge(
		dir.path(),
		"welcome",
		json!({"name": "welcome", "entry": "init.lua", "events": {"welcome": {}}, "needs": ["greet"], "listen": ["welcome"]}),
		r#"cartridge.listen("welcome", function(args) return cartridge.bail("greet", args) end)"#,
	);
	profile(dir.path(), &["greeter", "welcome"]);
	let host = boot(dir.path()).await;
	assert_eq!(
		host.bail("welcome", json!({"name": "a"})).await.unwrap(),
		Some(json!("hello a"))
	);
	write(
		dir.path(),
		"greeter/init.lua",
		r#"cartridge.listen("greet", function(args) return "hi " .. args.name end)"#,
	);
	let refused = host.replace("greeter").await.unwrap_err();
	assert!(
		refused.contains("has changed since it was trusted"),
		"{refused}"
	);
	trust(dir.path());
	host.replace("greeter").await.unwrap();
	assert_eq!(status(&host, "welcome").state, State::Active);
	assert_eq!(
		host.bail("welcome", json!({"name": "b"})).await.unwrap(),
		Some(json!("hi b"))
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_missing_listener_waits_and_says_for_what() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"welcome",
		json!({"name": "welcome", "entry": "init.lua", "events": {"greet": {}, "welcome": {}}, "needs": ["greet"], "listen": ["welcome"]}),
		"",
	);
	profile(dir.path(), &["welcome"]);
	let host = boot(dir.path()).await;
	let welcome = status(&host, "welcome");
	assert_eq!(welcome.state, State::Waiting);
	assert_eq!(welcome.waiting, vec!["greet".to_owned()]);
	assert!(host.bail("welcome", json!(null)).await.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cartridge_asks_the_host_what_only_the_host_knows() {
	let dir = tempfile::tempdir().unwrap();
	greeter(dir.path());
	cartridge(
		dir.path(),
		"curious",
		json!({"name": "curious", "entry": "init.lua", "events": {"ask": {}}, "listen": ["ask"]}),
		r#"cartridge.listen("ask", function()
			local cartridges = cartridge.host("cartridges", nil)
			local snapshot = cartridge.host("snapshot", nil)
			local ok, refused = pcall(cartridge.host, "stop", nil)
			return { count = #cartridges, entries = #snapshot.entries, stopped = ok, refused = tostring(refused),
				needs = cartridge.needs(), events = cartridge.events() }
		end)"#,
	);
	profile(dir.path(), &["greeter", "curious"]);
	let host = boot(dir.path()).await;
	let answer = host.bail("ask", json!(null)).await.unwrap().unwrap();
	assert_eq!(answer["count"], 2);
	assert_eq!(answer["entries"], 2);
	assert_eq!(answer["stopped"], false);
	assert!(
		answer["refused"].as_str().unwrap().contains("not granted"),
		"{answer}"
	);
	assert_eq!(answer["events"]["greet"]["owner"], "greeter");
	assert_eq!(answer["events"]["greet"]["description"], "a greeting");
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_host_socket_answers_the_command_line() {
	let dir = tempfile::tempdir().unwrap();
	greeter(dir.path());
	profile(dir.path(), &["greeter"]);
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
		.call("bail", json!({"name": "greet", "data": {"name": "cli"}}))
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
async fn a_glob_in_needs_names_what_the_others_listen_to() {
	let dir = tempfile::tempdir().unwrap();
	greeter(dir.path());
	cartridge(
		dir.path(),
		"tools",
		json!({"name": "tools", "entry": "init.lua", "events": {"tool.a": {}, "tool.b": {}, "other": {}}, "listen": ["tool.a", "tool.b", "other"]}),
		r#"cartridge.listen("tool.a", function() return "a" end)"#,
	);
	cartridge(
		dir.path(),
		"user",
		json!({"name": "user", "entry": "init.lua", "events": {"which": {}}, "listen": ["which"], "needs": ["tool.*", "greet"]}),
		r#"cartridge.listen("which", function() return cartridge.needs() end)"#,
	);
	profile(dir.path(), &["greeter", "tools", "user"]);
	let host = boot(dir.path()).await;
	assert_eq!(status(&host, "user").state, State::Active);
	assert_eq!(
		host.bail("which", json!(null)).await.unwrap(),
		Some(json!(["tool.a", "tool.b", "greet"]))
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_node_serves_other_events_while_a_handler_waits() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"busy",
		json!({"name": "busy", "entry": "init.lua", "events": {"slow": {}, "fast": {}}, "listen": ["slow", "fast"],
			"grant": {"exec": ["/bin/sleep"]}}),
		r#"local mute = cartridge.spawn({"/bin/sleep", "30"}, { timeout_ms = 3000 })
		cartridge.listen("slow", function() local ok, e = pcall(mute.request, mute, {}) return tostring(e) end)
		cartridge.listen("fast", function() return "fast" end)"#,
	);
	profile(dir.path(), &["busy"]);
	let host = boot(dir.path()).await;
	let slow = tokio::spawn({
		let host = host.clone();
		async move { host.bail("slow", json!(null)).await }
	});
	tokio::time::sleep(std::time::Duration::from_millis(300)).await;
	let started = std::time::Instant::now();
	assert_eq!(
		host.bail("fast", json!(null)).await.unwrap(),
		Some(json!("fast"))
	);
	assert!(started.elapsed() < std::time::Duration::from_secs(1));
	let answer = slow.await.unwrap().unwrap().unwrap();
	assert!(
		answer.as_str().unwrap().contains("did not answer"),
		"{answer}"
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn verify_sends_every_declared_contract() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"checked",
		json!({"name": "checked", "entry": "init.lua", "events": {"checked.ok": {}}, "listen": ["checked.ok"], "contracts": ["checked.ok"]}),
		r#"cartridge.listen("checked.ok", function() return true end)"#,
	);
	profile(dir.path(), &["checked"]);
	node_binary();
	let host = Host::new(dir.path(), dir.path().join(".cartridge")).unwrap();
	assert_eq!(host.verify().await.unwrap(), (1, Vec::new()));
}

#[tokio::test(flavor = "multi_thread")]
async fn grant_paths_name_the_project_and_settings() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"store",
		json!({
			"name": "store", "entry": "init.lua",
			"settings": {"dir": {"type": "string", "default": ".cartridge/store"}},
			"grant": {"write": ["${config.dir}", "$PROJECT/logs", "$TMPDIR/x"], "read": ["$HOME/.config"]},
		}),
		"",
	);
	profile(dir.path(), &["store"]);
	let host = Host::new(dir.path(), dir.path().join(".cartridge")).unwrap();
	let entry = host
		.entries()
		.unwrap()
		.into_iter()
		.find(|e| e.id == "store")
		.unwrap();
	let plan = host.plan(&entry).unwrap();
	let project = dir.path().canonicalize().unwrap();
	assert_eq!(
		plan.grant.write[0],
		project.join(".cartridge/store").display().to_string()
	);
	assert_eq!(
		plan.grant.write[1],
		project.join("logs").display().to_string()
	);
	assert!(!plan.grant.write[2].starts_with('$'));
	assert!(plan.grant.read[0].ends_with("/.config") && !plan.grant.read[0].starts_with('$'));
}

#[test]
fn a_document_refuses_a_bad_schema_or_a_contract_it_does_not_listen_to() {
	let dir = tempfile::tempdir().unwrap();
	for manifest in [
		json!({"name": "p", "entry": "init.lua", "events": {"a": {"schema": {"type": "no-such-type"}}}}),
		json!({"name": "p", "entry": "init.lua", "listen": ["a", "a"]}),
		json!({"name": "p", "entry": "init.lua", "contracts": ["a"]}),
	] {
		write(dir.path(), "cartridge.json", &manifest.to_string());
		assert!(
			crate::loader::Cartridge::document(&dir.path().join("cartridge.json")).is_err(),
			"{manifest}"
		);
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pipe_wakes_the_node_from_outside() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"piped",
		json!({"name": "piped", "entry": "init.lua", "events": {"path": {}, "seen": {}, "echo": {}}, "listen": ["path", "seen", "echo"], "grant": {"exec": ["/bin/sh"]}}),
		r#"local seen = {}
		local path = cartridge.pipe(function(line) table.insert(seen, line) end)
		cartridge.listen("path", function() return path end)
		cartridge.listen("echo", function(data) return data end)
		cartridge.listen("seen", function() return seen end)"#,
	);
	profile(dir.path(), &["piped"]);
	let host = boot(dir.path()).await;
	let path = host.bail("path", json!(null)).await.unwrap().unwrap();
	std::fs::write(
		path.as_str().unwrap(),
		"{\"n\":1}\nplain\n{\"ask\":\"a\",\"bail\":\"echo\",\"args\":\"hi\"}\n",
	)
	.unwrap();
	let mut seen = json!([]);
	for _ in 0..50 {
		seen = host
			.bail("seen", json!(null))
			.await
			.unwrap()
			.unwrap_or_default();
		if seen.as_array().is_some_and(|s| s.len() == 3) {
			break;
		}
		tokio::time::sleep(Duration::from_millis(20)).await;
	}
	assert_eq!(
		seen,
		json!([{"n": 1}, "plain", {"ask": "a", "result": "hi"}])
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_helper_asks_the_base_while_answering() {
	let dir = tempfile::tempdir().unwrap();
	greeter(dir.path());
	// A helper in shell: for the request line it asks `greet`, then answers with what came back.
	write(
		dir.path(),
		"asker/helper.sh",
		r#"#!/bin/sh
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
  printf '{"ask":"a1","bail":"greet","args":{"name":"helper"}}\n'
  IFS= read -r reply
  answer=$(printf '%s' "$reply" | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
  printf '{"id":%s,"result":"%s"}\n' "$id" "$answer"
done
"#,
	);
	cartridge(
		dir.path(),
		"asker",
		json!({"name": "asker", "entry": "init.lua", "events": {"ask": {}}, "needs": ["greet"], "listen": ["ask"], "grant": {"exec": ["/bin/sh", "/usr/bin/sed", "/usr/bin/printf"]}}),
		r#"local app = cartridge.spawn({"/bin/sh", cartridge.root .. "/helper.sh"})
		cartridge.listen("ask", function() return app:request({}) end)"#,
	);
	profile(dir.path(), &["greeter", "asker"]);
	let host = boot(dir.path()).await;
	let answer = host.bail("ask", json!(null)).await.unwrap().unwrap();
	assert_eq!(answer["result"], "hello helper");
	host.stop().await;
}

/// The refusal the whole design rests on, tested where it is enforced: a
/// profile no one trusted fails `entries`, naming the file and the command.
#[tokio::test(flavor = "multi_thread")]
async fn an_untrusted_profile_is_refused_where_it_is_enforced() {
	super::home();
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), ".cartridge/init.lua", "return {}");
	let host = Host::new(dir.path(), dir.path().join(".cartridge")).unwrap();
	let refused = host.entries().unwrap_err().to_string();
	assert!(refused.contains("is in no trusted project"), "{refused}");
	assert!(refused.contains("cartridge trust"), "{refused}");
}
