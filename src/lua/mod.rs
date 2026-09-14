//! The Lua interpreter every entry and configuration file runs in: one state
//! each, with its own memory cap and instruction budget.

use std::path::Path;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

use mlua::thread::ThreadTriggers;
use mlua::{HookTriggers, Lua, LuaOptions, LuaSerdeExt, StdLib, Table, VmState};

/// The `os` functions a cartridge keeps: the clock and the environment.
const OS_KEPT: &[&str] = &["clock", "date", "difftime", "getenv", "time"];

/// Base-library functions that read files.
const BASE_DROPPED: &[&str] = &["dofile", "loadfile"];

/// How often the budget is checked. Any count hook costs the same ~1.25x on
/// Lua-heavy code, so this is as coarse as the refusal latency allows.
const EVERY: u32 = 10_000;

/// Caught refusals before a node counts as a runaway. Each costs Lua `EVERY`
/// more instructions, so only code ignoring the answer gets here.
const RUNAWAY: u64 = 1024;

/// The status a node that will not stop leaves with.
const RUNAWAY_EXIT: i32 = 70;

/// What happens when Lua catches its own refusal and keeps running.
#[derive(Clone, Copy, PartialEq)]
pub enum Overrun {
	/// Keep refusing. The base's own path: a configuration file must not end
	/// the base.
	Refuse,
	/// Leave the process. A node is one cartridge, and nothing inside Lua can
	/// stop it.
	Exit,
}

/// The safe standard library without `io` and `package`, `os` trimmed to
/// [`OS_KEPT`], no file-reading base functions, and both limits in place
/// before any Lua runs. `0` turns a limit off.
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

/// Refuse Lua that runs `instructions` without handing control back. mlua
/// refills the count on every resume it drives — each entry from the base and
/// each return from an await — but not on a plain `call` or a
/// `coroutine.resume` inside Lua. The hook must never yield: the resume after
/// it would refill.
fn budget(lua: &Lua, instructions: i64, overrun: Overrun) -> mlua::Result<()> {
	let left = Arc::new(AtomicI64::new(instructions));
	let refusals = Arc::new(AtomicU64::new(0));
	let (spent, refused) = (left.clone(), refusals.clone());
	// Global and first: mlua gives it to every thread it makes, pooled ones
	// included, and Lua copies it into every coroutine. `install` (node.rs)
	// makes pooled threads, so the hook must exist before it runs.
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

/// The limits the base's own files run under: the declared ones, because
/// settling the configured ones is what evaluating these files is for.
fn declared() -> crate::settings::Host {
	serde_json::from_value(crate::settings::defaults(crate::settings::host_specs()))
		.expect("the host's declared defaults match their type")
}

/// One trusted Lua file as data, in a state of its own that is dropped with
/// the answer, so nothing a file leaves behind reaches the next file or the
/// next reload. Deserialized straight from Lua: through JSON an empty table
/// is a map, and `return {}` is a valid empty profile.
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
