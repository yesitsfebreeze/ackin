//! The Lua interpreter every entry and configuration file runs in.

use mlua::{Lua, LuaOptions, LuaSerdeExt, StdLib, Table};

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
	install_globals(&lua)?;
	Ok(lua)
}

/// What `cartridge.process(command, {inject = {...}})` returns.
pub(crate) struct Process {
	pub(crate) command: Vec<String>,
	pub(crate) inject: Vec<String>,
}

impl mlua::UserData for Process {}

fn install_globals(lua: &Lua) -> mlua::Result<()> {
	let cartridge = lua.create_table()?;
	cartridge.set(
		"process",
		lua.create_function(|lua, (command, options): (mlua::Value, Option<Table>)| {
			let command = match command {
				mlua::Value::String(s) => {
					s.to_str()?.split_whitespace().map(str::to_owned).collect()
				}
				value => lua.from_value::<Vec<String>>(value)?,
			};
			if command.first().is_none_or(String::is_empty) {
				return Err(mlua::Error::RuntimeError(
					"cartridge.process needs a nonempty command".into(),
				));
			}
			let inject = match options {
				Some(options) => match options.get::<mlua::Value>("inject")? {
					mlua::Value::Nil => Vec::new(),
					value => lua.from_value::<Vec<String>>(value)?,
				},
				None => Vec::new(),
			};
			Ok(Process { command, inject })
		})?,
	)?;
	cartridge.set(
		"trace",
		lua.create_function(|_, ()| Ok(transport::cartridge::trace()))?,
	)?;
	lua.globals().set("cartridge", cartridge)
}

#[cfg(test)]
#[path = "../.cartridge/tests/unit/src/lua/tests.rs"]
mod tests;
