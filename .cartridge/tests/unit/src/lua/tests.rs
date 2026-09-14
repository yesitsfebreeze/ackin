use super::*;
use crate::runtime::Runtime;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

fn write(dir: &Path, name: &str, body: &str) {
	std::fs::write(dir.join(name), body).unwrap();
}

/// A host over `dir`, reconciled once, with its outbox subscribed.
async fn boot(dir: &Path) -> (Arc<Host>, broadcast::Receiver<Value>) {
	let host = Host::new(Runtime::new(), dir, dir);
	let rx = host.outbox();
	host.reconcile().await.unwrap();
	(host, rx)
}

async fn next_event(rx: &mut broadcast::Receiver<Value>, name: &str) -> Value {
	loop {
		let m = tokio::time::timeout(Duration::from_secs(5), rx.recv())
			.await
			.unwrap_or_else(|_| panic!("no `{name}` event within 5s"))
			.unwrap();
		if m["event"] == name {
			return m["data"].clone();
		}
		if m.get("error").is_some() {
			panic!("{m}");
		}
	}
}

fn lua_bool(lua: &Lua, expr: &str) -> bool {
	lua.load(format!("return {expr}")).eval().unwrap()
}

#[test]
fn the_interpreter_keeps_computation_and_drops_the_machine() {
	let lua = interpreter().unwrap();
	for gone in [
		"io",
		"package",
		"require",
		"dofile",
		"loadfile",
		"debug",
		"os.execute",
		"os.exit",
		"os.remove",
		"os.rename",
		"os.tmpname",
		"os.setlocale",
	] {
		assert!(
			lua_bool(&lua, &format!("{gone} == nil")),
			"{gone} is reachable"
		);
	}
	for kept in [
		"string.format",
		"table.concat",
		"math.floor",
		"coroutine.yield",
		"os.getenv",
		"os.time",
		"os.clock",
		"os.date",
		"os.difftime",
		"pcall",
		"load",
	] {
		assert!(
			lua_bool(&lua, &format!("type({kept}) == 'function'")),
			"{kept} is missing"
		);
	}
	std::env::set_var("CARTRIDGE_INTERPRETER_TEST", "seen");
	assert!(lua_bool(
		&lua,
		"os.getenv('CARTRIDGE_INTERPRETER_TEST') == 'seen'"
	));
	assert!(lua_bool(&lua, "type(os.time()) == 'number'"));
}

#[test]
fn a_memory_limit_bounds_what_a_chunk_can_allocate() {
	let lua = interpreter().unwrap();
	lua.set_memory_limit(1 << 20).unwrap();
	let error = lua
		.load("local t = {} for i = 1, 10000000 do t[i] = i end")
		.exec()
		.unwrap_err();
	assert!(error.to_string().contains("not enough memory"), "{error}");
	// The interpreter is still usable after the refusal.
	assert!(lua_bool(&lua, "1 + 1 == 2"));
}

/// The `bail` a cartridge makes from inside `apply` parks its coroutine on
/// the runtime. On a current-thread runtime the old bridge panicked here
/// (`block_in_place` has no thread to block); an async host call has nothing
/// to block and answers.
#[tokio::test]
async fn a_host_call_parks_the_coroutine_on_a_current_thread_runtime() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"a.lua",
		r#"return { provide = {"a"}, apply = function(ctx)
			ctx:on("ask", function(n) return n * 2 end)
			ctx:provide("a", true)
		end }"#,
	);
	write(
		dir.path(),
		"b.lua",
		r#"return { inject = {"a"}, apply = function(ctx)
			local answer = ctx:bail("ask", 21)
			local rows = ctx:gather("ask", 1)
			ctx:parallel("ask", 0)
			ctx:send("answer", { bail = answer, gathered = rows[1].data, from = rows[1].from })
		end }"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return { { id = "a", path = "a.lua" }, { id = "b", path = "b.lua" } }"#,
	);
	let (_host, mut rx) = boot(dir.path()).await;
	assert_eq!(
		next_event(&mut rx, "answer").await,
		json!({ "bail": 42, "gathered": 2, "from": "a" })
	);
}

/// A listener is its own coroutine: one that waits on the host from inside
/// its answer — a `bail` into another cartridge — parks and resumes like the
/// apply that registered it, and its answer still reaches the asker.
#[tokio::test]
async fn a_listener_that_waits_on_the_host_still_answers() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"inner.lua",
		r#"return { provide = {"inner"}, apply = function(ctx)
			ctx:on("inner", function(n) return n + 1 end)
			ctx:provide("inner", true)
		end }"#,
	);
	write(
		dir.path(),
		"outer.lua",
		r#"return { inject = {"inner"}, provide = {"outer"}, apply = function(ctx)
			ctx:on("outer", function(n) return ctx:bail("inner", n) * 10 end)
			ctx:provide("outer", function(n) return ctx:bail("outer", n) end)
		end }"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return { { id = "inner", path = "inner.lua" }, { id = "outer", path = "outer.lua" } }"#,
	);
	let (host, _rx) = boot(dir.path()).await;
	for _ in 0..50 {
		if host.runtime().ctx().peek("outer").is_some() {
			break;
		}
		tokio::time::sleep(Duration::from_millis(20)).await;
	}
	assert_eq!(host.call("outer", json!(4)).await.unwrap(), json!(50));
}

/// A Lua error inside a service is the caller's typed error, with the Lua
/// message intact; a key nothing provides is its own kind.
#[tokio::test]
async fn service_failures_are_typed() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"p.lua",
		r#"return { provide = {"boom"}, apply = function(ctx)
			ctx:provide("boom", function() error("kaboom") end)
		end }"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return { { id = "p", path = "p.lua" } }"#,
	);
	let (host, _rx) = boot(dir.path()).await;
	for _ in 0..50 {
		if host.runtime().ctx().peek("boom").is_some() {
			break;
		}
		tokio::time::sleep(Duration::from_millis(20)).await;
	}
	let error = host.call("boom", Value::Null).await.unwrap_err();
	assert!(matches!(error, Error::Lua(_)), "{error:?}");
	assert!(error.to_string().contains("kaboom"), "{error}");
	let error = host.call("absent", Value::Null).await.unwrap_err();
	assert!(
		matches!(&error, Error::NotProvided(key) if key == "absent"),
		"{error:?}"
	);
	assert_eq!(error.to_string(), "`absent` is not provided");
}
