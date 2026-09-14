use super::*;

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
