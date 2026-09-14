//! The Lua composition surface: one interpreter per host, sandboxed, driving
//! every cartridge's `apply` as an asynchronous coroutine.
//!
//! A cartridge's `apply` runs on a Lua thread the host polls as a stream: every
//! `coroutine.yield` is one effect boundary, and every host call the cartridge
//! makes that has to wait — a `bail`, a `gather`, a call over the wire — is an
//! async function, so the coroutine parks on the runtime instead of blocking a
//! worker thread. Nothing here calls `block_on`.
//!
//! The interpreter is [`interpreter`]: the safe standard library without `io`
//! and `package`, and an `os` that reads the clock and the environment but
//! cannot execute, remove or rename anything. A Lua cartridge is confined the
//! way a process cartridge is confined by its grant — by what it can reach.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::cartridge::Remote;
use crate::context::LuaCtx;
use crate::error::{Error, Result};
use crate::runtime::{Component, Disposer, Error as RuntimeError, Runtime, Value};
use futures::StreamExt;
use mlua::{Function, Lua, LuaOptions, LuaSerdeExt, StdLib, Table};
use parking_lot::Mutex;
use tokio::sync::broadcast;

/// The `os` functions a cartridge keeps: reading the clock and the environment
/// is observation; everything else in the library acts on the machine.
const OS_KEPT: &[&str] = &["clock", "date", "difftime", "getenv", "time"];

/// Base-library functions that read files, which a cartridge never does
/// through the interpreter.
const BASE_DROPPED: &[&str] = &["dofile", "loadfile"];

/// A fresh interpreter every cartridge and configuration file runs in.
///
/// The safe standard library minus `io` and `package` (so no `require`), with
/// `os` trimmed to [`OS_KEPT`] and the file-reading base functions removed.
/// `debug` and FFI are never loaded. What is left is computation over the
/// values the host hands in, which is all a composition needs.
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

struct Process {
	command: Vec<String>,
	inject: Vec<String>,
}

impl mlua::UserData for Process {}

pub struct Host {
	/// Launch-scoped automatic execution mode, retained across reloads.
	yolo: bool,
	pub(crate) lua: Lua,
	pub(crate) rt: Arc<Runtime>,
	pub(crate) dir: PathBuf,
	pub(crate) profile: PathBuf,
	outbox: broadcast::Sender<serde_json::Value>,
	pub(crate) loaded: Mutex<Vec<crate::loader::Loaded>>,
	/// The entry set a one-cartridge verify runs, in place of the profile's.
	/// `None` everywhere else, so the profile stays the manifest of record.
	pub(crate) solo: Mutex<Option<Vec<crate::loader::Entry>>>,
	pub(crate) reload_lock: tokio::sync::Mutex<()>,
	/// The debug tap, present exactly while debug mode is on.
	debug: Mutex<Option<tokio::task::JoinHandle<()>>>,
	/// The chain link: a node's needs bound to the dependency that provides
	/// them, over the dependency's socket. Empty everywhere but node mode,
	/// where the chain is the resolution the ledger's walk already made.
	deps: Mutex<HashMap<String, crate::cartridge::Remote>>,
}

impl Host {
	/// A host with automatic execution off.
	pub fn new(
		rt: Arc<Runtime>,
		dir: impl Into<PathBuf>,
		profile: impl Into<PathBuf>,
	) -> Arc<Self> {
		Self::with_yolo(rt, dir, profile, false)
	}

	/// `dir` holds the cartridge files, `profile` the `init.lua` and `config.lua`
	/// that compose them.
	pub fn with_yolo(
		rt: Arc<Runtime>,
		dir: impl Into<PathBuf>,
		profile: impl Into<PathBuf>,
		yolo: bool,
	) -> Arc<Self> {
		let (outbox, _) = broadcast::channel(crate::settings::host().outbox_queue);
		let dir: PathBuf = dir.into();
		let profile: PathBuf = profile.into();
		let lua = interpreter().expect("the Lua interpreter builds");
		let limit = crate::settings::host().lua_memory_bytes;
		if limit > 0 {
			lua.set_memory_limit(limit)
				.expect("a memory limit on a fresh interpreter");
		}
		install_globals(&lua).expect("the cartridge global builds");
		Arc::new(Self {
			yolo,
			lua,
			rt,
			dir: dir.canonicalize().unwrap_or(dir),
			profile: profile.canonicalize().unwrap_or(profile),
			outbox,
			loaded: Mutex::new(Vec::new()),
			solo: Mutex::new(None),
			reload_lock: tokio::sync::Mutex::new(()),
			debug: Mutex::new(None),
			deps: Mutex::new(HashMap::new()),
		})
	}

	/// Where debug mode writes, beside the profile that composed this host.
	pub fn debug_log(&self) -> PathBuf {
		self.profile
			.parent()
			.unwrap_or(&self.profile)
			.join("logs/debug.log")
	}

	/// Debug mode is one diagnostic setting: while it is on, every message the
	/// host puts on its socket is also appended to [`Host::debug_log`], so a
	/// shell can read the evidence after the fact instead of holding `cartridge tail`
	/// open. `on` selects; `None` only reports. Leaving drops the tap, which is
	/// the whole setting, so the host is back to normal.
	pub fn debug(self: &Arc<Self>, on: Option<bool>) -> serde_json::Value {
		let log = self.debug_log();
		let mut tap = self.debug.lock();
		match on {
			Some(true) if tap.is_none() => {
				// The same two streams the socket writer merges: cartridge messages
				// and fiber transitions.
				let (outbox, lifecycle) = (self.outbox(), self.rt.lifecycle());
				*tap = Some(tokio::spawn(debug_tap(log.clone(), outbox, lifecycle)));
			}
			Some(false) => {
				if let Some(tap) = tap.take() {
					tap.abort();
				}
			}
			_ => {}
		}
		serde_json::json!({
			"on": tap.is_some(),
			"log": log,
			"bytes": std::fs::metadata(&log).map_or(0, |m| m.len()),
		})
	}

	pub fn runtime(&self) -> &Arc<Runtime> {
		&self.rt
	}

	/// Bind one chain need to its provider's remote: the value a `ctx:get` of
	/// that need resolves to from now on, on this node and on every fiber
	/// nested under it that walks out to the host. Node mode only — a host
	/// with no chain has no dependency to bind.
	pub fn bind_dependency(&self, key: &str, remote: crate::cartridge::Remote) {
		self.deps.lock().insert(key.to_owned(), remote);
	}

	/// The remote a chain need resolves to, when this host's node mode holds one.
	pub fn dependency(&self, key: &str) -> Option<crate::runtime::Value> {
		self.deps
			.lock()
			.get(key)
			.map(|remote| Arc::new(remote.clone()) as crate::runtime::Value)
	}

	pub fn dir(&self) -> &Path {
		&self.dir
	}

	pub fn profile(&self) -> &Path {
		&self.profile
	}

	pub fn outbox(&self) -> broadcast::Receiver<serde_json::Value> {
		self.outbox.subscribe()
	}

	pub(crate) fn send(&self, message: serde_json::Value) {
		let _ = self.outbox.send(message);
	}

	pub(crate) fn report(&self, cartridge: &str, message: impl std::fmt::Display) {
		tracing::error!(target: "cartridge", cartridge = %cartridge, "{message}");
		self.send(
			serde_json::json!({ "error": { "cartridge": cartridge, "message": message.to_string() } }),
		);
		// Errors are events: the failure lands on the stream too, so a watcher
		// of this cartridge's channel learns of it without calling into it.
		self.rt.stream().publish(
			cartridge,
			cartridge,
			crate::stream::Kind::Error,
			serde_json::json!(message.to_string()),
		);
	}

	/// The envelope a cartridge's `send` puts on the socket.
	pub(crate) fn send_event(&self, name: &str, data: serde_json::Value) {
		self.send(serde_json::json!({ "event": name, "data": data }));
	}

	pub fn emit(&self, name: &str, data: serde_json::Value) {
		self.rt.ctx().emit(name, Arc::new(data));
	}

	pub fn component(
		self: &Arc<Self>,
		path: &Path,
		config: serde_json::Value,
	) -> Result<Component> {
		self.load_component(path, config, &[])
			.map(|(component, _)| component)
	}

	pub(crate) fn load_component(
		self: &Arc<Self>,
		path: &Path,
		config: serde_json::Value,
		extra: &[String],
	) -> Result<(Component, Vec<PathBuf>)> {
		let declared = crate::loader::resolve(path)?;
		let (path, name, mut files) = (
			declared.entry.clone(),
			declared.name.clone(),
			declared.sources.clone(),
		);
		let config = self.settle_config(&declared, config)?;
		let source = std::fs::read_to_string(&path).map_err(|e| Error::file(&path, e))?;
		let module: mlua::Value = self
			.lua
			.load(&source)
			.set_name(path.to_string_lossy())
			.eval()?;
		if let mlua::Value::UserData(descriptor) = &module {
			let descriptor = descriptor.borrow::<Process>()?;
			let mut command = descriptor.command.clone();
			let root = path.parent().expect("resolved entry parent");
			let executable = crate::cartridge::executable(&command[0], root)?;
			command[0] = executable.to_string_lossy().into_owned();
			files.push(executable);
			let component = crate::cartridge::component(self.clone(), name, command, config)?;
			let component = declarations(
				component,
				descriptor.inject.iter().chain(extra).cloned(),
				&declared,
			)?;
			return Ok((component, files));
		}
		let mlua::Value::Table(module) = module else {
			return Err(Error::Profile(format!(
				"{}: cartridge must return a component table or cartridge.process descriptor",
				path.display()
			)));
		};
		let apply: Function = module.get("apply")?;
		let keys = |field: &str| -> mlua::Result<Vec<String>> {
			match module.get::<mlua::Value>(field)? {
				mlua::Value::Nil => Ok(Vec::new()),
				value => self.lua.from_value(value),
			}
		};
		let inject = keys("inject")?;
		let provide = keys("provide")?;
		let config = self.lua.to_value(&config)?;
		let component = Component::new(name.clone(), self.coroutine(apply, name, config))
			.inject(inject)
			.provide(provide);
		Ok((
			declarations(component, extra.iter().cloned(), &declared)?,
			files,
		))
	}

	/// The configuration a component is handed, settled once so that every
	/// way in — a profile entry, the ledger, a nested load — hands it the same
	/// complete table. Declared defaults first, then the document's own
	/// `config`, then what the caller names, each laid over the last field by
	/// field. A caller that names one key keeps the rest.
	fn settle_config(
		&self,
		declared: &crate::loader::Declared,
		config: serde_json::Value,
	) -> Result<serde_json::Value> {
		let mut settled = crate::settings::defaults(&declared.settings);
		let mut carried = false;
		for layer in [declared.config.clone(), config] {
			if !layer.is_null() {
				crate::settings::merge(&mut settled, layer);
				carried = true;
			}
		}
		// Absent and empty are different answers, and a cartridge reads them
		// differently: `{}` is a configuration that names nothing, and `null` is
		// no configuration at all. Nothing declared and no layer carrying
		// anything stays `null`, so a cartridge that never had a configuration
		// does not start seeing one — but a caller that passed `{}` keeps it.
		let mut config = match declared.settings.is_empty() && !carried {
			true => serde_json::Value::Null,
			false => crate::settings::apply(&declared.settings, settled, &declared.name)?,
		};
		if self.yolo && matches!(declared.name.as_str(), "agent" | "memo") {
			if config.is_null() {
				config = serde_json::json!({});
			}
			config
				.as_object_mut()
				.ok_or_else(|| Error::Settings("yolo requires object cartridge config".into()))?
				.insert("yolo".into(), true.into());
		}
		Ok(config)
	}

	/// The apply of a Lua component: one coroutine per fiber, polled as a
	/// stream of effect boundaries. Every yielded value is a disposer; a host
	/// call that waits parks the coroutine on the runtime.
	fn coroutine(
		self: &Arc<Self>,
		apply: Function,
		cartridge: String,
		config: mlua::Value,
	) -> crate::runtime::Apply {
		let host = self.clone();
		Arc::new(move |ctx| {
			let failed = |e: mlua::Error| {
				futures::stream::once(async move { Err(RuntimeError::Apply(e.to_string())) })
					.boxed()
			};
			let thread = match host.lua.create_thread(apply.clone()) {
				Ok(thread) => thread,
				Err(e) => return failed(e),
			};
			let first = (
				LuaCtx::new(host.clone(), ctx, cartridge.clone()),
				config.clone(),
			);
			let yields = match thread.into_async::<mlua::Value>(first) {
				Ok(yields) => yields,
				Err(e) => return failed(e),
			};
			let host = host.clone();
			yields
				.map(move |yielded| match yielded {
					Ok(v) => Ok(host.disposer(v)),
					Err(e) => Err(RuntimeError::Apply(e.to_string())),
				})
				.boxed()
		})
	}

	pub(crate) fn disposer(&self, v: mlua::Value) -> Disposer {
		match v {
			mlua::Value::Function(f) => Box::new(move || {
				Box::pin(async move {
					if let Err(error) = f.call_async::<()>(()).await {
						tracing::warn!(target: "cartridge", "disposer failed: {error}");
					}
				})
			}),
			_ => Box::new(|| Box::pin(async {})),
		}
	}

	pub(crate) fn to_lua(&self, v: &Value) -> mlua::Value {
		if let Some(service) = v.downcast_ref::<crate::service::Service>() {
			let current = service.value();
			if current.downcast_ref::<Remote>().is_some()
				|| matches!(
					current.downcast_ref::<mlua::Value>(),
					Some(mlua::Value::Function(_))
				) {
				return self.service_function(v.clone());
			}
			return self.to_lua(&current);
		}
		if let Some(v) = v.downcast_ref::<mlua::Value>() {
			return v.clone();
		}
		if let Some(remote) = v.downcast_ref::<Remote>() {
			return self.remote_function(remote.clone());
		}
		v.downcast_ref::<serde_json::Value>()
			.and_then(|j| self.lua.to_value(j).ok())
			.unwrap_or(mlua::Value::Nil)
	}

	/// A resident service as a Lua function: every call reads the current
	/// generation under the reload gate, so a call in flight finishes on the
	/// generation it started on and a switch waits for it.
	fn service_function(&self, value: Value) -> mlua::Value {
		self.lua
			.create_async_function(move |lua, args: mlua::Value| {
				let value = value.clone();
				async move {
					let service = value
						.downcast_ref::<crate::service::Service>()
						.expect("a service value");
					let guard = service.reload.gate().try_read_owned().map_err(|_| {
						mlua::Error::RuntimeError("service is being replaced".into())
					})?;
					let current = service.value();
					let result = if let Some(remote) = current.downcast_ref::<Remote>() {
						let out = remote.call(lua.from_value(args)?).await?;
						lua.to_value(&out)
					} else if let Some(mlua::Value::Function(f)) =
						current.downcast_ref::<mlua::Value>()
					{
						f.call_async::<mlua::Value>(args).await
					} else {
						Err(mlua::Error::RuntimeError(
							"service is no longer callable".into(),
						))
					};
					drop(guard);
					result
				}
			})
			.map(mlua::Value::Function)
			.unwrap_or(mlua::Value::Nil)
	}

	/// A process cartridge's key as a Lua function: the call crosses the wire
	/// and the coroutine parks until the reply lands.
	fn remote_function(&self, remote: Remote) -> mlua::Value {
		self.lua
			.create_async_function(move |lua, args: mlua::Value| {
				let remote = remote.clone();
				async move {
					let args: serde_json::Value = lua.from_value(args)?;
					let out = remote.call(args).await?;
					lua.to_value(&out)
				}
			})
			.map(mlua::Value::Function)
			.unwrap_or(mlua::Value::Nil)
	}

	pub(crate) fn to_json(&self, v: mlua::Value) -> serde_json::Value {
		self.lua.from_value(v).unwrap_or(serde_json::Value::Null)
	}

	pub(crate) fn json_of(&self, v: &Value) -> serde_json::Value {
		if let Some(service) = v.downcast_ref::<crate::service::Service>() {
			return self.json_of(&service.value());
		}
		if let Some(j) = v.downcast_ref::<serde_json::Value>() {
			return j.clone();
		}
		v.downcast_ref::<mlua::Value>()
			.map(|l| self.to_json(l.clone()))
			.unwrap_or(serde_json::Value::Null)
	}

	/// Call a provided value with one JSON argument: a Lua function, a process
	/// cartridge's [`Remote`], or plain data (returned as is).
	pub(crate) async fn invoke(
		&self,
		v: Value,
		args: serde_json::Value,
	) -> Result<serde_json::Value> {
		if let Some(service) = v.downcast_ref::<crate::service::Service>() {
			return service.call(self, args).await;
		}
		self.invoke_raw(v, args).await
	}

	pub(crate) async fn invoke_raw(
		&self,
		v: Value,
		args: serde_json::Value,
	) -> Result<serde_json::Value> {
		if let Some(remote) = v.downcast_ref::<Remote>() {
			return remote.call(args).await;
		}
		match v.downcast_ref::<mlua::Value>() {
			Some(mlua::Value::Function(f)) => {
				let args = self.lua.to_value(&args)?;
				let out = f.call_async::<mlua::Value>(args).await?;
				Ok(self.to_json(out))
			}
			_ => Ok(self.json_of(&v)),
		}
	}

	/// A socket client's call: bare store lookup, no access check.
	pub async fn call(&self, key: &str, args: serde_json::Value) -> Result<serde_json::Value> {
		let v = self
			.rt
			.ctx()
			.peek(key)
			.ok_or_else(|| Error::NotProvided(key.to_owned()))?;
		self.invoke(v, args).await
	}
}

/// The `cartridge` global every entry sees: `cartridge.process` names a
/// process component and `cartridge.trace` reads the ambient trace.
fn install_globals(lua: &Lua) -> mlua::Result<()> {
	let cartridge = lua.create_table()?;
	cartridge.set(
		"process",
		lua.create_function(|lua, (command, options): (mlua::Value, Option<Table>)| {
			let command = match command {
				mlua::Value::String(s) => crate::cartridge::split(s.to_str()?.as_ref()),
				value => lua.from_value::<Vec<String>>(value)?,
			};
			if command.is_empty() || command[0].is_empty() {
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
		lua.create_function(|_, ()| Ok(crate::trace::current().map(|id| id.to_string())))?,
	)?;
	lua.globals().set("cartridge", cartridge)
}

/// The debug tap: the file is opened once and every merged message is one
/// appended line, flushed as it lands so a reader sees it at once. The tap
/// ends with the streams it reads, or when the file refuses a write.
async fn debug_tap(
	path: PathBuf,
	mut outbox: broadcast::Receiver<serde_json::Value>,
	mut lifecycle: broadcast::Receiver<crate::runtime::Transition>,
) {
	use tokio::io::AsyncWriteExt;
	if let Some(parent) = path.parent() {
		let _ = tokio::fs::create_dir_all(parent).await;
	}
	let file = tokio::fs::OpenOptions::new()
		.create(true)
		.append(true)
		.open(&path)
		.await;
	let mut file = match file {
		Ok(file) => tokio::io::BufWriter::new(file),
		Err(error) => {
			tracing::warn!(target: "cartridge", path = %path.display(), "debug log: {error}");
			return;
		}
	};
	loop {
		let message = tokio::select! {
			m = outbox.recv() => match m {
				Ok(m) => m,
				Err(broadcast::error::RecvError::Lagged(_)) => continue,
				Err(_) => break,
			},
			t = lifecycle.recv() => match t {
				Ok(t) => serde_json::json!({ "fiber": t }),
				Err(broadcast::error::RecvError::Lagged(_)) => continue,
				Err(_) => break,
			},
		};
		let line = format!("{message}\n");
		if file.write_all(line.as_bytes()).await.is_err() || file.flush().await.is_err() {
			break;
		}
	}
}

/// Declarations are settled before the runtime constructs a fiber.
fn declarations(
	mut component: Component,
	extra: impl IntoIterator<Item = String>,
	declared: &crate::loader::Declared,
) -> Result<Component> {
	// The document is the declaration when it makes one: what a cartridge takes
	// away when it goes is exactly what it declared on the way in, so a key the
	// entry offers and the document never names is not offered. A document that
	// declares nothing leaves the entry as the only source, which is how every
	// cartridge written before the document carried these fields still loads.
	if !declared.provide.is_empty() {
		component.provide = declared.provide.clone();
	}
	if !declared.needs.is_empty() {
		component.inject = declared.needs.clone();
	}
	component.inject.extend(extra);
	for key in component.inject.iter().chain(&component.provide) {
		if key.trim().is_empty() || key.contains('*') {
			return Err(Error::Profile(format!(
				"dependency `{key}` must be a nonempty exact key (wildcards are unsupported)"
			)));
		}
	}
	let mut seen = std::collections::HashSet::new();
	component.inject.retain(|key| seen.insert(key.clone()));
	seen.clear();
	for key in &component.provide {
		if !seen.insert(key.clone()) {
			return Err(Error::Profile(format!(
				"duplicate provide declaration `{key}`"
			)));
		}
	}
	Ok(component)
}

#[cfg(test)]
#[path = "../.cartridge/tests/unit/src/lua/tests.rs"]
mod tests;
