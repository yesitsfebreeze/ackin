use super::*;

fn lua_bool(lua: &Lua, expr: &str) -> bool {
	lua.load(format!("return {expr}")).eval().unwrap()
}

#[test]
fn the_interpreter_keeps_computation_and_drops_the_machine() {
	let lua = interpreter(0, 0, Overrun::Refuse).unwrap();
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
	let lua = interpreter(1 << 20, 0, Overrun::Refuse).unwrap();
	let error = lua
		.load("local t = {} for i = 1, 10000000 do t[i] = i end")
		.exec()
		.unwrap_err();
	assert!(error.to_string().contains("not enough memory"), "{error}");
	// The interpreter is still usable after the refusal.
	assert!(lua_bool(&lua, "1 + 1 == 2"));
}

/// One file per state, and the gate still in front: what one file leaves
/// behind never reaches the next, and an unrecorded file never runs.
#[test]
fn each_file_is_evaluated_in_a_state_of_its_own() {
	crate::tests::home();
	let dir = tempfile::tempdir().unwrap();
	std::fs::write(
		dir.path().join("a.lua"),
		r#"leak = 7 string.format = function() return "poisoned" end return {ok = true}"#,
	)
	.unwrap();
	let refused = evaluate::<serde_json::Value>(&dir.path().join("a.lua"));
	assert!(refused.is_err(), "an unrecorded file must be refused");
	crate::trust::record(dir.path()).unwrap();
	let a: serde_json::Value = evaluate(&dir.path().join("a.lua")).unwrap();
	assert_eq!(a["ok"], true);
	std::fs::write(
		dir.path().join("b.lua"),
		r#"return {seen = tostring(leak) .. "/" .. string.format("%d", 1)}"#,
	)
	.unwrap();
	crate::trust::record(dir.path()).unwrap();
	let b: serde_json::Value = evaluate(&dir.path().join("b.lua")).unwrap();
	assert_eq!(
		b["seen"], "nil/1",
		"a.lua's globals must not leak into b.lua"
	);
	// Straight from Lua: through JSON an empty table would be a map, and
	// `return {}` is a valid empty profile.
	std::fs::write(dir.path().join("empty.lua"), "return {}").unwrap();
	crate::trust::record(dir.path()).unwrap();
	let empty: Vec<String> = evaluate(&dir.path().join("empty.lua")).unwrap();
	assert!(empty.is_empty());
}

#[tokio::test]
async fn a_runaway_is_refused_and_the_state_survives() {
	let lua = interpreter(1 << 30, 1_000_000, Overrun::Refuse).unwrap();
	let error = lua
		.load("while true do end")
		.exec_async()
		.await
		.unwrap_err();
	assert!(error.to_string().contains("Lua instructions"), "{error}");
	assert!(lua
		.load("return 1 + 1 == 2")
		.eval_async::<bool>()
		.await
		.unwrap());
}

#[tokio::test]
async fn every_resume_gets_the_budget_back() {
	let lua = interpreter(1 << 30, 1_000_000, Overrun::Refuse).unwrap();
	let work: mlua::Function = lua
		.load("return function() for i = 1, 600000 do end end")
		.eval()
		.unwrap();
	for call in 1..=3 {
		work.call_async::<()>(())
			.await
			.unwrap_or_else(|e| panic!("call {call} refilled its budget: {e}"));
	}
	let spin: mlua::Function = lua
		.load("return function() while true do end end")
		.eval()
		.unwrap();
	for call in 1..=3 {
		assert!(
			spin.call_async::<()>(()).await.is_err(),
			"spinning call {call} must be refused, not hang"
		);
	}
}

#[tokio::test]
async fn a_wait_refills_the_budget_and_a_coroutine_does_not() {
	let lua = interpreter(1 << 30, 1_000_000, Overrun::Refuse).unwrap();
	let pause = lua
		.create_async_function(|_, ()| async {
			tokio::task::yield_now().await;
			Ok(())
		})
		.unwrap();
	lua.globals().set("pause", pause).unwrap();
	lua.load("for i = 1, 600000 do end pause() for i = 1, 600000 do end")
		.exec_async()
		.await
		.expect("a wait hands control back, so each half gets the budget");
	assert!(
		lua.load("for i = 1, 600000 do end for i = 1, 600000 do end")
			.exec_async()
			.await
			.is_err(),
		"no wait in between spends one budget"
	);
	assert!(
		lua.load("coroutine.wrap(function() while true do end end)()")
			.exec_async()
			.await
			.is_err(),
		"a Lua-side resume does not refill"
	);
}
