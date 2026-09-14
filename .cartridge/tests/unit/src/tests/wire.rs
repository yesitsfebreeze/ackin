//! Recursion over the process wire: a cartridge that hosts cartridges over the
//! same wire it is itself hosted on. The daemon above a sub-host never learns
//! that it nested — every frame it sees still names only the sub-host.
use super::{boot, next_event, settle, write};
use serde_json::json;

/// A cartridge that hosts `rpc_fixture` as its own child.
fn nested_fixture() -> std::path::PathBuf {
	static BINARY: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	BINARY
		.get_or_init(|| super::built(&["-p", "cartridge", "--example", "nested_fixture"]))
		.clone()
}

/// An executable shell child speaking the wire by hand, for behaviors the
/// compiled fixtures do not carry: a `hello` that declares nothing, a reload
/// answer of its own, a bridge call, a child that never becomes ready.
fn script(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
	let path = dir.join(name);
	std::fs::write(&path, body).unwrap();
	use std::os::unix::fs::PermissionsExt;
	std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
	path
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cartridge_hosts_a_cartridge_over_the_same_wire() {
	let parent = nested_fixture();
	let child = super::process::sdk_fixture();
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
		"parent".into(),
		vec![parent.to_string_lossy().into_owned()],
		json!({ "child": [child.to_string_lossy()], "lifecycle": true }),
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
	// Three wire hops on one protocol: the daemon calls the parent, the parent
	// forwards to its child, the child calls the injected key back up through
	// the parent. The answer is the child's, assembled from keys the daemon
	// injected into the parent.
	assert_eq!(
		call(json!(21)).await.unwrap().unwrap(),
		json!({"data": {"value": 21}, "meta": {"description": "Lua service"}, "absent": null})
	);
	// An event the child listens for arrives through the parent, and the
	// child's `send` leaves through the parent to the socket audience.
	host.emit("probe", json!(1));
	assert_eq!(next_event(&mut rx, "observed").await, json!(1));
	// Disposing the sub-host disposes the child: the child's own finalizer
	// runs, and its farewell still reaches the socket through the parent.
	fiber.dispose().await;
	assert_eq!(next_event(&mut rx, "disposed").await, json!(true));
	assert!(call(json!(21)).await.unwrap().is_err());
	settle().await;
}
/// The shared `lua` composition every recursion test here intercepts.
fn lua_composition(dir: &std::path::Path) {
	write(
		dir,
		"lua.lua",
		r#"return { provide = {"lua"}, apply = function(ctx)
		ctx:provide("lua", function(args)
			if args == "error" then error("lua service failed") end
			return { value = args }
		end)
	end }"#,
	);
	write(
		dir,
		"init.lua",
		r#"return {{ id = "lua", path = "lua.lua" }}"#,
	);
}

/// Apply the parent fixture over the shared `lua` composition and return its
/// fiber, the way every recursion test here drives the nest.
async fn nest(
	host: &std::sync::Arc<crate::lua::Host>,
	parent: &std::path::Path,
	child: &std::path::Path,
	meta: serde_json::Value,
) -> crate::runtime::FiberHandle {
	let component = crate::cartridge::component(
		host.clone(),
		"parent".into(),
		vec![parent.to_string_lossy().into_owned()],
		json!({ "child": [child.to_string_lossy()], "lifecycle": false }),
	)
	.unwrap();
	let fiber = host
		.runtime()
		.ctx()
		.intercept("lua", meta)
		.cartridge(component);
	fiber.settled().await;
	fiber
}

#[tokio::test(flavor = "multi_thread")]
async fn a_child_key_the_sub_host_never_declared_stays_private() {
	let parent = nested_fixture();
	let dir = tempfile::tempdir().unwrap();
	lua_composition(dir.path());
	// A child that fronts two keys, only one of which its sub-host declares.
	let child = script(
		dir.path(),
		"child.sh",
		r#"#!/bin/sh
if [ "$1" = hello ]; then echo '{"provide":["roundtrip","secret"]}'; exit 0; fi
echo '{"provide":"roundtrip"}'
echo '{"provide":"secret"}'
echo '{"ready":true}'
while read line; do
	id=$(printf '%s' "$line" | sed 's/.*"id":\([0-9]*\).*/\1/')
	case "$line" in
		*'"call"'*) key=$(printf '%s' "$line" | sed 's/.*"call":"\([^"]*\)".*/\1/'); echo "{\"reply\":$id,\"data\":{\"served\":\"$key\"}}";;
		*'"dispose"'*) exit 0;;
	esac
done
"#,
	);
	let (host, _rx) = boot(dir.path()).await;
	settle().await;
	let fiber = nest(
		&host,
		&parent,
		&child,
		json!({"description": "Lua service"}),
	)
	.await;
	async fn call(
		host: &std::sync::Arc<crate::lua::Host>,
		key: &str,
	) -> Result<serde_json::Value, String> {
		tokio::time::timeout(
			std::time::Duration::from_secs(5),
			host.call(key, json!(null)),
		)
		.await
		.unwrap()
		.map_err(String::from)
	}
	// The declared key is fronted: the daemon's call crosses the nest and the
	// child serves it.
	assert_eq!(
		call(&host, "roundtrip").await.unwrap(),
		json!({"served": "roundtrip"})
	);
	// The undeclared key never reached the store above: the daemon does not
	// know it, and a call of it errs there, above the sub-host.
	let error = call(&host, "secret").await.unwrap_err();
	assert_eq!(error, "`secret` is not provided", "{error}");
	fiber.dispose().await;
	settle().await;
}

/// A wrapper entry that hosts the parent fixture, so the entry is a slot the
/// daemon's reload machinery and profile grant can reach.
fn wrapper(dir: &std::path::Path, parent: &std::path::Path, child: &std::path::Path, extra: &str) {
	lua_composition(dir);
	write(
		dir,
		"parent.lua",
		&format!(
			"return cartridge.process({{{}}})",
			json!(parent.to_string_lossy())
		),
	);
	write(
		dir,
		"init.lua",
		&format!(
			r#"return {{{{ id = "lua", path = "lua.lua" }}, {{ id = "parent", path = "parent.lua", config = {{ child = {{ {} }}, {extra} }} }}}}"#,
			json!(child.to_string_lossy())
		),
	);
}

/// The roundtrip answer's child pid, polled until it differs from `first`.
async fn reloaded(
	host: &std::sync::Arc<crate::lua::Host>,
	first: u64,
) -> Result<u64, &'static str> {
	let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
	while tokio::time::Instant::now() < deadline {
		settle().await;
		let served = tokio::time::timeout(
			std::time::Duration::from_secs(5),
			host.call("roundtrip", json!(null)),
		)
		.await;
		if let Ok(Ok(answer)) = served {
			if answer["pid"].as_u64().is_some_and(|pid| pid != first) {
				return Ok(answer["pid"].as_u64().unwrap());
			}
		}
	}
	Err("reload never switched the served pid")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_reload_carries_down_to_the_child_that_declared_it() {
	let parent = nested_fixture();
	// One child whose `hello` declares reload; the test steers its answer
	// through a file beside the script.
	let dir = tempfile::tempdir().unwrap();
	let refuse = dir.path().join("refuse");
	let child = script(
		dir.path(),
		"child.sh",
		&format!(
			r#"#!/bin/sh
if [ "$1" = hello ]; then echo '{{"provide":["roundtrip"],"reload":true}}'; exit 0; fi
echo '{{"provide":"roundtrip"}}'
echo '{{"ready":true}}'
while read line; do
	id=$(printf '%s' "$line" | sed 's/.*"id":\([0-9]*\).*/\1/')
	case "$line" in
		*'"reload"'*)
			if [ -f "{}" ]; then
				echo "{{\"reply\":$id,\"error\":\"child refused reload\"}}"
			else
				echo "{{\"reply\":$id,\"data\":\"child reloaded\"}}"
			fi;;
		*'"call"'*) echo "{{\"reply\":$id,\"data\":{{\"pid\":$$}}}}";;
		*'"dispose"'*) exit 0;;
	esac
done
"#,
			refuse.display(),
		),
	);
	wrapper(dir.path(), &parent, &child, "");
	let (host, mut rx) = boot(dir.path()).await;
	settle().await;
	host.fiber_of("parent").unwrap().settled().await;
	let first = tokio::time::timeout(
		std::time::Duration::from_secs(5),
		host.call("roundtrip", json!(null)),
	)
	.await
	.unwrap()
	.unwrap()["pid"]
		.as_u64()
		.unwrap();
	// The child answers the carried-down reload frame, the sub-host answers
	// above with the child's reply, and the generation switches.
	host.request_reload(host.fiber_of("parent").unwrap().uid());
	let second = reloaded(&host, first).await.unwrap();
	// With the child refusing, the child's own error text is what the daemon
	// reads, and the transaction it fails keeps the live generation.
	std::fs::write(&refuse, "refuse").unwrap();
	host.request_reload(host.fiber_of("parent").unwrap().uid());
	let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
	let reported = loop {
		let m = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
			.await
			.unwrap_or_else(|_| panic!("the child's refusal was never reported"))
			.unwrap();
		if m.get("error").is_some() {
			break m;
		}
		if tokio::time::Instant::now() > deadline {
			panic!("the child's refusal was never reported: {m}");
		}
	};
	assert!(
		reported["error"]["message"]
			.as_str()
			.unwrap()
			.contains("child refused reload"),
		"{reported}"
	);
	assert_eq!(
		tokio::time::timeout(
			std::time::Duration::from_secs(5),
			host.call("roundtrip", json!(null))
		)
		.await
		.unwrap()
		.unwrap()["pid"]
			.as_u64(),
		Some(second)
	);
	settle().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_child_that_declared_no_reload_still_answers_null() {
	let parent = nested_fixture();
	let dir = tempfile::tempdir().unwrap();
	// A child whose `hello` declares no reload: the sub-host answers the
	// daemon's request itself, with null, and the transaction proceeds.
	let child = script(
		dir.path(),
		"child.sh",
		r#"#!/bin/sh
if [ "$1" = hello ]; then echo '{"provide":["roundtrip"]}'; exit 0; fi
echo '{"provide":"roundtrip"}'
echo '{"ready":true}'
while read line; do
	id=$(printf '%s' "$line" | sed 's/.*"id":\([0-9]*\).*/\1/')
	case "$line" in
		*'"call"'*) echo "{\"reply\":$id,\"data\":{\"pid\":$$}}";;
		*'"dispose"'*) exit 0;;
	esac
done
"#,
	);
	wrapper(dir.path(), &parent, &child, "");
	let (host, mut rx) = boot(dir.path()).await;
	settle().await;
	host.fiber_of("parent").unwrap().settled().await;
	let first = tokio::time::timeout(
		std::time::Duration::from_secs(5),
		host.call("roundtrip", json!(null)),
	)
	.await
	.unwrap()
	.unwrap()["pid"]
		.as_u64()
		.unwrap();
	host.request_reload(host.fiber_of("parent").unwrap().uid());
	reloaded(&host, first).await.unwrap();
	// No error frame arrived: the null answer let the reload finish clean.
	for _ in 0..3 {
		let m = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
			.await
			.ok()
			.and_then(Result::ok);
		if let Some(m) = m {
			assert!(m.get("error").is_none(), "{m}");
		}
	}
	settle().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_nested_bridge_call_resolves_against_the_daemons_profile_grant() {
	let parent = nested_fixture();
	for granted in [false, true] {
		let dir = tempfile::tempdir().unwrap();
		// The folder cartridge the child's bridge call names: a client module
		// and one service of its own.
		std::fs::create_dir(dir.path().join("p")).unwrap();
		write(
			dir.path(),
			"p/cartridge.json",
			r#"{"name":"p","entry":"init.lua","ui":"ui.tsx"}"#,
		);
		write(dir.path(), "p/ui.tsx", "export function setup() {}");
		write(
			dir.path(),
			"p/init.lua",
			r#"return {provide={"counter"},apply=function(ctx) ctx:provide("counter",function(args) return args end) end}"#,
		);
		// The child asks for the bridge status, calls `counter` with what it
		// read, and reports the answer it got through the parent.
		let child = script(
			dir.path(),
			"child.sh",
			r#"#!/bin/sh
if [ "$1" = hello ]; then echo '{"provide":["roundtrip"]}'; exit 0; fi
echo '{"ready":true}'
asked=""
echo '{"bridge":"status","id":1}'
while read line; do
	case "$line" in
		*'"reply"'*)
			if [ -z "$asked" ]; then
				gen=$(printf '%s' "$line" | sed 's/.*"generation":\([0-9]*\).*/\1/')
				if [ -z "$gen" ]; then
					sleep 0.2
					echo '{"bridge":"status","id":1}'
				else
					asked=1
					owner=$(printf '%s' "$line" | sed 's/.*"id":"\([^"]*\)".*/\1/')
					echo "{\"bridge\":\"call\",\"owner\":\"$owner\",\"generation\":$gen,\"key\":\"counter\",\"args\":3,\"id\":2}"
				fi
			else
				echo "{\"send\":\"bridged\",\"data\":$line}"
			fi;;
		*'"call"'*) id=$(printf '%s' "$line" | sed 's/.*"id":\([0-9]*\).*/\1/'); echo "{\"reply\":$id,\"data\":null}";;
		*'"dispose"'*) exit 0;;
	esac
done
"#,
		);
		wrapper(
			dir.path(),
			&parent,
			&child,
			&format!("bridge = {}", granted),
		);
		// The folder cartridge is ledger-derived and starts only when named:
		// the profile enables it so the bridge has an owner to resolve.
		write(
			dir.path(),
			"init.lua",
			&format!(
				r#"return {{{{ id = "lua", path = "lua.lua" }}, {{ id = "p", path = "p" }}, {{ id = "parent", path = "parent.lua", config = {{ child = {{ {} }}, bridge = {} }} }}}}"#,
				json!(child.to_string_lossy()),
				granted
			),
		);
		let (host, mut rx) = boot(dir.path()).await;
		settle().await;
		host.fiber_of("parent").unwrap().settled().await;
		// forwarded frame resolves against the daemon's profile exactly as a
		// first-generation cartridge's would; without it the daemon refuses it
		// in its own words.
		let bridged = next_event(&mut rx, "bridged").await;
		if granted {
			assert_eq!(bridged["data"], json!(3), "{bridged}");
			assert!(bridged.get("error").is_none(), "{bridged}");
		} else {
			assert_eq!(
				bridged["error"],
				json!("profile has not granted bridge access"),
				"{bridged}"
			);
		}
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn a_child_that_never_becomes_ready_fails_the_spawn_where_it_failed() {
	let parent = nested_fixture();
	for (body, nested_expected, direct_expected) in [
		(
			r#"#!/bin/sh
if [ "$1" = hello ]; then echo '{"provide":["roundtrip"]}'; exit 0; fi
echo '{"not":"ready"}'
exit 7
"#,
			"nested exited before ready: exit status: 7",
			"exited before ready: exit status: 7",
		),
		(
			r#"#!/bin/sh
if [ "$1" = hello ]; then echo '{"provide":["roundtrip"]}'; exit 0; fi
echo 'garbage'
"#,
			"nested sent an unreadable line: garbage",
			"expected value at line 1 column 1",
		),
		(
			r#"#!/bin/sh
if [ "$1" = hello ]; then echo '{"provide":["roundtrip"]}'; exit 0; fi
echo '{"not":"ready"}'
exec 1>&-
sleep 0.05
exit 7
"#,
			"nested exited before ready: exit status: 7",
			"exited before ready: exit status: 7",
		),
	] {
		for nested in [false, true] {
			let dir = tempfile::tempdir().unwrap();
			lua_composition(dir.path());
			let direct_body = body.replace("{\"not\":\"ready\"}", "{\"provide\":\"roundtrip\"}");
			let child = script(
				dir.path(),
				"child.sh",
				if nested { body } else { &direct_body },
			);
			let (host, _rx) = boot(dir.path()).await;
			settle().await;
			let fiber = if nested {
				nest(
					&host,
					&parent,
					&child,
					json!({"description": "Lua service"}),
				)
				.await
			} else {
				let component = crate::cartridge::component(
					host.clone(),
					"direct".into(),
					vec![child.to_string_lossy().into_owned()],
					json!({}),
				)
				.unwrap();
				let fiber = host.runtime().ctx().cartridge(component);
				fiber.settled().await;
				fiber
			};
			let error = fiber.error().expect("the spawn never failed");
			let expected = if nested {
				nested_expected
			} else {
				direct_expected
			};
			assert!(error.contains(expected), "nested={nested}: {error}");
		}
	}
}

async fn startup_child_reaped(pid: &std::path::Path) {
	let pid = std::fs::read_to_string(pid).unwrap();
	tokio::time::timeout(std::time::Duration::from_secs(2), async {
		while std::process::Command::new("/bin/kill")
			.args(["-0", pid.trim()])
			.stderr(std::process::Stdio::null())
			.status()
			.unwrap()
			.success()
		{
			tokio::time::sleep(std::time::Duration::from_millis(10)).await;
		}
	})
	.await
	.expect("owned startup child was not reaped");
}

#[tokio::test(flavor = "multi_thread")]
async fn stdout_closed_live_children_obey_each_host_deadline_and_are_reaped() {
	use std::process::Stdio;
	use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
	let parent = nested_fixture();
	// The startup deadline is a host setting. The child must outlive it, or it
	// exits on its own and nothing is left to reap; the wait runs a little past
	// it, so the deadline is what ends the child, whatever it is set to.
	let startup = crate::settings::host().startup_timeout();
	let deadline = startup + std::time::Duration::from_secs(2);
	for nested in [false, true] {
		let dir = tempfile::tempdir().unwrap();
		let pid = dir.path().join("owned.pid");
		let child = script(
			dir.path(),
			"live.sh",
			&format!(
				r#"#!/bin/sh
if [ "$1" = hello ]; then echo '{{"provide":["roundtrip"]}}'; exit 0; fi
echo $$ > '{}'
exec 1>&-
exec sleep {}
"#,
				pid.display(),
				startup.as_secs() * 2
			),
		);
		if nested {
			// Isolate the SDK's deadline: an outer runtime deadline starts earlier
			// and would obscure whether the nested SDK owns and reaps its child.
			let mut process = tokio::process::Command::new(&parent)
				.stdin(Stdio::piped())
				.stdout(Stdio::piped())
				.stderr(Stdio::piped())
				.kill_on_drop(true)
				.spawn()
				.unwrap();
			let mut stdin = process.stdin.take().unwrap();
			stdin
				.write_all(
					format!("{}\n", json!({"apply":{"config":{"child":[child]}}})).as_bytes(),
				)
				.await
				.unwrap();
			let mut lines = BufReader::new(process.stdout.take().unwrap()).lines();
			let error = tokio::time::timeout(deadline, async {
				loop {
					let line = lines
						.next_line()
						.await
						.unwrap()
						.expect("SDK stdout ended before startup error");
					let value: serde_json::Value = serde_json::from_str(&line).unwrap();
					if let Some(error) = value["error"].as_str() {
						break error.to_owned();
					}
				}
			})
			.await
			.expect("SDK startup exceeded its deadline");
			assert!(error.contains("nested timed out before ready"), "{error}");
			startup_child_reaped(&pid).await;
			process.kill().await.unwrap();
			process.wait().await.unwrap();
		} else {
			lua_composition(dir.path());
			let (host, _) = boot(dir.path()).await;
			let component = crate::cartridge::component(
				host.clone(),
				"direct".into(),
				vec![child.to_string_lossy().into_owned()],
				json!({}),
			)
			.unwrap();
			let fiber = host.runtime().ctx().cartridge(component);
			tokio::time::timeout(deadline, fiber.settled())
				.await
				.expect("runtime startup exceeded its deadline");
			let error = fiber.error().expect("live child became ready");
			assert!(error.contains("direct timed out before ready"), "{error}");
			startup_child_reaped(&pid).await;
		}
	}
}
