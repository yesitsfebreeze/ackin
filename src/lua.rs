use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::cartridge::Remote;
use crate::context::LuaCtx;
use crate::runtime::{Component, Disposer, Error, Runtime, Value};
use futures::StreamExt;
use mlua::thread::ThreadStatus;
use mlua::{Function, Lua, LuaSerdeExt, Table};

struct Process {
	command: Vec<String>,
	inject: Vec<String>,
}

impl mlua::UserData for Process {}
use parking_lot::Mutex;
use tokio::sync::broadcast;

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
	pub fn new(rt: Arc<Runtime>, dir: impl Into<PathBuf>, profile: impl Into<PathBuf>) -> Arc<Self> {
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
		let (outbox, _) = broadcast::channel(1024);
		let dir: PathBuf = dir.into();
		let profile: PathBuf = profile.into();
		let lua = Lua::new();
		let zirkle = lua.create_table().expect("zirkle table");
		zirkle
			.set(
				"process",
				lua
					.create_function(|lua, (command, options): (mlua::Value, Option<Table>)| {
						let command = match command {
							mlua::Value::String(s) => crate::cartridge::split(s.to_str()?.as_ref()),
							value => lua.from_value::<Vec<String>>(value)?,
						};
						if command.is_empty() || command[0].is_empty() {
							return Err(mlua::Error::RuntimeError(
								"zirkle.process needs a nonempty command".into(),
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
					})
					.expect("process helper"),
			)
			.expect("zirkle.process");
		zirkle
			.set(
				"turn",
				lua
					.create_function(|_, ()| Ok(crate::turn::current().map(|id| id.to_string())))
					.expect("turn helper"),
			)
			.expect("zirkle.turn");
		lua.globals().set("zirkle", zirkle).expect("zirkle global");
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
		self
			.profile
			.parent()
			.unwrap_or(&self.profile)
			.join("logs/debug.log")
	}

	/// Debug mode is one diagnostic setting: while it is on, every message the
	/// host puts on its socket is also appended to [`Host::debug_log`], so a
	/// shell can read the evidence after the fact instead of holding `zirkle tail`
	/// open. `on` selects; `None` only reports. Leaving drops the tap, which is
	/// the whole setting, so the host is back to normal.
	pub fn debug(self: &Arc<Self>, on: Option<bool>) -> serde_json::Value {
		use std::io::Write;
		let log = self.debug_log();
		let mut tap = self.debug.lock();
		match on {
			Some(true) if tap.is_none() => {
				let _ = std::fs::create_dir_all(log.parent().expect("debug log parent"));
				// The same two streams the socket writer merges: cartridge messages
				// and fiber transitions.
				let (mut outbox, mut lifecycle) = (self.outbox(), self.rt.lifecycle());
				let path = log.clone();
				*tap = Some(tokio::spawn(async move {
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
						let opened = std::fs::OpenOptions::new()
							.create(true)
							.append(true)
							.open(&path);
						let Ok(mut file) = opened else { break };
						if writeln!(file, "{message}").is_err() {
							break;
						}
					}
				}));
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
		crate::turn::diagnostic(cartridge, &message, serde_json::Value::Null);
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
	) -> Result<Component, mlua::Error> {
		self
			.load_component(path, config, &[])
			.map(|(component, _)| component)
	}

	pub(crate) fn load_component(
		self: &Arc<Self>,
		path: &Path,
		mut config: serde_json::Value,
		extra: &[String],
	) -> mlua::Result<(Component, Vec<PathBuf>)> {
		let declared = crate::loader::resolve(path)?;
		let (path, name, mut files) = (
			declared.entry.clone(),
			declared.name.clone(),
			declared.sources.clone(),
		);
		// The document carries the cartridge's own configuration; a caller
		// that names none inherits it, and one that names its own wins.
		if config.is_null() && !declared.config.is_null() {
			config = declared.config.clone();
		}
		if self.yolo && matches!(name.as_str(), "agent" | "memo") {
			if config.is_null() {
				config = serde_json::json!({});
			}
			config
				.as_object_mut()
				.ok_or_else(|| mlua::Error::RuntimeError("yolo requires object cartridge config".into()))?
				.insert("yolo".into(), true.into());
		}
		let source = std::fs::read_to_string(&path).map_err(mlua::Error::external)?;
		let module: mlua::Value = self
			.lua
			.load(&source)
			.set_name(path.to_string_lossy())
			.eval()?;
		if let mlua::Value::UserData(descriptor) = &module {
			let descriptor = descriptor.borrow::<Process>()?;
			let mut command = descriptor.command.clone();
			let root = path.parent().expect("resolved entry parent");
			let executable =
				crate::cartridge::executable(&command[0], root).map_err(mlua::Error::external)?;
			command[0] = executable.to_string_lossy().into_owned();
			files.push(executable);
			let component = crate::cartridge::component(self.clone(), name, command, config)
				.map_err(mlua::Error::external)?;
			let component = declarations(
				component,
				descriptor.inject.iter().chain(extra).cloned(),
				&declared,
			)?;
			return Ok((component, files));
		}
		let module = match module {
			mlua::Value::Table(module) => module,
			_ => {
				return Err(mlua::Error::RuntimeError(
					"cartridge must return a component table or zirkle.process descriptor".into(),
				))
			}
		};
		let apply: Function = module.get("apply")?;
		let keys = |field: &str| -> Result<Vec<String>, mlua::Error> {
			match module.get::<mlua::Value>(field)? {
				mlua::Value::Nil => Ok(Vec::new()),
				value => self.lua.from_value(value),
			}
		};
		let inject = keys("inject")?;
		let provide = keys("provide")?;
		let config = self.lua.to_value(&config)?;
		let host = self.clone();
		let cartridge = name.clone();
		let component = Component::new(
			name,
			Arc::new(move |ctx| {
				let thread = match host.lua.create_thread(apply.clone()) {
					Ok(t) => t,
					Err(e) => {
						return futures::stream::once(async move { Err(Error::Apply(e.to_string())) }).boxed()
					}
				};
				let host = host.clone();
				let mut first = Some((
					LuaCtx::new(host.clone(), ctx, cartridge.clone()),
					config.clone(),
				));
				futures::stream::iter(std::iter::from_fn(move || {
					if !matches!(thread.status(), ThreadStatus::Resumable) {
						return None;
					}
					let yielded = match first.take() {
						Some(args) => thread.resume::<mlua::Value>(args),
						None => thread.resume::<mlua::Value>(()),
					};
					Some(match yielded {
						Ok(v) => Ok(host.disposer(v)),
						Err(e) => Err(Error::Apply(e.to_string())),
					})
				}))
				.boxed()
			}),
		)
		.inject(inject)
		.provide(provide);
		Ok((
			declarations(component, extra.iter().cloned(), &declared)?,
			files,
		))
	}

	pub(crate) fn disposer(&self, v: mlua::Value) -> Disposer {
		match v {
			mlua::Value::Function(f) => Box::new(move || {
				Box::pin(async move {
					let _ = f.call::<()>(());
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
				let value = v.clone();
				let lua = self.lua.clone();
				return self
					.lua
					.create_function(move |_, args: mlua::Value| {
						let service = value.downcast_ref::<crate::service::Service>().unwrap();
						let guard = service
							.reload
							.gate()
							.try_read_owned()
							.map_err(|_| mlua::Error::RuntimeError("service is being replaced".into()))?;
						let current = service.value();
						let result = if let Some(remote) = current.downcast_ref::<Remote>() {
							block_on(remote.call(lua.from_value(args)?))
								.map_err(mlua::Error::RuntimeError)
								.and_then(|out| lua.to_value(&out))
						} else if let Some(mlua::Value::Function(f)) = current.downcast_ref::<mlua::Value>() {
							f.call(args)
						} else {
							Err(mlua::Error::RuntimeError(
								"service is no longer callable".into(),
							))
						};
						drop(guard);
						result
					})
					.map(mlua::Value::Function)
					.unwrap_or(mlua::Value::Nil);
			}
			return self.to_lua(&current);
		}
		if let Some(v) = v.downcast_ref::<mlua::Value>() {
			return v.clone();
		}
		if let Some(remote) = v.downcast_ref::<Remote>() {
			let remote = remote.clone();
			return self
				.lua
				.create_function(move |lua, args: mlua::Value| {
					let args: serde_json::Value = lua.from_value(args)?;
					let out = block_on(remote.call(args)).map_err(mlua::Error::RuntimeError)?;
					lua.to_value(&out)
				})
				.map(mlua::Value::Function)
				.unwrap_or(mlua::Value::Nil);
		}
		v.downcast_ref::<serde_json::Value>()
			.and_then(|j| self.lua.to_value(j).ok())
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
	) -> Result<serde_json::Value, String> {
		if let Some(service) = v.downcast_ref::<crate::service::Service>() {
			return service.call(self, args).await;
		}
		self.invoke_raw(v, args).await
	}

	pub(crate) async fn invoke_raw(
		&self,
		v: Value,
		args: serde_json::Value,
	) -> Result<serde_json::Value, String> {
		if let Some(remote) = v.downcast_ref::<Remote>() {
			return remote.call(args).await;
		}
		match v.downcast_ref::<mlua::Value>() {
			Some(mlua::Value::Function(f)) => {
				let args = self.lua.to_value(&args).map_err(|e| e.to_string())?;
				f.call::<mlua::Value>(args)
					.map(|r| self.to_json(r))
					.map_err(|e| e.to_string())
			}
			_ => Ok(self.json_of(&v)),
		}
	}

	/// A socket client's call: bare store lookup, no access check.
	pub async fn call(
		&self,
		key: &str,
		args: serde_json::Value,
	) -> Result<serde_json::Value, String> {
		let v = self
			.rt
			.ctx()
			.peek(key)
			.ok_or_else(|| format!("`{key}` is not provided"))?;
		self.invoke(v, args).await
	}
}

pub(crate) fn block_on<F: std::future::Future>(f: F) -> F::Output {
	tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(f))
}

/// Declarations are settled before the runtime constructs a fiber.
fn declarations(
	mut component: Component,
	extra: impl IntoIterator<Item = String>,
	declared: &crate::loader::Declared,
) -> mlua::Result<Component> {
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
			return Err(mlua::Error::RuntimeError(format!(
				"dependency `{key}` must be a nonempty exact key (wildcards are unsupported)"
			)));
		}
	}
	let mut seen = std::collections::HashSet::new();
	component.inject.retain(|key| seen.insert(key.clone()));
	seen.clear();
	for key in &component.provide {
		if !seen.insert(key.clone()) {
			return Err(mlua::Error::RuntimeError(format!(
				"duplicate provide declaration `{key}`"
			)));
		}
	}
	Ok(component)
}
