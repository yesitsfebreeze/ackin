use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use super::{built, write};
use crate::host::{Host, State};
use crate::transport::cartridge::Outcome;

fn cartridge(dir: &Path, id: &str, manifest: Value, init: &str) {
	write(dir, &format!("{id}/cartridge.json"), &manifest.to_string());
	write(dir, &format!("{id}/init.lua"), init);
}

fn descriptor(dir: &Path, ids: &[&str]) {
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

/// A cartridge that did not come up carries why, so an assertion reading
/// only `Failed` sends the reader off to find it.
fn active(host: &Host, id: &str) -> crate::host::Status {
	let status = status(host, id);
	assert_eq!(
		status.state,
		State::Active,
		"{id} is not running: {}",
		status
			.error
			.clone()
			.unwrap_or_else(|| "no reason given".into())
	);
	status
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
	descriptor(dir.path(), &["welcome", "greeter"]);
	let host = boot(dir.path()).await;
	active(&host, "greeter");
	active(&host, "welcome");
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
	descriptor(dir.path(), &["greeter"]);
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
	descriptor(dir.path(), &["greeter"]);
	let host = boot(dir.path()).await;
	assert_eq!(
		host.bail("greet", json!({"name": "you"})).await.unwrap(),
		Some(json!("hello you"))
	);
	let error = host.bail("greet", json!({"name": 7})).await.unwrap_err();
	assert!(error.contains("rejected by its schema"), "{error}");
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
	descriptor(dir.path(), &["stray", "needy"]);
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
	descriptor(dir.path(), &["greeter", "twin"]);
	let host = boot(dir.path()).await;
	active(&host, "greeter");
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
	descriptor(dir.path(), &["greeter", "sneaky"]);
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
	descriptor(dir.path(), &["a", "b", "asker"]);
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
	descriptor(dir.path(), &["greeter", "stranger"]);
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
	descriptor(dir.path(), &["left", "right"]);
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
	descriptor(dir.path(), &["stuck"]);
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
async fn a_waiting_cartridge_starts_when_the_descriptor_adds_its_listener() {
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
	descriptor(dir.path(), &["shadow", "welcome"]);
	let host = boot(dir.path()).await;
	assert_eq!(status(&host, "welcome").state, State::Waiting);

	descriptor(dir.path(), &["greeter", "welcome"]);
	host.reconcile().await.unwrap();
	assert_eq!(status(&host, "shadow").state, State::Disabled);
	active(&host, "welcome");
	assert_eq!(
		host.bail("welcome", json!({"name": "late"})).await.unwrap(),
		Some(json!("hello late"))
	);

	descriptor(dir.path(), &["welcome"]);
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
	descriptor(dir.path(), &["greeter", "native"]);
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
	descriptor(dir.path(), &["echoer"]);
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
	descriptor(dir.path(), &["source", "watcher"]);
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
	descriptor(dir.path(), &["greeter", "welcome"]);
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
	active(&host, "welcome");
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
	descriptor(dir.path(), &["welcome"]);
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
	descriptor(dir.path(), &["greeter", "curious"]);
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
	descriptor(dir.path(), &["greeter"]);
	let host = boot(dir.path()).await;
	let served = tokio::spawn(crate::host::socket::serve(host.clone()));
	let descriptor = host.descriptor().to_path_buf();
	let mut client = None;
	for _ in 0..100 {
		if let Ok(connected) = crate::host::socket::client(&descriptor).await {
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
	descriptor(dir.path(), &["greeter", "tools", "user"]);
	let host = boot(dir.path()).await;
	active(&host, "user");
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
	descriptor(dir.path(), &["busy"]);
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
	descriptor(dir.path(), &["checked"]);
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
	descriptor(dir.path(), &["store"]);
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
	descriptor(dir.path(), &["piped"]);
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
	descriptor(dir.path(), &["greeter", "asker"]);
	let host = boot(dir.path()).await;
	let answer = host.bail("ask", json!(null)).await.unwrap().unwrap();
	assert_eq!(answer["result"], "hello helper");
	host.stop().await;
}

// `grant.env` is the one way a secret from the base's environment reaches a
// node: the composition chooses the name, nothing else comes with it.
#[tokio::test(flavor = "multi_thread")]
async fn a_granted_env_prefix_reaches_the_node_and_nothing_beside_it_does() {
	let dir = tempfile::tempdir().unwrap();
	std::env::set_var("ENVTEST_ALLOWED_TOKEN", "granted");
	std::env::set_var("ENVTEST_OTHER_TOKEN", "withheld");
	cartridge(
		dir.path(),
		"reader",
		json!({
			"name": "reader", "entry": "init.lua",
			"events": {"env": {}}, "listen": ["env"],
			"grant": {"exec": ["/usr/bin/env"], "env": ["ENVTEST_ALLOWED_*"]},
		}),
		r#"local lines = {}
			local env = cartridge.spawn({"/usr/bin/env"})
			env:on_line(function(line) table.insert(lines, line) end)
			cartridge.listen("env", function() return lines end)"#,
	);
	descriptor(dir.path(), &["reader"]);
	let host = boot(dir.path()).await;
	let mut lines = json!([]);
	for _ in 0..50 {
		lines = host
			.bail("env", json!(null))
			.await
			.unwrap()
			.unwrap_or_default();
		if lines.as_array().is_some_and(|s| !s.is_empty()) {
			break;
		}
		tokio::time::sleep(Duration::from_millis(20)).await;
	}
	let lines = lines.as_array().expect("env printed its environment");
	let has = |name: &str| {
		lines
			.iter()
			.any(|line| line.as_str().unwrap_or_default().starts_with(name))
	};
	assert!(
		has("ENVTEST_ALLOWED_TOKEN=granted"),
		"the granted prefix reaches the node: {lines:?}"
	);
	assert!(
		!has("ENVTEST_OTHER_TOKEN"),
		"a variable outside the grant does not: {lines:?}"
	);
	host.stop().await;
}

// Only `CARTRIDGE_BIN` and `CARTRIDGE_HOME` are deliberate passthroughs
// (paths, no credential); every other `CARTRIDGE_*` — this node's own and any
// the operator's shell held — must not reach a spawned helper.
#[tokio::test(flavor = "multi_thread")]
async fn a_spawned_helper_sees_only_the_two_named_cartridge_variables_and_a_path() {
	let dir = tempfile::tempdir().unwrap();
	std::env::set_var("CARTRIDGE_TEST_SECRET", "host-admin-leak");
	cartridge(
		dir.path(),
		"spiller",
		json!({
			"name": "spiller", "entry": "init.lua",
			"events": {"env": {}}, "listen": ["env"],
			"grant": {"exec": ["/usr/bin/env"]},
		}),
		r#"local lines = {}
			local env = cartridge.spawn({"/usr/bin/env"})
			env:on_line(function(line) table.insert(lines, line) end)
			cartridge.listen("env", function() return lines end)"#,
	);
	descriptor(dir.path(), &["spiller"]);
	let host = boot(dir.path()).await;
	let mut lines = json!([]);
	for _ in 0..50 {
		lines = host
			.bail("env", json!(null))
			.await
			.unwrap()
			.unwrap_or_default();
		if lines.as_array().is_some_and(|s| !s.is_empty()) {
			break;
		}
		tokio::time::sleep(Duration::from_millis(20)).await;
	}
	let lines = lines.as_array().expect("env printed its environment");
	let leaked: Vec<&str> = lines
		.iter()
		.filter_map(|line| line.as_str())
		.filter(|line| line.starts_with("CARTRIDGE_"))
		.filter(|line| {
			!["CARTRIDGE_BIN=", "CARTRIDGE_HOME="]
				.iter()
				.any(|named| line.starts_with(named))
		})
		.collect();
	assert!(
		leaked.is_empty(),
		"only CARTRIDGE_BIN and CARTRIDGE_HOME reach a helper: {leaked:?}"
	);
	assert!(
		lines
			.iter()
			.any(|line| line.as_str().unwrap_or_default().starts_with("PATH=")),
		"PATH is passed through: {lines:?}"
	);
	host.stop().await;
}

// A node's own credential is worth its socket only: it does not authenticate
// on the host socket.
#[tokio::test(flavor = "multi_thread")]
async fn a_node_presenting_its_own_credential_is_refused_on_the_host_socket() {
	let dir = tempfile::tempdir().unwrap();
	greeter(dir.path());
	descriptor(dir.path(), &["greeter"]);
	let host = boot(dir.path()).await;
	let token = host.node_token("greeter");
	assert!(!token.is_empty(), "every started slot holds a node token");
	assert_ne!(token, host.host_token());
	let served = tokio::spawn(crate::host::socket::serve(host.clone()));
	let socket = host.socket_path();
	let mut refused = String::new();
	for _ in 0..200 {
		if let Err(error) = crate::host::connect(&socket, &token).await {
			refused = error.to_string();
			if refused.contains("unknown token") {
				break;
			}
		}
		tokio::time::sleep(Duration::from_millis(10)).await;
	}
	assert!(
		refused.contains("unknown token"),
		"a node token never authenticates on the host socket: {refused}"
	);
	let descriptor = host.descriptor().to_path_buf();
	for _ in 0..100 {
		if let Ok(connected) = crate::host::socket::client(&descriptor).await {
			let (peer, _incoming) = connected;
			let status = peer.call("status", json!(null)).await.unwrap();
			assert_eq!(status[0]["state"], "active");
			peer.call("stop", json!(null)).await.unwrap();
			break;
		}
		tokio::time::sleep(Duration::from_millis(10)).await;
	}
	served.abort();
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_untrusted_descriptor_is_refused_where_it_is_enforced() {
	super::home();
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), ".cartridge/init.lua", "return {}");
	let host = Host::new(dir.path(), dir.path().join(".cartridge")).unwrap();
	let refused = host.entries().unwrap_err().to_string();
	assert!(refused.contains("is in no trusted project"), "{refused}");
	assert!(refused.contains("cartridge trust"), "{refused}");
}

// `entries` runs the socket-name-clash check on the solo list too, not only
// on the derived-plus-descriptor list.
#[tokio::test(flavor = "multi_thread")]
async fn solo_entries_sharing_a_socket_name_are_refused() {
	let dir = tempfile::tempdir().unwrap();
	let host = Host::new(dir.path(), dir.path().join(".cartridge")).unwrap();
	*host.solo.lock() = Some(vec![
		crate::loader::Entry {
			id: "vendor/x".into(),
			path: "vendor/x".into(),
			config: json!(null),
			disabled: false,
		},
		crate::loader::Entry {
			id: "vendor_x".into(),
			path: "vendor_x".into(),
			config: json!(null),
			disabled: false,
		},
	]);
	let refused = host.entries().unwrap_err().to_string();
	assert!(refused.contains("share the socket name"), "{refused}");
}

// No 60 s deadline, no wedged state: a spinning handler is refused directly.
#[tokio::test(flavor = "multi_thread")]
async fn a_spinning_handler_is_refused_and_its_node_keeps_serving() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"spin",
		json!({"name": "spin", "entry": "init.lua",
			"events": {"spin": {}, "ping": {}}, "listen": ["spin", "ping"]}),
		r#"cartridge.listen("spin", function() while true do end end)
			cartridge.listen("ping", function() return "pong" end)"#,
	);
	descriptor(dir.path(), &["spin"]);
	let host = boot(dir.path()).await;
	let error = host
		.bail("spin", json!(null))
		.await
		.unwrap_err()
		.to_string();
	assert!(error.contains("Lua instructions"), "{error}");
	assert_eq!(
		host.bail("ping", json!(null)).await.unwrap(),
		Some(json!("pong"))
	);
	active(&host, "spin");
	host.stop().await;
}

// Nothing inside Lua can stop a node that catches its own refusal and loops;
// it ends itself with status 70.
#[tokio::test(flavor = "multi_thread")]
async fn a_node_that_catches_its_own_refusal_exits() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"loop",
		json!({"name": "loop", "entry": "init.lua",
			"events": {"spin": {}, "ping": {}}, "listen": ["spin", "ping"]}),
		r#"cartridge.listen("spin", function() while true do pcall(function() while true do end end) end end)
			cartridge.listen("ping", function() return "pong" end)"#,
	);
	descriptor(dir.path(), &["loop"]);
	let host = boot(dir.path()).await;
	assert!(host.bail("spin", json!(null)).await.is_err());
	let mut spin = status(&host, "loop");
	for _ in 0..100 {
		if spin.state == State::Failed {
			break;
		}
		tokio::time::sleep(Duration::from_millis(100)).await;
		spin = status(&host, "loop");
	}
	assert_eq!(spin.state, State::Failed, "{:?}", spin.error);
	assert!(
		spin.error.unwrap().contains("exited: exit status: 70"),
		"a runaway node leaves with status 70"
	);
	host.stop().await;
}

#[tokio::test]
async fn a_lifecycle_subscriber_that_falls_behind_is_disconnected() {
	let (a, b) = crate::transport::typed::InprocAdapter::pair();
	let (peer, _incoming) = crate::transport::rpc::Peer::spawn(a, None);
	let (_other, _theirs) = crate::transport::rpc::Peer::spawn(b, None);
	let (tx, events) = tokio::sync::broadcast::channel::<Value>(2);
	for i in 0..5 {
		tx.send(json!(i)).unwrap();
	}
	let forwarded = tokio::time::timeout(
		Duration::from_secs(5),
		crate::host::socket::forward(events, peer.clone()),
	)
	.await;
	assert!(forwarded.is_ok(), "forwarding outlived the lag");
	assert!(peer.is_closed(), "a lagging subscriber kept its connection");
	drop(tx);
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread")]
async fn a_spawn_the_grant_did_not_name_is_denied() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"echoer",
		json!({"name": "echoer", "entry": "init.lua", "events": {"echo": {}}, "listen": ["echo"]}),
		r#"local cat = cartridge.spawn({"/bin/cat"})
		cartridge.listen("echo", function(data) return cat:request({ data = data }) end)"#,
	);
	descriptor(dir.path(), &["echoer"]);
	let host = boot(dir.path()).await;
	assert_eq!(
		status(&host, "echoer").state,
		State::Failed,
		"an exec the grant did not name must be denied by the sandbox"
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn one_cartridge_cannot_see_anothers_globals() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"leaker",
		json!({"name": "leaker", "entry": "init.lua", "events": {"leak": {}}, "listen": ["leak"]}),
		r#"SECRET = "leaked"
		cartridge.listen("leak", function() return SECRET end)"#,
	);
	cartridge(
		dir.path(),
		"peeker",
		json!({"name": "peeker", "entry": "init.lua", "events": {"peek": {}}, "listen": ["peek"]}),
		r#"cartridge.listen("peek", function() return tostring(SECRET) end)"#,
	);
	descriptor(dir.path(), &["leaker", "peeker"]);
	let host = boot(dir.path()).await;
	assert_eq!(
		host.bail("leak", json!({})).await.unwrap(),
		Some(json!("leaked")),
		"the leaker must be running, or this proves nothing"
	);
	assert_eq!(
		host.bail("peek", json!({})).await.unwrap(),
		Some(json!("nil")),
		"each cartridge runs in its own interpreter"
	);
	host.stop().await;
}

// Stopping a cartridge kills its whole process group (a job object on
// Windows), so a spawned program does not outlive it.
#[cfg(any(target_os = "macos", target_os = "windows"))]
#[tokio::test(flavor = "multi_thread")]
async fn a_stopped_cartridge_takes_its_programs_along() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"parent",
		json!({"name": "parent", "entry": "init.lua",
			"events": {"pid": {}}, "listen": ["pid"], "grant": {"exec": ["*"]}}),
		r#"local pid = 0
local sh = cartridge.spawn({"/bin/sh", "-c", "/bin/sleep 300 & echo $!; wait"})
sh:on_line(function(line) pid = tonumber(line) end)
cartridge.listen("pid", function() return pid end)"#,
	);
	descriptor(dir.path(), &["parent"]);
	let host = boot(dir.path()).await;
	let mut pid = 0;
	for _ in 0..100 {
		pid = host
			.bail("pid", json!(null))
			.await
			.unwrap()
			.and_then(|v| v.as_i64())
			.unwrap_or(0);
		if pid > 0 {
			break;
		}
		tokio::time::sleep(Duration::from_millis(20)).await;
	}
	assert!(pid > 0, "the program never reported its child");
	host.stop().await;
	let pid = pid as u32;
	let gone = (0..100).any(|_| {
		std::thread::sleep(Duration::from_millis(20));
		!alive(pid)
	});
	if !gone {
		kill_now(pid);
	}
	assert!(gone, "a program's child outlived its cartridge");
}

// A stopped run returns `Stopped` rather than waiting out the event deadline.
#[tokio::test(flavor = "multi_thread")]
async fn a_stop_request_ends_a_foreground_run() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"stuck",
		json!({"name": "stuck", "entry": "init.lua",
			"events": {"hang": {}}, "listen": ["hang"], "grant": {"exec": ["/bin/sleep"]}}),
		r#"local sleeper = cartridge.spawn({"/bin/sleep", "60"}, {timeout_ms = 60000})
cartridge.listen("hang", function() return sleeper:request({}) end)"#,
	);
	descriptor(dir.path(), &["stuck"]);
	let host = boot(dir.path()).await;
	let run = tokio::spawn({
		let host = host.clone();
		async move { host.run("hang", json!(null)).await }
	});
	host.stop_signal().cancel();
	let ended = tokio::time::timeout(Duration::from_secs(10), run)
		.await
		.expect("a stopped run returns")
		.unwrap();
	assert!(matches!(ended, Err(crate::Error::Stopped)), "{ended:?}");
	assert_ne!(status(&host, "stuck").state, State::Active);
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread")]
async fn a_terminated_mcp_exits_with_its_input_still_open() {
	use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"tools",
		json!({"name": "tools", "entry": "init.lua", "events": {"mcp": {}}, "listen": ["mcp"]}),
		r#"cartridge.listen("mcp", function() return {ok = true} end)"#,
	);
	descriptor(dir.path(), &["tools"]);
	node_binary();
	let mut mcp = tokio::process::Command::new(built(&["--bin", "cartridge"]))
		.args(["--dir", ".", "mcp"])
		.current_dir(dir.path())
		.stdin(std::process::Stdio::piped())
		.stdout(std::process::Stdio::piped())
		.stderr(std::process::Stdio::null())
		.kill_on_drop(true)
		.spawn()
		.unwrap();
	let mut input = mcp.stdin.take().unwrap();
	input
		.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n")
		.await
		.unwrap();
	let mut lines = tokio::io::BufReader::new(mcp.stdout.take().unwrap()).lines();
	tokio::time::timeout(Duration::from_secs(60), lines.next_line())
		.await
		.expect("mcp serves")
		.unwrap();
	// SAFETY: only signals the child this test spawned.
	unsafe { libc::kill(mcp.id().unwrap() as libc::pid_t, libc::SIGTERM) };
	let exited = tokio::time::timeout(Duration::from_secs(20), mcp.wait()).await;
	drop(input);
	assert!(exited.is_ok(), "mcp held its exit on an open stdin");
}

#[cfg(target_os = "macos")]
fn alive(pid: u32) -> bool {
	// SAFETY: signal 0 only checks that the process exists.
	unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

#[cfg(target_os = "macos")]
fn kill_now(pid: u32) {
	// SAFETY: the pid was reported by a child of this test.
	unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
}

#[cfg(windows)]
fn alive(pid: u32) -> bool {
	use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
	use windows_sys::Win32::System::Threading::{
		GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
	};
	// SAFETY: a query-only handle, closed below; a dead or unknown pid opens
	// nothing. A pid that still has an exit code is a handle, not a process.
	unsafe {
		let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
		if handle.is_null() {
			return false;
		}
		let mut code = 0u32;
		let read = GetExitCodeProcess(handle, &mut code);
		CloseHandle(handle);
		read != 0 && code == STILL_ACTIVE as u32
	}
}

#[cfg(windows)]
fn kill_now(pid: u32) {
	use windows_sys::Win32::Foundation::CloseHandle;
	use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
	// SAFETY: the pid was reported by a child of this test.
	unsafe {
		let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
		if !handle.is_null() {
			TerminateProcess(handle, 1);
			CloseHandle(handle);
		}
	}
}
