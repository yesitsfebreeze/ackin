//! The Lua interpreter every entry and configuration file runs in.

use mlua::{Lua, LuaOptions, StdLib, Table};

/// The `os` functions a cartridge keeps: the clock and the environment.
const OS_KEPT: &[&str] = &["clock", "date", "difftime", "getenv", "time"];

/// Base-library functions that read files.
const BASE_DROPPED: &[&str] = &["dofile", "loadfile"];

/// The safe standard library without `io` and `package`, `os` trimmed to
/// [`OS_KEPT`], and no file-reading base functions.
pub fn interpreter() -> mlua::Result<Lua> {
	let lua = Lua::new_with(
		StdLib::ALL_SAFE ^ StdLib::IO ^ StdLib::PACKAGE,
		LuaOptions::default(),
	)?;
	let globals = lua.globals();
	let os: Table = globals.get("os")?;
	let kept = lua.create_table()?;
	for name in OS_KEPT {
		kept.set(*name, os.get::<mlua::Value>(*name)?)?;
	}
	globals.set("os", kept)?;
	for name in BASE_DROPPED {
		globals.set(*name, mlua::Value::Nil)?;
	}
	Ok(lua)
}

#[cfg(test)]
#[path = "../.cartridge/tests/unit/src/lua/tests.rs"]
mod tests;
