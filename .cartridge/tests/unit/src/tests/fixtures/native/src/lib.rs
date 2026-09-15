use mlua::prelude::*;

fn twice(_: &Lua, n: i64) -> LuaResult<i64> {
	Ok(n * 2)
}

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
