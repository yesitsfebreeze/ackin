use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use super::{built, write};
use crate::host::{Host, State};

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
}

fn greeter(dir: &Path) {
	cartridge(
		dir,
		"greeter",
		json!({
			"name": "greeter", "entry": "init.lua",
			"events": {"greet": {"description": "a greeting", "schema": {"type": "object", "required": ["name"], "properties": {"name": {"type": "string"}}}}},
			"on": ["greet"],
		}),
		r#"cartridge.on("greet", function(args) return "hello " .. args.name end)"#,
	);
}

/// The test binary is not the base; nodes run the built `cartridge`.
fn node_binary() {
	static BIN: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	let bin = BIN.get_or_init(|| built(&["--bin", "cartridge"]));
	std::env::set_var(crate::host::NODE_BIN_ENV, bin);
}

async fn boot(dir: &Path) -> Arc<Host> {
	node_binary();
	let host = Host::new(dir, dir.join(".cartridge"), false).unwrap();
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
			"events": {"welcome": {}}, "needs": ["greet"], "on": ["welcome"],
		}),
		r#"return { apply = function(ctx, config)
			ctx.on("welcome", function(args) return ctx.bail("greet", args) end)
		end }"#,
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
async fn an_undeclared_event_fails_the_cartridge_before_it_starts() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"stray",
		json!({"name": "stray", "entry": "init.lua", "on": ["nobody.declares"]}),
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
		json!({"name": "sneaky", "entry": "init.lua", "events": {"sneak": {}}, "on": ["sneak"]}),
		r#"cartridge.on("sneak", function()
			local ok, error = pcall(cartridge.on, "greet", function() end)
			return tostring(error)
		end)"#,
	);
	profile(dir.path(), &["greeter", "sneaky"]);
	let host = boot(dir.path()).await;
	let answer = host.bail("sneak", json!(null)).await.unwrap().unwrap();
	assert!(
		answer.as_str().unwrap().contains("not declared in `on`"),
		"{answer}"
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn gather_parallel_and_emit_reach_every_listener() {
	let dir = tempfile::tempdir().unwrap();
	for id in ["a", "b"] {
		cartridge(
			dir.path(),
			id,
			json!({"name": id, "entry": "init.lua", "on": ["ping"]}),
			&format!(
				r#"cartridge.on("ping", function(data) return {{ from = {id:?}, data = data }} end)"#
			),
		);
	}
	cartridge(
		dir.path(),
		"asker",
		json!({"name": "asker", "entry": "init.lua", "events": {"ping": {}, "ask": {}}, "on": ["ask"]}),
		r#"cartridge.on("ask", function(data)
			local all = cartridge.gather("ping", data)
			cartridge.parallel("ping", data)
			cartridge.emit("ping", data)
			return { count = #all, first = all[1].from, second = all[2].from }
		end)"#,
	);
	profile(dir.path(), &["a", "b", "asker"]);
	let host = boot(dir.path()).await;
	assert_eq!(
		host.bail("ask", json!(1)).await.unwrap(),
		Some(json!({"count": 2, "first": "a", "second": "b"}))
	);
	let answers = host.emit("ping", json!(2)).await.unwrap();
	assert_eq!(answers.len(), 2);
	assert_eq!(
		answers[0],
		("a".to_owned(), Ok(json!({"from": "a", "data": 2})))
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
			"needs": ["greet"], "on": ["twice", "relay"],
		}),
		r#"local native = cartridge.load("native_fixture")
		cartridge.on("twice", function(n) return native.twice(n) end)
		cartridge.on("relay", function(args) return native.ask("greet", args) end)"#,
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
			"events": {"echo": {}}, "on": ["echo"],
			"grant": {"exec": ["/bin/cat"]},
		}),
		r#"local cat = cartridge.spawn({"/bin/cat"})
		cartridge.on("echo", function(data) return cat:request({ data = data }) end)"#,
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
		json!({"name": "source", "entry": "init.lua", "events": {"tick": {}}, "on": ["tick"]}),
		r#"cartridge.on("tick", function(n) cartridge.publish("ticks", n) return true end)"#,
	);
	cartridge(
		dir.path(),
		"watcher",
		json!({"name": "watcher", "entry": "init.lua", "events": {"seen": {}}, "needs": ["tick"], "on": ["seen"]}),
		r#"local seen = {}
		cartridge.subscribe("source", "ticks", function(envelope)
			if envelope.kind == "data" then table.insert(seen, envelope.data) end
		end)
		cartridge.on("seen", function() return seen end)"#,
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
		json!({"name": "welcome", "entry": "init.lua", "events": {"welcome": {}}, "needs": ["greet"], "on": ["welcome"]}),
		r#"cartridge.on("welcome", function(args) return cartridge.bail("greet", args) end)"#,
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
		r#"cartridge.on("greet", function(args) return "hi " .. args.name end)"#,
	);
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
		json!({"name": "welcome", "entry": "init.lua", "events": {"greet": {}, "welcome": {}}, "needs": ["greet"], "on": ["welcome"]}),
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
		json!({"name": "curious", "entry": "init.lua", "events": {"ask": {}}, "on": ["ask"]}),
		r#"cartridge.on("ask", function()
			local cartridges = cartridge.host("cartridges", nil)
			local snapshot = cartridge.host("snapshot", nil)
			local ok, refused = pcall(cartridge.host, "bridge.status", nil)
			return { count = #cartridges, entries = #snapshot.entries, bridge = ok, refused = tostring(refused),
				needs = cartridge.needs(), events = cartridge.events() }
		end)"#,
	);
	profile(dir.path(), &["greeter", "curious"]);
	let host = boot(dir.path()).await;
	let answer = host.bail("ask", json!(null)).await.unwrap().unwrap();
	assert_eq!(answer["count"], 2);
	assert_eq!(answer["entries"], 2);
	assert_eq!(answer["bridge"], false);
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
async fn verify_sends_every_declared_contract() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"checked",
		json!({"name": "checked", "entry": "init.lua", "events": {"checked.ok": {}}, "on": ["checked.ok"], "selftest": "checked.ok"}),
		r#"cartridge.on("checked.ok", function() return true end)"#,
	);
	profile(dir.path(), &["checked"]);
	node_binary();
	let host = Host::new(dir.path(), dir.path().join(".cartridge"), false).unwrap();
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
	let host = Host::new(dir.path(), dir.path().join(".cartridge"), false).unwrap();
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
		json!({"name": "p", "entry": "init.lua", "on": ["a", "a"]}),
		json!({"name": "p", "entry": "init.lua", "selftest": "a"}),
	] {
		write(dir.path(), "cartridge.json", &manifest.to_string());
		assert!(
			crate::loader::Cartridge::document(&dir.path().join("cartridge.json")).is_err(),
			"{manifest}"
		);
	}
}
