use mlua::prelude::*;

/// 2, or 3 when built with `--features rebuilt`. The two builds of this fixture
/// differ in what they answer, so a reload can be observed to load the file
/// that is on disk now rather than the image it already had.
const FACTOR: i64 = if cfg!(feature = "rebuilt") { 3 } else { 2 };

fn twice(_: &Lua, n: i64) -> LuaResult<i64> {
	Ok(n * FACTOR)
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
