//! A Rust cartridge as a native Lua module: it reaches the base through the
//! injected `cartridge` global and imports nothing.

use mlua::prelude::*;

fn twice(_: &Lua, n: i64) -> LuaResult<i64> {
	Ok(n * 2)
}

/// Ask the composition through the global, as a cartridge's own code would.
fn ask(lua: &Lua, (event, data): (String, LuaValue)) -> LuaResult<LuaValue> {
	let cartridge: LuaTable = lua.globals().get("cartridge")?;
	cartridge.call_function("bail", (event, data))
}

#[mlua::lua_module]
fn native_fixture(lua: &Lua) -> LuaResult<LuaTable> {
	let module = lua.create_table()?;
	module.set("twice", lua.create_function(twice)?)?;
	module.set("ask", lua.create_function(ask)?)?;
	Ok(module)
}
