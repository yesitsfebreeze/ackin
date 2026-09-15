use std::path::Path;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

use mlua::thread::ThreadTriggers;
use mlua::{HookTriggers, Lua, LuaOptions, LuaSerdeExt, StdLib, Table, VmState};

const OS_KEPT: &[&str] = &["clock", "date", "difftime", "getenv", "time"];

const BASE_DROPPED: &[&str] = &["dofile", "loadfile"];

const EVERY: u32 = 10_000;

const RUNAWAY: u64 = 1024;

const RUNAWAY_EXIT: i32 = 70;

#[derive(Clone, Copy, PartialEq)]
pub enum Overrun {
	Refuse,
	Exit,
}

pub fn interpreter(memory_bytes: usize, instructions: u64, overrun: Overrun) -> mlua::Result<Lua> {
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
	if memory_bytes > 0 {
		lua.set_memory_limit(memory_bytes)?;
	}
	if instructions > 0 {
		budget(&lua, instructions as i64, overrun)?;
	}
	Ok(lua)
}

/// mlua refills the count on every resume it drives, not on a plain `call` or
/// a `coroutine.resume` inside Lua. The hook must never yield: the resume
/// after it would refill.
fn budget(lua: &Lua, instructions: i64, overrun: Overrun) -> mlua::Result<()> {
	let left = Arc::new(AtomicI64::new(instructions));
	let refusals = Arc::new(AtomicU64::new(0));
	let (spent, refused) = (left.clone(), refusals.clone());
	// Set before `install` (node.rs) makes pooled threads: mlua copies the
	// global hook into every thread and coroutine it creates from here on.
	lua.set_global_hook(
		HookTriggers::new().every_nth_instruction(EVERY),
		move |_, _| {
			if spent.fetch_sub(EVERY as i64, Ordering::Relaxed) > EVERY as i64 {
				return Ok(VmState::Continue);
			}
			if overrun == Overrun::Exit && refused.fetch_add(1, Ordering::Relaxed) >= RUNAWAY {
				tracing::error!(target: "cartridge", "ran past its budget of {instructions} Lua instructions and kept going");
				std::process::exit(RUNAWAY_EXIT);
			}
			Err(mlua::Error::RuntimeError(format!(
				"exceeded its budget of {instructions} Lua instructions"
			)))
		},
	)?;
	lua.set_thread_event_callback(ThreadTriggers::ON_RESUME, move |_, _| {
		left.store(instructions, Ordering::Relaxed);
		refusals.store(0, Ordering::Relaxed);
		Ok(())
	});
	Ok(())
}

/// Declared, not settled: settling reads configuration files through this
/// same evaluator, so using settled limits here would be circular.
fn declared() -> crate::settings::Host {
	serde_json::from_value(crate::settings::defaults(crate::settings::host_specs()))
		.expect("the host's declared defaults match their type")
}

/// Deserialized straight from Lua: through JSON an empty table is a map, and
/// `return {}` is a valid empty descriptor.
pub fn evaluate<T: serde::de::DeserializeOwned>(path: &Path) -> crate::error::Result<T> {
	let source = crate::trust::read(path)?;
	let limits = declared();
	let lua = interpreter(
		limits.lua_memory_bytes,
		limits.lua_instruction_budget,
		Overrun::Refuse,
	)?;
	let value: mlua::Value = lua.load(&source).set_name(path.to_string_lossy()).eval()?;
	Ok(lua.from_value(value)?)
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/lua/tests.rs"]
mod tests;
