//! A node: one cartridge's `init.lua`, run by the base in its own process with
//! the `cartridge` global injected. Everything the entry registers goes to the
//! base; native modules it loads reach the same global.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::FutureExt;
use mlua::{Function, Lua, LuaSerdeExt, Table, UserData, UserDataMethods};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::oneshot;

use crate::error::{Error, Result};
use crate::transport::cartridge::{self, Ctx, CONNECT_TIMEOUT_ENV, HOST_TOKEN_ENV, SOCKET_ENV};

pub const ENTRY_ENV: &str = "CARTRIDGE_ENTRY";
pub const ROOT_ENV: &str = "CARTRIDGE_ROOT";
pub const ON_ENV: &str = "CARTRIDGE_ON";

static RUNTIME: std::sync::OnceLock<tokio::runtime::Handle> = std::sync::OnceLock::new();

/// Wait for a future from a Lua call: on a runtime thread inside a handler, or
/// on a thread of a native module's own.
fn wait<F: std::future::Future>(future: F) -> F::Output {
	let handle = RUNTIME.get().expect("the node runtime").clone();
	match tokio::runtime::Handle::try_current() {
		Ok(_) => tokio::task::block_in_place(|| handle.block_on(future)),
		Err(_) => handle.block_on(future),
	}
}

fn external(error: String) -> mlua::Error {
	mlua::Error::RuntimeError(error)
}

fn json(lua: &Lua, value: mlua::Value) -> mlua::Result<Value> {
	lua.from_value(value)
}

fn env(name: &str) -> Result<String> {
	std::env::var(name)
		.map_err(|_| Error::Profile(format!("{name} must be set; a node is started by the base")))
}

/// Run this process as a node until the base disposes it or goes away.
pub async fn main() -> Result<ExitCode> {
	let socket = PathBuf::from(env(SOCKET_ENV)?);
	let host_token = env(HOST_TOKEN_ENV)?;
	let entry = PathBuf::from(env(ENTRY_ENV)?);
	let root = PathBuf::from(env(ROOT_ENV)?);
	let on: Vec<String> = serde_json::from_str(&env(ON_ENV)?)?;
	let timeout = std::env::var(CONNECT_TIMEOUT_ENV)
		.ok()
		.and_then(|secs| secs.parse().ok())
		.map(Duration::from_secs)
		.unwrap_or(Duration::from_secs(30));
	let _ = RUNTIME.set(tokio::runtime::Handle::current());
	let listener = cartridge::listen(&socket)
		.await
		.map_err(|e| Error::process(entry.display().to_string(), e))?;
	let ctx = Ctx::new(host_token, timeout);
	let lua = crate::lua::interpreter()?;
	let limit = crate::settings::host().lua_memory_bytes;
	if limit > 0 {
		lua.set_memory_limit(limit)?;
	}
	install(&lua, ctx.clone(), root, on)?;
	let lifeline = ctx.clone();
	std::thread::spawn(move || {
		let _ = std::io::copy(&mut std::io::stdin(), &mut std::io::sink());
		lifeline.stop();
	});
	let apply_lua = lua.clone();
	let apply = Box::new(move |ctx: Ctx, config: Value| {
		async move { tokio::task::block_in_place(|| apply(&apply_lua, &ctx, &entry, config)) }
			.boxed()
	});
	cartridge::serve(listener, ctx, apply).await;
	Ok(ExitCode::SUCCESS)
}

/// Run `init.lua` with the config the base handed over.
fn apply(lua: &Lua, ctx: &Ctx, entry: &Path, config: Value) -> cartridge::Result<()> {
	let run = || -> mlua::Result<()> {
		let global: Table = lua.globals().get("cartridge")?;
		global.set("name", ctx.name())?;
		global.set("config", lua.to_value(&config)?)?;
		let source = std::fs::read_to_string(entry).map_err(mlua::Error::external)?;
		let returned: mlua::Value = lua.load(&source).set_name(entry.to_string_lossy()).eval()?;
		let disposer = match returned {
			mlua::Value::Table(table) => match table.get::<mlua::Value>("apply")? {
				mlua::Value::Function(apply) => apply
					.call::<mlua::Value>((global.clone(), global.get::<mlua::Value>("config")?))?,
				_ => mlua::Value::Nil,
			},
			other => other,
		};
		if let mlua::Value::Function(dispose) = disposer {
			ctx.on_dispose(move || async move {
				if let Err(error) = tokio::task::block_in_place(|| dispose.call::<()>(())) {
					tracing::warn!(target: "cartridge", "disposer failed: {error}");
				}
			});
		}
		Ok(())
	};
	run().map_err(|e| e.to_string())
}

/// The `cartridge` global: the base's event system, streams, host queries, and
/// the two ways a cartridge brings code of its own.
fn install(lua: &Lua, ctx: Ctx, root: PathBuf, on: Vec<String>) -> mlua::Result<()> {
	let global = lua.create_table()?;
	global.set("root", root.to_string_lossy().into_owned())?;
	global.set(
		"trace",
		lua.create_function(|_, ()| Ok(cartridge::trace()))?,
	)?;
	global.set("needs", {
		let ctx = ctx.clone();
		lua.create_function(move |_, ()| Ok(ctx.needs()))?
	})?;
	global.set("events", {
		let ctx = ctx.clone();
		lua.create_function(move |lua, ()| lua.to_value(&ctx.events()))?
	})?;
	global.set("on", {
		let (ctx, lua_handle) = (ctx.clone(), lua.clone());
		lua.create_function(move |_, (name, f): (String, Function)| {
			if !on.contains(&name) {
				return Err(external(format!("`{name}` is not declared in `on`")));
			}
			let lua = lua_handle.clone();
			ctx.on(&name, move |data| {
				let (lua, f) = (lua.clone(), f.clone());
				async move {
					tokio::task::block_in_place(|| -> mlua::Result<Value> {
						let out: mlua::Value = f.call(lua.to_value(&data)?)?;
						lua.from_value(out)
					})
					.map_err(|e| e.to_string())
				}
			});
			Ok(())
		})?
	})?;
	global.set("on_dispose", {
		let ctx = ctx.clone();
		lua.create_function(move |_, f: Function| {
			ctx.on_dispose(move || async move {
				if let Err(error) = tokio::task::block_in_place(|| f.call::<()>(())) {
					tracing::warn!(target: "cartridge", "disposer failed: {error}");
				}
			});
			Ok(())
		})?
	})?;
	global.set("emit", {
		let ctx = ctx.clone();
		lua.create_function(move |lua, (name, data): (String, mlua::Value)| {
			let data = json(lua, data)?;
			wait(async { ctx.emit(&name, data) }).map_err(external)
		})?
	})?;
	global.set("notify", {
		let ctx = ctx.clone();
		lua.create_function(move |lua, (name, data): (String, mlua::Value)| {
			let data = json(lua, data)?;
			wait(async { ctx.notify(&name, data) }).map_err(external)
		})?
	})?;
	global.set("bail", {
		let ctx = ctx.clone();
		lua.create_function(move |lua, (name, data): (String, mlua::Value)| {
			let answer = wait(ctx.bail(&name, json(lua, data)?)).map_err(external)?;
			match answer {
				Some(answer) => lua.to_value(&answer),
				None => Ok(mlua::Value::Nil),
			}
		})?
	})?;
	global.set("parallel", {
		let ctx = ctx.clone();
		lua.create_function(move |lua, (name, data): (String, mlua::Value)| {
			wait(ctx.parallel(&name, json(lua, data)?)).map_err(external)
		})?
	})?;
	global.set("gather", {
		let ctx = ctx.clone();
		lua.create_function(
			move |lua, (name, data, timeout): (String, mlua::Value, Option<u64>)| {
				let data = json(lua, data)?;
				let answers = match timeout {
					Some(ms) => wait(ctx.gather_within(&name, data, Duration::from_millis(ms))),
					None => wait(ctx.gather(&name, data)),
				}
				.map_err(external)?;
				let rows = lua.create_table()?;
				for (from, data) in answers {
					let row = lua.create_table()?;
					row.set("from", from)?;
					row.set("data", lua.to_value(&data)?)?;
					rows.push(row)?;
				}
				Ok(rows)
			},
		)?
	})?;
	global.set("publish", {
		let ctx = ctx.clone();
		lua.create_function(move |lua, (channel, data): (String, mlua::Value)| {
			ctx.publish(&channel, json(lua, data)?);
			Ok(())
		})?
	})?;
	global.set("subscribe", {
		let (ctx, lua_handle) = (ctx.clone(), lua.clone());
		lua.create_function(
			move |_, (cartridge, channel, f): (String, String, Function)| {
				let lua = lua_handle.clone();
				wait(ctx.subscribe(&cartridge, &channel, None, move |envelope| {
					let (lua, f) = (lua.clone(), f.clone());
					async move {
						let result = tokio::task::block_in_place(|| -> mlua::Result<()> {
							f.call::<()>(lua.to_value(&envelope)?)
						});
						if let Err(error) = result {
							tracing::warn!(target: "cartridge", "subscriber failed: {error}");
						}
					}
				}))
				.map_err(external)
			},
		)?
	})?;
	global.set("host", {
		let ctx = ctx.clone();
		lua.create_function(move |lua, (method, params): (String, mlua::Value)| {
			let out = wait(ctx.host(&method, json(lua, params)?)).map_err(external)?;
			lua.to_value(&out)
		})?
	})?;
	global.set("load", {
		let (root, loaded) = (root.clone(), lua.create_table()?);
		lua.create_function(move |lua, name: String| {
			if let Some(module) = loaded.get::<Option<mlua::Value>>(name.as_str())? {
				return Ok(module);
			}
			let module = load_native(lua, &root, &name)?;
			loaded.set(name.as_str(), module.clone())?;
			Ok(module)
		})?
	})?;
	global.set("spawn", lua.create_function(spawn)?)?;
	lua.globals().set("cartridge", global)
}

/// Load `lib<name>.dylib` or `<name>.so` from the cartridge folder (or its
/// `target/{release,debug}` while developing) and run its `luaopen_<name>`.
fn load_native(lua: &Lua, root: &Path, name: &str) -> mlua::Result<mlua::Value> {
	let symbol = name.replace(['-', '.'], "_");
	let files = [
		format!("lib{symbol}.dylib"),
		format!("{symbol}.dylib"),
		format!("lib{symbol}.so"),
		format!("{symbol}.so"),
	];
	let dirs = [
		root.to_path_buf(),
		root.join("target/release"),
		root.join("target/debug"),
	];
	let path = dirs
		.iter()
		.flat_map(|dir| files.iter().map(move |file| dir.join(file)))
		.find(|path| path.is_file())
		.ok_or_else(|| external(format!("no native module `{name}` in {}", root.display())))?;
	// Leaked on purpose: Lua holds functions from it for the life of the node.
	let library = unsafe { libloading::Library::new(&path) }
		.map_err(|e| external(format!("{}: {e}", path.display())))?;
	let library: &'static libloading::Library = Box::leak(Box::new(library));
	let open: libloading::Symbol<
		unsafe extern "C-unwind" fn(*mut mlua::lua_State) -> std::os::raw::c_int,
	> = unsafe { library.get(format!("luaopen_{symbol}").as_bytes()) }
		.map_err(|e| external(format!("{}: {e}", path.display())))?;
	let open = unsafe { lua.create_c_function(*open) }?;
	open.call::<mlua::Value>(name)
}

/// A helper program: lines of JSON in and out. `request` matches a reply by `id`;
/// every other line reaches the `on_line` handler.
struct Spawned {
	stdin: tokio::sync::Mutex<tokio::process::ChildStdin>,
	pending: Mutex<HashMap<u64, oneshot::Sender<Value>>>,
	on_line: Mutex<Option<Function>>,
	next: AtomicU64,
	timeout: Duration,
	child: Mutex<Option<tokio::process::Child>>,
}

fn spawn(
	lua: &Lua,
	(command, options): (mlua::Value, Option<Table>),
) -> mlua::Result<mlua::AnyUserData> {
	let command: Vec<String> = match command {
		mlua::Value::String(s) => s.to_str()?.split_whitespace().map(str::to_owned).collect(),
		value => lua.from_value(value)?,
	};
	if command.first().is_none_or(String::is_empty) {
		return Err(external("cartridge.spawn needs a nonempty command".into()));
	}
	let mut process = tokio::process::Command::new(&command[0]);
	process
		.args(&command[1..])
		.stdin(std::process::Stdio::piped())
		.stdout(std::process::Stdio::piped())
		.stderr(std::process::Stdio::inherit())
		.kill_on_drop(true);
	let mut timeout = Duration::from_secs(30);
	if let Some(options) = options {
		if let Some(cwd) = options.get::<Option<String>>("cwd")? {
			process.current_dir(cwd);
		}
		if let Some(env) = options.get::<Option<Table>>("env")? {
			for pair in env.pairs::<String, String>() {
				let (key, value) = pair?;
				process.env(key, value);
			}
		}
		if let Some(ms) = options.get::<Option<u64>>("timeout_ms")? {
			timeout = Duration::from_millis(ms);
		}
	}
	let mut child = process
		.spawn()
		.map_err(|e| external(format!("{}: {e}", command[0])))?;
	let stdin = child.stdin.take().expect("piped stdin");
	let stdout = child.stdout.take().expect("piped stdout");
	let spawned = Arc::new(Spawned {
		stdin: tokio::sync::Mutex::new(stdin),
		pending: Mutex::default(),
		on_line: Mutex::default(),
		next: AtomicU64::new(1),
		timeout,
		child: Mutex::new(Some(child)),
	});
	let (reader, reader_lua) = (spawned.clone(), lua.clone());
	tokio::spawn(async move {
		let lua = reader_lua;
		let mut lines = BufReader::new(stdout).lines();
		while let Ok(Some(line)) = lines.next_line().await {
			let value: Value = serde_json::from_str(&line).unwrap_or(Value::String(line));
			let waiter = value["id"]
				.as_u64()
				.and_then(|id| reader.pending.lock().expect("pending lock").remove(&id));
			if let Some(waiter) = waiter {
				let _ = waiter.send(value);
				continue;
			}
			let handler = reader.on_line.lock().expect("on_line lock").clone();
			if let Some(handler) = handler {
				let result = tokio::task::block_in_place(|| -> mlua::Result<()> {
					handler.call::<()>(lua.to_value(&value)?)
				});
				if let Err(error) = result {
					tracing::warn!(target: "cartridge", "line handler failed: {error}");
				}
			}
		}
	});
	lua.create_userdata(Handle(spawned))
}

struct Handle(Arc<Spawned>);

impl Spawned {
	async fn write(&self, value: &Value) -> std::io::Result<()> {
		let mut line = value.to_string();
		line.push('\n');
		let mut stdin = self.stdin.lock().await;
		stdin.write_all(line.as_bytes()).await?;
		stdin.flush().await
	}
}

impl UserData for Handle {
	fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
		methods.add_method("send", |lua, this, value: mlua::Value| {
			wait(this.0.write(&json(lua, value)?)).map_err(|e| external(e.to_string()))
		});
		methods.add_method("request", |lua, this, value: mlua::Value| {
			let mut value = json(lua, value)?;
			let id = this.0.next.fetch_add(1, Ordering::SeqCst);
			if !value.is_object() {
				value = json!({ "data": value });
			}
			value["id"] = json!(id);
			let (tx, rx) = oneshot::channel();
			this.0.pending.lock().expect("pending lock").insert(id, tx);
			wait(this.0.write(&value)).map_err(|e| external(e.to_string()))?;
			let reply = wait(tokio::time::timeout(this.0.timeout, rx));
			this.0.pending.lock().expect("pending lock").remove(&id);
			match reply {
				Ok(Ok(reply)) => lua.to_value(&reply),
				Ok(Err(_)) => Err(external("the program exited".into())),
				Err(_) => Err(external("the program did not answer in time".into())),
			}
		});
		methods.add_method("on_line", |_, this, f: Function| {
			*this.0.on_line.lock().expect("on_line lock") = Some(f);
			Ok(())
		});
		methods.add_method("kill", |_, this, ()| {
			if let Some(mut child) = this.0.child.lock().expect("child lock").take() {
				let _ = wait(child.kill());
			}
			Ok(())
		});
	}
}
