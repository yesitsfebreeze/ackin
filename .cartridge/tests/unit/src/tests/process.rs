use super::{boot, next_event, settle, write};
use crate::runtime::State;
use serde_json::json;

const SCRIPT: &str = r#"#!/bin/sh
if [ "$1" = hello ]; then echo '{"provide":["twice"]}'; exit 0; fi
echo '{"provide":"twice"}'
echo '{"on":"ping"}'
echo '{"ready":true}'
while read line; do
	id=$(printf '%s' "$line" | sed 's/.*"id":\([0-9]*\).*/\1/')
	case "$line" in
		*'"call"'*) n=$(printf '%s' "$line" | sed 's/.*"args":\([0-9]*\).*/\1/'); echo "{\"reply\":$id,\"data\":$((n*2))}";;
		*'"event"'*) d=$(printf '%s' "$line" | sed 's/.*"data":\([0-9]*\).*/\1/'); echo "{\"send\":\"pong\",\"data\":$d}"; echo "{\"reply\":$id,\"data\":null}";;
		*'"dispose"'*) exit 0;;
	esac
done
"#;

#[tokio::test(flavor = "multi_thread")]
async fn a_process_provides_listens_and_is_disposed() {
	use std::os::unix::fs::PermissionsExt;
	let dir = tempfile::tempdir().unwrap();
	let script = dir.path().join("twice.sh");
	write(dir.path(), "twice.sh", SCRIPT);
	std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
	write(
		dir.path(),
		"user.lua",
		r#"return { inject = {"twice"}, apply = function(ctx) ctx:send("got", ctx.twice(21)) end }"#,
	);
	write(
		dir.path(),
		"process.lua",
		&format!("return cartridge.process({})", json!(script)),
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{ id = "p", path = "process.lua" }, { id = "u", path = "user.lua" }}"#,
	);
	let (host, mut rx) = boot(dir.path()).await;
	assert_eq!(next_event(&mut rx, "got").await, json!(42));
	host.emit("ping", json!(5));
	assert_eq!(next_event(&mut rx, "pong").await, json!(5));
	assert_eq!(host.call("twice", json!(4)).await, Ok(json!(8)));
	host.fiber_of("p").unwrap().dispose().await;
	settle().await;
	assert_eq!(host.fiber_of("u").unwrap().state(), Some(State::Inactive));
}

pub(super) fn sdk_fixture() -> std::path::PathBuf {
	static BINARY: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	BINARY
		.get_or_init(|| super::built(&["-p", "cartridge", "--example", "rpc_fixture"]))
		.clone()
}

/// A process that provides `lua`, the isolated alternate provider in the
/// composition regression tests.
pub(super) fn lua_fixture() -> std::path::PathBuf {
	static BINARY: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	BINARY
		.get_or_init(|| super::built(&["-p", "cartridge", "--example", "lua_fixture"]))
		.clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn sdk_child_roundtrip_errors_metadata_and_eof() {
	let binary = sdk_fixture();
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"lua.lua",
		r#"return { provide = {"lua"}, apply = function(ctx)
		ctx:provide("lua", function(args)
			if args == "error" then error("lua service failed") end
			return { value = args }
		end)
	end }"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{ id = "lua", path = "lua.lua" }}"#,
	);
	let (host, mut rx) = boot(dir.path()).await;
	settle().await;
	let component = crate::cartridge::component(
		host.clone(),
		"sdk".into(),
		vec![binary.to_string_lossy().into_owned()],
		json!({}),
	)
	.unwrap();
	let fiber = host
		.runtime()
		.ctx()
		.intercept("lua", json!({"description": "Lua service"}))
		.cartridge(component);
	fiber.settled().await;
	let call = |args| {
		tokio::time::timeout(
			std::time::Duration::from_secs(5),
			host.call("roundtrip", args),
		)
	};
	// Both links start at ID 1. The host waits for roundtrip while the child
	// waits for Lua; confusing the namespaces returns the unwrapped Lua data.
	assert_eq!(
		call(json!(21)).await.unwrap().unwrap(),
		json!({"data": {"value": 21}, "meta": {"description": "Lua service"}, "absent": null})
	);
	let error = call(json!("error")).await.unwrap().unwrap_err();
	assert!(error.contains("lua service failed"), "{error}");
	let (a, b) = tokio::join!(call(json!(1)), call(json!(2)));
	assert_eq!(a.unwrap().unwrap()["data"]["value"], 1);
	assert_eq!(b.unwrap().unwrap()["data"]["value"], 2);
	assert_eq!(
		call(json!("exit")).await.unwrap().unwrap_err(),
		"cartridge is gone"
	);
	fiber.dispose().await;
	let component = crate::cartridge::component(
		host.clone(),
		"sdk-apply".into(),
		vec![binary.to_string_lossy().into_owned()],
		json!({"apply_call": true}),
	)
	.unwrap();
	let fiber = host.runtime().ctx().cartridge(component);
	assert_eq!(next_event(&mut rx, "apply").await, json!({"value": 7}));
	fiber.dispose().await;
}

#[tokio::test]
async fn closed_links_release_pending_and_reject_new_requests() {
	let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
	let link = crate::cartridge::Link::new(tx, "peer gone");
	let waiting = link.request(json!({"call": "wait"}));
	tokio::pin!(waiting);
	assert!(futures::poll!(&mut waiting).is_pending());
	assert_eq!(rx.recv().await.unwrap().unwrap()["id"], 1);
	link.close();
	assert_eq!(waiting.await, Err("peer gone".into()));
	assert_eq!(
		link.request(json!({"call": "late"})).await,
		Err("peer gone".into())
	);
}

#[tokio::test]
async fn dropping_a_waiter_ignores_its_late_reply() {
	let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
	let link = crate::cartridge::Link::new(tx, "peer gone");
	let mut waiting = Box::pin(link.request(json!({"call": "cancel"})));
	assert!(futures::poll!(&mut waiting).is_pending());
	let id = rx.recv().await.unwrap().unwrap()["id"].as_u64().unwrap();
	drop(waiting);
	assert_eq!(link.pending_count(), 0);
	let mut waiting = Box::pin(link.request(json!({"call": "next"})));
	assert!(futures::poll!(&mut waiting).is_pending());
	let next = rx.recv().await.unwrap().unwrap()["id"].as_u64().unwrap();
	link.answer(id, Ok(json!("late")));
	assert!(futures::poll!(&mut waiting).is_pending());
	link.answer(next, Ok(json!("correct")));
	assert_eq!(waiting.await, Ok(json!("correct")));
	assert_eq!(link.pending_count(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn sdk_eof_and_dispose_release_waiters_before_finalizers() {
	use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
	for dispose in [false, true] {
		let mut child = tokio::process::Command::new(sdk_fixture())
			.stdin(std::process::Stdio::piped())
			.stdout(std::process::Stdio::piped())
			.kill_on_drop(true)
			.spawn()
			.unwrap();
		let mut input = child.stdin.take().unwrap();
		let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
		input
			.write_all(b"{\"apply\":{\"config\":{\"shutdown\":true}}}\n")
			.await
			.unwrap();
		let run = async {
			loop {
				let line = lines.next_line().await.unwrap().unwrap();
				let message: serde_json::Value = serde_json::from_str(&line).unwrap();
				if message["call"] == "lua" {
					break;
				}
			}
			if dispose {
				input.write_all(b"{\"dispose\":true}\n").await.unwrap();
			}
			drop(input);
			loop {
				let line = lines.next_line().await.unwrap().expect("finalizer output");
				let message: serde_json::Value = serde_json::from_str(&line).unwrap();
				if message["send"] == "finished" {
					assert_eq!(
						message["data"],
						json!({"pending":"host is gone", "late":"host is gone"})
					);
					break;
				}
			}
			assert!(child.wait().await.unwrap().success());
		};
		tokio::time::timeout(std::time::Duration::from_secs(5), run)
			.await
			.unwrap();
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn lua_wrappers_merge_injections_before_start_and_preserve_config() {
	let binary = sdk_fixture();
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"provider.lua",
		r#"return { provide = {"lua", "secret"}, apply = function(ctx)
		ctx:provide("lua", function(args) return args end)
		ctx:provide("secret", 99)
	end }"#,
	);
	write(
		dir.path(),
		"process.lua",
		&format!(
			r#"
		local command = {{{}}}
		local function wrap() return cartridge.process(command, {{ inject = {{"lua", "lua"}} }}) end
		return wrap()
	"#,
			json!(binary)
		),
	);
	write(
		dir.path(),
		"init.lua",
		r#"
		local tools = {"tool.extra"}
		return {{id="provider", path="provider.lua"},
			{id="p", path="process.lua", inject={"lua", tools[1], tools[1]},
			config={n=1, keep=true, tools=tools, inject={"secret"}}}}
	"#,
	);
	write(dir.path(), "config.lua", r#"return {p={n=2}}"#);
	let (host, _) = boot(dir.path()).await;
	settle().await;
	assert_eq!(host.fiber_of("p").unwrap().state(), Some(State::Inactive));
	let manifest = host.manifest().unwrap();
	assert_eq!(manifest[1].inject, vec!["lua", "tool.extra"]);
	assert_eq!(manifest[1].provide, vec!["roundtrip"]);
	use futures::StreamExt;
	let provider = crate::runtime::Component::new(
		"extra",
		std::sync::Arc::new(|ctx| {
			futures::stream::once(async move {
				ctx.provide("tool.extra", std::sync::Arc::new(json!(42)))?;
				let disposer: crate::runtime::Disposer = Box::new(|| Box::pin(async {}));
				Ok(disposer)
			})
			.boxed()
		}),
	)
	.provide(["tool.extra"]);
	let extra = host.runtime().ctx().cartridge(provider);
	extra.settled().await;
	host.fiber_of("p").unwrap().settled().await;
	let call = |args| {
		tokio::time::timeout(
			std::time::Duration::from_secs(5),
			host.call("roundtrip", args),
		)
	};
	assert_eq!(
		call(json!({"key":"tool.extra"})).await.unwrap().unwrap()["data"],
		42
	);
	assert_eq!(
		call(json!("config")).await.unwrap().unwrap(),
		json!({"n":2,"keep":true,"tools":["tool.extra"],"inject":["secret"]})
	);
	let error = call(json!({"key":"secret"})).await.unwrap().unwrap_err();
	assert!(error.contains("undeclared access"), "{error}");
	extra.dispose().await;
	settle().await;
	assert_eq!(host.fiber_of("p").unwrap().state(), Some(State::Inactive));
	host.fiber_of("p").unwrap().dispose().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn disabled_processes_never_run_hello_apply_or_replace() {
	use std::os::unix::fs::PermissionsExt;
	let dir = tempfile::tempdir().unwrap();
	let marker = dir.path().join("spawned");
	let script = dir.path().join("mark.sh");
	write(
		dir.path(),
		"mark.sh",
		&format!("#!/bin/sh\ntouch '{}'\necho '{{}}'\n", marker.display()),
	);
	std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
	write(
		dir.path(),
		"p.lua",
		&format!("return cartridge.process({})", json!(script)),
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="p",path="p.lua",disabled=true}}"#,
	);
	let (host, _) = boot(dir.path()).await;
	assert!(host.manifest().unwrap()[0].error.is_none());
	write(
		dir.path(),
		"p.lua",
		&format!("-- changed\nreturn cartridge.process({})", json!(script)),
	);
	host.replace(&dir.path().join("p.lua")).await;
	assert!(host.fiber_of("p").is_none());
	assert!(!marker.exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_registration_fails_before_changing_loaded_fibers() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "p.lua", "return {apply=function() end}");
	write(dir.path(), "init.lua", r#"return {{id="p",path="p.lua"}}"#);
	let (host, _) = boot(dir.path()).await;
	let original = host.fiber_of("p").unwrap().uid();
	for entry in [
		r#"{id="p",cmd="program"}"#,
		r#"{id="p",path="p.lua",cmd="program"}"#,
		r#"{id="p",path="p.lua",cmd="program",disabled=true}"#,
	] {
		write(dir.path(), "init.lua", &format!("return {{{entry}}}"));
		let error = host.reconcile().await.unwrap_err().to_string();
		assert!(error.contains("unknown field `cmd`"), "{error}");
		assert_eq!(host.fiber_of("p").unwrap().uid(), original);
	}
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="p",path="p.lua"},{id="p",path="p.lua"}}"#,
	);
	assert!(host
		.reconcile()
		.await
		.unwrap_err()
		.to_string()
		.contains("duplicate entry id"));
	assert_eq!(host.fiber_of("p").unwrap().uid(), original);
	for source in [
		"return cartridge.process('')",
		"return cartridge.process('/does/not/exist')",
		"return {inject={'tool.*'},apply=function() end}",
		"return {provide={'key','key'},apply=function() end}",
		"return {inject={3},apply=function() end}",
	] {
		write(dir.path(), "bad.lua", source);
		assert!(
			host.component(&dir.path().join("bad.lua"), json!({}))
				.is_err(),
			"{source}"
		);
	}
}

fn watched_script(dir: &std::path::Path, version: u8, log: &std::path::Path) -> std::path::PathBuf {
	use std::os::unix::fs::PermissionsExt;
	let script = dir.join("service.sh");
	let next = dir.join("service.next");
	let body = format!(
		r#"#!/bin/sh
if [ "$1" = hello ]; then echo 'hello:{version}' >> '{log}'; echo '{{"provide":["watch"]}}'; exit 0; fi
echo 'start:{version}' >> '{log}'
echo '{{"provide":"watch"}}'
echo '{{"ready":true}}'
echo '{{"send":"version","data":{version}}}'
while read line; do
case "$line" in
*'"dispose"'*) echo 'stop:{version}' >> '{log}'; exit 0;;
esac
done
"#,
		log = log.display()
	);
	std::fs::write(&next, body).unwrap();
	std::fs::set_permissions(&next, std::fs::Permissions::from_mode(0o755)).unwrap();
	std::fs::rename(&next, &script).unwrap();
	script
}

#[tokio::test(flavor = "multi_thread")]
async fn watches_reload_wrappers_binaries_and_new_executable_directories_once() {
	let dir = tempfile::tempdir().unwrap();
	let first_bin = tempfile::tempdir().unwrap();
	let second_bin = tempfile::tempdir().unwrap();
	let log = dir.path().join("lifecycle.log");
	let first = watched_script(first_bin.path(), 1, &log);
	write(
		dir.path(),
		"p.lua",
		&format!("return cartridge.process({})", json!(first)),
	);
	write(
		dir.path(),
		"dependent.lua",
		&format!(
			r#"return {{inject={{"watch"}},apply=function(ctx)
		local function log(line) local f=assert(io.open({}, "a")); f:write(line .. "\n"); f:close() end
		log("dependent-start")
		return function() log("dependent-stop") end
	end}}"#,
			json!(log)
		),
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="p",path="p.lua"},{id="d",path="dependent.lua"}}"#,
	);
	let (host, mut rx) = boot(dir.path()).await;
	assert_eq!(next_event(&mut rx, "version").await, 1);
	host.watch().unwrap();
	settle().await;
	write(
		dir.path(),
		"p.lua",
		&format!(
			"-- wrapper edit\nreturn cartridge.process({})",
			json!(first)
		),
	);
	assert_eq!(next_event(&mut rx, "version").await, 1);
	watched_script(first_bin.path(), 2, &log);
	assert_eq!(next_event(&mut rx, "version").await, 2);
	let second = watched_script(second_bin.path(), 3, &log);
	write(
		dir.path(),
		"new.lua",
		&format!("return cartridge.process({})", json!(second)),
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="p",path="new.lua"},{id="d",path="dependent.lua"}}"#,
	);
	assert_eq!(next_event(&mut rx, "version").await, 3);
	settle().await;
	watched_script(second_bin.path(), 4, &log);
	assert_eq!(next_event(&mut rx, "version").await, 4);
	// A wrapper and its binary changing together still replace only one fiber.
	write(
		dir.path(),
		"new.lua",
		&format!(
			"-- paired edit\nreturn cartridge.process({})",
			json!(second)
		),
	);
	watched_script(second_bin.path(), 5, &log);
	assert_eq!(next_event(&mut rx, "version").await, 5);
	tokio::time::sleep(std::time::Duration::from_millis(250)).await;
	host.fiber_of("p").unwrap().dispose().await;
	let lines = std::fs::read_to_string(&log).unwrap();
	let process: Vec<_> = lines
		.lines()
		.filter(|line| line.starts_with("start:") || line.starts_with("stop:"))
		.collect();
	assert_eq!(
		process,
		[
			"start:1", "start:1", "stop:1", "start:2", "stop:1", "start:3", "stop:2", "start:4",
			"stop:3", "start:5", "stop:4", "stop:5"
		]
	);
	assert_eq!(
		lines
			.lines()
			.filter(|line| line.starts_with("hello:"))
			.count(),
		6,
		"one hello per load: {lines}"
	);
	assert_eq!(
		lines
			.lines()
			.filter(|line| *line == "dependent-start")
			.count(),
		1,
		"{lines}"
	);
	assert_eq!(
		lines
			.lines()
			.filter(|line| *line == "dependent-stop")
			.count(),
		1,
		"{lines}"
	);
	assert!(lines.contains("dependent-stop\nstop:5"), "{lines}");
}

#[tokio::test(flavor = "multi_thread")]
async fn ordinary_lua_composition_can_load_a_process_wrapper() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"child.lua",
		&format!(
			"local descriptor = cartridge.process({}); return descriptor",
			json!(sdk_fixture())
		),
	);
	write(
		dir.path(),
		"parent.lua",
		r#"return {provide={"lua"},apply=function(ctx)
		ctx:provide("lua", 42)
		ctx:cartridge("child.lua", {composed=true, apply_call=true})
	end}"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="parent",path="parent.lua"}}"#,
	);
	let (host, mut rx) = boot(dir.path()).await;
	assert_eq!(next_event(&mut rx, "apply").await, 42);
	settle().await;
	assert_eq!(
		host.call("roundtrip", json!("config")).await.unwrap(),
		json!({"composed":true,"apply_call":true})
	);
	host.fiber_of("parent").unwrap().dispose().await;
	assert!(host.call("roundtrip", json!("config")).await.is_err());
}

async fn unready_fixture() -> (
	tempfile::TempDir,
	std::sync::Arc<crate::lua::Host>,
	std::path::PathBuf,
) {
	use std::os::unix::fs::PermissionsExt;
	let dir = tempfile::tempdir().unwrap();
	let child = dir.path().join("unready.sh");
	let pid = dir.path().join("pid");
	std::fs::write(&child, format!("#!/bin/sh\nif [ \"$1\" = hello ]; then echo '{{}}'; exit 0; fi\necho $$ > '{}'\nwhile read line; do :; done\n",pid.display())).unwrap();
	std::fs::set_permissions(&child, std::fs::Permissions::from_mode(0o755)).unwrap();
	write(
		dir.path(),
		"child.lua",
		&format!("return cartridge.process({})", serde_json::json!(child)),
	);
	write(
		dir.path(),
		"init.lua",
		"return {{id='child',path='child.lua'}}",
	);
	let (host, _) = boot(dir.path()).await;
	tokio::time::timeout(std::time::Duration::from_secs(1), async {
		while !pid.exists() {
			tokio::task::yield_now().await;
		}
	})
	.await
	.unwrap();
	(dir, host, pid)
}

async fn assert_reaped(pid: &std::path::Path) {
	let pid = std::fs::read_to_string(pid).unwrap();
	tokio::time::timeout(std::time::Duration::from_secs(1), async {
		loop {
			if !std::process::Command::new("/bin/kill")
				.args(["-0", pid.trim()])
				.stderr(std::process::Stdio::null())
				.status()
				.unwrap()
				.success()
			{
				break;
			}
			tokio::time::sleep(std::time::Duration::from_millis(10)).await;
		}
	})
	.await
	.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn disposing_a_child_before_ready_kills_it_without_waiting_for_startup_deadline() {
	let (_dir, host, pid) = unready_fixture().await;
	tokio::time::timeout(
		std::time::Duration::from_secs(1),
		host.fiber_of("child").unwrap().dispose(),
	)
	.await
	.unwrap();
	assert_reaped(&pid).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_child_that_stays_alive_without_ready_times_out_and_is_reaped() {
	let (_dir, host, pid) = unready_fixture().await;
	let fiber = host.fiber_of("child").unwrap();
	tokio::time::timeout(std::time::Duration::from_secs(7), fiber.settled())
		.await
		.unwrap();
	assert!(fiber.error().unwrap().contains("timed out before ready"));
	assert_reaped(&pid).await;
	fiber.dispose().await;
}
