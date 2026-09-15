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
use mlua::{
	FromLuaMulti, Function, IntoLuaMulti, Lua, LuaSerdeExt, Table, UserData, UserDataMethods,
};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::oneshot;

use crate::error::{Error, Result};
use crate::transport::cartridge::{self, Ctx, CONNECT_TIMEOUT_ENV, NODE_TOKEN_ENV, SOCKET_ENV};

pub const ENTRY_ENV: &str = "CARTRIDGE_ENTRY";
pub const ROOT_ENV: &str = "CARTRIDGE_ROOT";
pub const LISTEN_ENV: &str = "CARTRIDGE_LISTEN";
pub const ENTRY_SHA256_ENV: &str = "CARTRIDGE_ENTRY_SHA256";

static RUNTIME: std::sync::OnceLock<tokio::runtime::Handle> = std::sync::OnceLock::new();

/// Block on a future for a Lua call that cannot yield: a native module's call
/// into the global, or one from a thread of the module's own. It holds the Lua
/// state until it returns.
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

const ENCODE: &str = "cartridge.encode";

/// One call holds the state for the whole conversion, so a table another
/// coroutine mutates meanwhile still encodes consistently.
fn json(lua: &Lua, value: mlua::Value) -> mlua::Result<Value> {
	let encode: Function = lua.named_registry_value(ENCODE)?;
	let text: String = encode.call(value)?;
	serde_json::from_str(&text).map_err(mlua::Error::external)
}

/// Pick the yielding or the blocking function by whether the caller can yield.
const EITHER: &str = r#"local yielding, blocking, yieldable = ...
return function(...)
	if yieldable() then return yielding(...) end
	return blocking(...)
end"#;

/// A Lua function for an async call. Called from Lua code (a handler, a
/// subscriber, init.lua) it yields, so the node serves other events while it
/// waits. Called where Lua cannot yield (inside a native module's function, or
/// from its own thread) it blocks instead.
fn either<A, R, F, Fut>(lua: &Lua, f: F) -> mlua::Result<Function>
where
	A: FromLuaMulti + 'static,
	R: IntoLuaMulti + 'static,
	F: Fn(Lua, A) -> Fut + Clone + Send + Sync + 'static,
	Fut: std::future::Future<Output = mlua::Result<R>> + Send + 'static,
{
	let blocking = {
		let f = f.clone();
		lua.create_function(move |lua, args: A| wait(f(lua.clone(), args)))?
	};
	let yielding = lua.create_async_function(f)?;
	let yieldable: Function = lua
		.globals()
		.get::<Table>("coroutine")?
		.get("isyieldable")?;
	lua.load(EITHER)
		.set_name("=cartridge")
		.call((yielding, blocking, yieldable))
}

fn text(error: mlua::Error) -> String {
	error.to_string()
}

fn env(name: &str) -> Result<String> {
	std::env::var(name).map_err(|_| {
		Error::Descriptor(format!("{name} must be set; a node is started by the base"))
	})
}

/// Run this process as a node until the base disposes it or goes away.
pub async fn main() -> Result<ExitCode> {
	let socket = PathBuf::from(env(SOCKET_ENV)?);
	// The node's own credential: what its socket grants as `Host`. It names
	// authority over this node only, so the six variables are scrubbed below.
	let host_token = env(NODE_TOKEN_ENV)?;
	let entry = PathBuf::from(env(ENTRY_ENV)?);
	let expected = env(ENTRY_SHA256_ENV)?;
	let root = PathBuf::from(env(ROOT_ENV)?);
	let listen: Vec<String> = serde_json::from_str(&env(LISTEN_ENV)?)?;
	let timeout = std::env::var(CONNECT_TIMEOUT_ENV)
		.ok()
		.and_then(|secs| secs.parse().ok())
		.map(Duration::from_secs)
		.unwrap_or(Duration::from_secs(30));
	let _ = RUNTIME.set(tokio::runtime::Handle::current());
	// The seven variables named this node; no helper or Lua library spawned
	// from here needs them. `ponytail:` env is process-global, so a helper
	// racing a remove_var could still read its own variable — the token is
	// per-node and the socket answers before any helper exists, so the
	// exposure is one node's own credential to itself.
	for key in [
		SOCKET_ENV,
		NODE_TOKEN_ENV,
		CONNECT_TIMEOUT_ENV,
		ENTRY_ENV,
		ROOT_ENV,
		LISTEN_ENV,
		ENTRY_SHA256_ENV,
	] {
		std::env::remove_var(key);
	}
	let listener = cartridge::listen(&socket)
		.await
		.map_err(|e| Error::process(entry.display().to_string(), e))?;
	let ctx = Ctx::new(host_token, timeout);
	let settings = crate::settings::host();
	let lua = crate::lua::interpreter(
		settings.lua_memory_bytes,
		settings.lua_instruction_budget,
		crate::lua::Overrun::Exit,
	)?;
	let socket_dir = socket.parent().map(Path::to_path_buf).unwrap_or_default();
	install(&lua, ctx.clone(), root, listen, socket_dir)?;
	let lifeline = ctx.clone();
	std::thread::spawn(move || {
		let _ = std::io::copy(&mut std::io::stdin(), &mut std::io::sink());
		lifeline.stop();
	});
	let apply =
		Box::new(move |ctx: Ctx, config: Value| apply(lua, ctx, entry, expected, config).boxed());
	cartridge::serve(listener, ctx, apply)
		.await
		.map_err(|e| Error::process(socket.display().to_string(), e))?;
	Ok(ExitCode::SUCCESS)
}

/// The entry's bytes, re-checked against the SHA-256 the base verified when it
/// planned the cartridge: a file edited since is refused, not loaded.
pub(crate) fn entry_bytes(entry: &Path, expected: &str) -> mlua::Result<String> {
	let bytes = std::fs::read(entry).map_err(mlua::Error::external)?;
	if crate::trust::digest_bytes(&bytes) != expected {
		return Err(mlua::Error::external(format!(
			"{} changed since the base verified it; run `cartridge trust`, then `cartridge reload`",
			entry.display()
		)));
	}
	String::from_utf8(bytes).map_err(mlua::Error::external)
}

/// Run `init.lua` with the config the base handed over, as a coroutine, so its
/// top level may send events too.
async fn apply(
	lua: Lua,
	ctx: Ctx,
	entry: PathBuf,
	expected: String,
	config: Value,
) -> cartridge::Result<()> {
	let run = async {
		let global: Table = lua.globals().get("cartridge")?;
		global.set("name", ctx.name())?;
		global.set("config", lua.to_value(&config)?)?;
		let source = entry_bytes(&entry, &expected)?;
		let returned: mlua::Value = lua
			.load(source)
			.set_name(entry.to_string_lossy())
			.eval_async()
			.await?;
		let disposer = match returned {
			mlua::Value::Table(table) => match table.get::<mlua::Value>("apply")? {
				mlua::Value::Function(apply) => {
					let config = global.get::<mlua::Value>("config")?;
					apply
						.call_async::<mlua::Value>((global.clone(), config))
						.await?
				}
				_ => mlua::Value::Nil,
			},
			other => other,
		};
		if let mlua::Value::Function(dispose) = disposer {
			ctx.on_dispose(move || async move {
				if let Err(error) = dispose.call_async::<()>(()).await {
					tracing::warn!(target: "cartridge", "disposer failed: {error}");
				}
			});
		}
		Ok::<(), mlua::Error>(())
	};
	run.await.map_err(text)
}

/// The `cartridge` global: the base's event system, streams, host queries, and
/// the two ways a cartridge brings code of its own.
fn install(
	lua: &Lua,
	ctx: Ctx,
	root: PathBuf,
	listen: Vec<String>,
	socket_dir: PathBuf,
) -> mlua::Result<()> {
	lua.set_named_registry_value(
		ENCODE,
		lua.create_function(|lua, value: mlua::Value| {
			Ok(lua.from_value::<Value>(value)?.to_string())
		})?,
	)?;
	let global = lua.create_table()?;
	global.set("root", root.to_string_lossy().into_owned())?;
	// A native module's own mlua does not know this marker; it tags arrays with it.
	global.set("array_metatable", lua.array_metatable())?;
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
	global.set("listen", {
		let (ctx, lua_handle) = (ctx.clone(), lua.clone());
		lua.create_function(move |_, (name, f): (String, Function)| {
			if !listen.contains(&name) {
				return Err(external(format!("`{name}` is not declared in `listen`")));
			}
			let lua = lua_handle.clone();
			ctx.on(&name, move |data| {
				let (lua, f) = (lua.clone(), f.clone());
				async move {
					let out: mlua::Value = f
						.call_async(lua.to_value(&data).map_err(text)?)
						.await
						.map_err(text)?;
					lua.from_value(out).map_err(text)
				}
			});
			Ok(())
		})?
	})?;
	global.set("on_dispose", {
		let ctx = ctx.clone();
		lua.create_function(move |_, f: Function| {
			ctx.on_dispose(move || async move {
				if let Err(error) = f.call_async::<()>(()).await {
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
			let _runtime = RUNTIME.get().expect("the node runtime").enter();
			ctx.emit(&name, data).map_err(external)
		})?
	})?;
	global.set("notify", {
		let ctx = ctx.clone();
		lua.create_function(move |lua, (name, data): (String, mlua::Value)| {
			let data = json(lua, data)?;
			let _runtime = RUNTIME.get().expect("the node runtime").enter();
			ctx.notify(&name, data).map_err(external)
		})?
	})?;
	global.set("bail", {
		let ctx = ctx.clone();
		either(lua, move |lua: Lua, (name, data): (String, mlua::Value)| {
			let ctx = ctx.clone();
			async move {
				let data = json(&lua, data)?;
				match ctx.bail(&name, data).await.map_err(external)? {
					Some(answer) => lua.to_value(&answer),
					None => Ok(mlua::Value::Nil),
				}
			}
		})?
	})?;
	global.set("gather", {
		let ctx = ctx.clone();
		either(lua, move |lua: Lua, (name, data): (String, mlua::Value)| {
			let ctx = ctx.clone();
			async move {
				let data = json(&lua, data)?;
				let outcomes = ctx.gather(&name, data).await.map_err(external)?;
				lua.to_value(&outcomes)
			}
		})?
	})?;
	global.set("publish", {
		let ctx = ctx.clone();
		lua.create_function(move |lua, (channel, data): (String, mlua::Value)| {
			ctx.publish(&channel, json(lua, data)?);
			Ok(())
		})?
	})?;
	global.set("subscribe", {
		let ctx = ctx.clone();
		either(
			lua,
			move |lua: Lua, (cartridge, channel, f): (String, String, Function)| {
				let ctx = ctx.clone();
				async move {
					ctx.subscribe(&cartridge, &channel, None, move |envelope| {
						let (lua, f) = (lua.clone(), f.clone());
						async move {
							let result = match lua.to_value(&envelope) {
								Ok(envelope) => f.call_async::<()>(envelope).await,
								Err(error) => Err(error),
							};
							if let Err(error) = result {
								tracing::warn!(target: "cartridge", "subscriber failed: {error}");
							}
						}
					})
					.await
					.map_err(external)
				}
			},
		)?
	})?;
	global.set("host", {
		let ctx = ctx.clone();
		either(
			lua,
			move |lua: Lua, (method, params): (String, mlua::Value)| {
				let ctx = ctx.clone();
				async move {
					let params = json(&lua, params)?;
					let out = ctx.host(&method, params).await.map_err(external)?;
					lua.to_value(&out)
				}
			},
		)?
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
	global.set("pipe", {
		let (dir, ctx) = (socket_dir.clone(), ctx.clone());
		let counter = Arc::new(AtomicU64::new(1));
		lua.create_function(move |lua, f: Option<Function>| pipe(lua, &ctx, &dir, &counter, f))?
	})?;
	global.set("spawn", {
		let ctx = ctx.clone();
		lua.create_function(
			move |lua, (command, options): (mlua::Value, Option<Table>)| {
				spawn(lua, &ctx, command, options)
			},
		)?
	})?;
	lua.globals().set("cartridge", global)
}

/// Load `lib<name>.dylib`/`<name>.so` (or `<name>.dll`/`lib<name>.dll` on
/// Windows) from the cartridge folder (or its `target/{release,debug}` while
/// developing) and run its `luaopen_<name>`.
fn load_native(lua: &Lua, root: &Path, name: &str) -> mlua::Result<mlua::Value> {
	let symbol = name.replace(['-', '.'], "_");
	let files: Vec<String> = if cfg!(windows) {
		vec![format!("{symbol}.dll"), format!("lib{symbol}.dll")]
	} else {
		vec![
			format!("lib{symbol}.dylib"),
			format!("{symbol}.dylib"),
			format!("lib{symbol}.so"),
			format!("{symbol}.so"),
		]
	};
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

/// A protocol line from a helper or a pipe, served by the node in Rust without
/// touching Lua, so it works while a Lua handler is busy:
///   {"emit"|"notify"|"publish": name, "data"}
///   {"ask": id, "bail"|"gather"|"host": name, "args"|"params"}  answered as {"ask": id, "result"|"error"}
/// Anything else is not protocol and goes to the Lua handler.
async fn relay(ctx: &Ctx, value: &Value, answer: impl FnOnce(Value) + Send + 'static) -> bool {
	let name = |key: &str| value[key].as_str().map(str::to_owned);
	if let Some(name) = name("emit") {
		if let Err(error) = ctx.emit(&name, value["data"].clone()) {
			tracing::warn!(target: "cartridge", "emit `{name}` refused: {error}");
		}
		return true;
	}
	if let Some(name) = name("notify") {
		if let Err(error) = ctx.notify(&name, value["data"].clone()) {
			tracing::warn!(target: "cartridge", "notify `{name}` refused: {error}");
		}
		return true;
	}
	if let Some(channel) = name("publish") {
		ctx.publish(&channel, value["data"].clone());
		return true;
	}
	let Some(ask) = value.get("ask").cloned() else {
		return false;
	};
	let (ctx, value) = (ctx.clone(), value.clone());
	let asked = if let Some(name) = name("bail") {
		async move {
			ctx.bail(&name, value["args"].clone())
				.await
				.map(|a| a.unwrap_or(Value::Null))
		}
		.boxed()
	} else if let Some(name) = name("gather") {
		async move {
			ctx.gather(&name, value["args"].clone())
				.await
				.map(|rows| json!(rows))
		}
		.boxed()
	} else if let Some(method) = name("host") {
		async move { ctx.host(&method, value["params"].clone()).await }.boxed()
	} else {
		return false;
	};
	// Answered out of line: the asker may have more to say while it waits.
	tokio::spawn(async move {
		answer(match asked.await {
			Ok(result) => json!({ "ask": ask, "result": result }),
			Err(error) => json!({ "ask": ask, "error": error.to_string() }),
		});
	});
	true
}

/// Every line one writer sends, until it closes. Shared by both platforms so
/// the protocol a native module speaks is the same wherever it runs.
async fn pipe_lines(
	reader: impl tokio::io::AsyncRead + Unpin,
	lua: Lua,
	ctx: Ctx,
	f: Option<Function>,
) {
	let mut lines = BufReader::new(reader).lines();
	while let Ok(Some(line)) = lines.next_line().await {
		let value: Value = serde_json::from_str(&line).unwrap_or(Value::String(line));
		let answer = {
			let (lua, f) = (lua.clone(), f.clone());
			move |answer: Value| {
				let Some(f) = f else { return };
				tokio::spawn(async move {
					let run = async { f.call_async::<()>(lua.to_value(&answer)?).await };
					if let Err(error) = run.await {
						tracing::warn!(target: "cartridge", "pipe handler failed: {error}");
					}
				});
			}
		};
		if relay(&ctx, &value, answer).await {
			continue;
		}
		let Some(f) = &f else { continue };
		let run = async { f.call_async::<()>(lua.to_value(&value)?).await };
		if let Err(error) = run.await {
			tracing::warn!(target: "cartridge", "pipe handler failed: {error}");
		}
	}
}

/// A pipe whose lines a native module's own threads may write from: they reach
/// the node in Rust, so the Lua state is never entered from those threads.
/// Protocol lines are served by the node itself; anything else goes to `f`.
///
/// A FIFO on Unix and a named pipe on Windows. Both are opened by a writer with
/// nothing but the name this returns, and both outlive any one writer: the node
/// holds the FIFO open itself so a reader never sees EOF between writers, and
/// on Windows takes the next connection when one ends.
#[cfg(unix)]
fn pipe(
	lua: &Lua,
	ctx: &Ctx,
	dir: &Path,
	counter: &AtomicU64,
	f: Option<Function>,
) -> mlua::Result<String> {
	let path = dir.join(format!(
		"{}.{}.fifo",
		std::process::id(),
		counter.fetch_add(1, Ordering::SeqCst)
	));
	let c_path = std::ffi::CString::new(path.to_string_lossy().into_owned())
		.map_err(|e| external(e.to_string()))?;
	// SAFETY: a NUL-terminated path; mkfifo touches nothing else.
	if unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) } != 0 {
		return Err(external(format!(
			"{}: {}",
			path.display(),
			std::io::Error::last_os_error()
		)));
	}
	// A clean dispose unlinks the FIFO, so a reload doesn't leave it behind;
	// the base's monitor still has to sweep the crash path beside the socket.
	{
		let unlink = path.clone();
		ctx.on_dispose(move || async move {
			let _ = std::fs::remove_file(&unlink);
		});
	}
	// Held open by the node itself before init.lua goes on, so a writer's open
	// never waits for a reader and readers never see EOF between writers.
	let keep = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.open(&path)
		.map_err(|e| external(format!("{}: {e}", path.display())))?;
	let (lua, ctx) = (lua.clone(), ctx.clone());
	let reader = path.clone();
	tokio::spawn(async move {
		let Ok(file) = tokio::fs::File::open(&reader).await else {
			return;
		};
		pipe_lines(file, lua, ctx, f).await;
		drop(keep);
	});
	Ok(path.to_string_lossy().into_owned())
}

#[cfg(windows)]
fn pipe(
	lua: &Lua,
	ctx: &Ctx,
	dir: &Path,
	counter: &AtomicU64,
	f: Option<Function>,
) -> mlua::Result<String> {
	use tokio::net::windows::named_pipe::ServerOptions;
	// The directory names the node's own sockets; a pipe is not a file, so it
	// borrows the directory's identity for its name rather than living in it.
	let name = format!(
		r"\\.\pipe\cartridge-{}-{}-{}",
		crate::transport::typed::path_tag(dir),
		std::process::id(),
		counter.fetch_add(1, Ordering::SeqCst)
	);
	// The first instance is made here, before init.lua goes on, so a writer
	// that opens the name immediately finds it already there.
	let mut server = ServerOptions::new()
		.first_pipe_instance(true)
		.create(&name)
		.map_err(|e| external(format!("{name}: {e}")))?;
	let (lua, ctx) = (lua.clone(), ctx.clone());
	let listening = name.clone();
	tokio::spawn(async move {
		loop {
			if server.connect().await.is_err() {
				return;
			}
			// The next instance is made before this one is served, so a writer
			// arriving while another is mid-line still finds the name open.
			let next = match ServerOptions::new().create(&listening) {
				Ok(next) => next,
				Err(error) => {
					tracing::warn!(target: "cartridge", "pipe {listening}: {error}");
					return;
				}
			};
			pipe_lines(
				std::mem::replace(&mut server, next),
				lua.clone(),
				ctx.clone(),
				f.clone(),
			)
			.await;
		}
	});
	Ok(name)
}

/// A helper program: lines of JSON in and out. `request` matches a reply by `id`;
/// every other line reaches the `on_line` handler.
struct Spawned {
	/// `None` once a write timed out: a half-written line nobody can finish,
	/// so the handle is gone the way the killed `child` is.
	stdin: tokio::sync::Mutex<Option<tokio::process::ChildStdin>>,
	pending: Mutex<HashMap<u64, oneshot::Sender<Value>>>,
	on_line: Mutex<Option<Function>>,
	next: AtomicU64,
	timeout: Duration,
	child: Mutex<Option<tokio::process::Child>>,
}

fn spawn(
	lua: &Lua,
	ctx: &Ctx,
	command: mlua::Value,
	options: Option<Table>,
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
	// A node's own wiring is not its helpers'. The socket it serves, the
	// credential it was minted, the entry it ran, the pipe instances it was
	// handed — a helper needs none of them, and one that reads them is holding
	// the node's credential. The base already withholds its own from the node;
	// this is the same rule one step down.
	for (key, _) in std::env::vars_os() {
		if key.to_string_lossy().starts_with("CARTRIDGE_") {
			process.env_remove(key);
		}
	}
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
		stdin: tokio::sync::Mutex::new(Some(stdin)),
		pending: Mutex::default(),
		on_line: Mutex::default(),
		next: AtomicU64::new(1),
		timeout,
		child: Mutex::new(Some(child)),
	});
	let (reader, reader_lua, ctx) = (spawned.clone(), lua.clone(), ctx.clone());
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
			let back = reader.clone();
			if relay(&ctx, &value, move |answer| {
				tokio::spawn(async move {
					let _ = back.write(&answer).await;
				});
			})
			.await
			{
				continue;
			}
			let handler = reader.on_line.lock().expect("on_line lock").clone();
			if let Some(handler) = handler {
				let result = match lua.to_value(&value) {
					Ok(value) => handler.call_async::<()>(value).await,
					Err(error) => Err(error),
				};
				if let Err(error) = result {
					tracing::warn!(target: "cartridge", "line handler failed: {error}");
				}
			}
		}
		// The helper exited without answering these; drop their senders so
		// every waiting `rx` resolves to `RecvError` instead of hanging for
		// the full timeout.
		reader.pending.lock().expect("pending lock").clear();
	});
	lua.create_userdata(Handle(spawned))
}

struct Handle(Arc<Spawned>);

impl Spawned {
	async fn write(&self, value: &Value) -> std::io::Result<()> {
		let mut line = value.to_string();
		line.push('\n');
		// Bounded, so a helper that stops reading its stdin cannot park every
		// later send behind the filled pipe forever. The lock is taken outside
		// the timed block and held across it: on timeout the write is dropped
		// mid-line, and no writer parked behind may resume that line, so the
		// handle is taken away and the helper killed before the lock is let go.
		let mut held = self.stdin.lock().await;
		let Some(stdin) = held.as_mut() else {
			return Err(std::io::Error::new(
				std::io::ErrorKind::BrokenPipe,
				"the program's input is closed",
			));
		};
		let written = tokio::time::timeout(self.timeout, async {
			stdin.write_all(line.as_bytes()).await?;
			stdin.flush().await
		})
		.await;
		match written {
			Ok(written) => written,
			Err(_) => {
				held.take();
				drop(held);
				if let Some(mut child) = self.child.lock().expect("child lock").take() {
					let _ = child.start_kill();
				}
				Err(std::io::Error::new(
					std::io::ErrorKind::TimedOut,
					"the program did not read its input in time",
				))
			}
		}
	}
}

impl UserData for Handle {
	fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
		methods.add_async_method("send", |lua, this, value: mlua::Value| {
			let spawned = this.0.clone();
			async move {
				let value = json(&lua, value)?;
				spawned
					.write(&value)
					.await
					.map_err(|e| external(e.to_string()))
			}
		});
		methods.add_async_method("request", |lua, this, value: mlua::Value| {
			let spawned = this.0.clone();
			async move {
				let mut value = json(&lua, value)?;
				let id = spawned.next.fetch_add(1, Ordering::SeqCst);
				if !value.is_object() {
					value = json!({ "data": value });
				}
				value["id"] = json!(id);
				let (tx, rx) = oneshot::channel();
				spawned.pending.lock().expect("pending lock").insert(id, tx);
				// The write is bounded by `timeout` too, so a stalled helper
				// can make one `:request` wait twice it: once writing, once
				// for the reply.
				let written = spawned.write(&value).await;
				let reply = match &written {
					Ok(()) => Some(tokio::time::timeout(spawned.timeout, rx).await),
					Err(_) => None,
				};
				spawned.pending.lock().expect("pending lock").remove(&id);
				match reply {
					// The write's own error says what went wrong: a stalled
					// helper did not answer, it did not close its input.
					None => Err(external(match written.unwrap_err() {
						e if e.kind() == std::io::ErrorKind::TimedOut => {
							"the program did not answer in time".to_string()
						}
						e if e.kind() == std::io::ErrorKind::BrokenPipe => {
							"the program's input is closed".to_string()
						}
						e => e.to_string(),
					})),
					Some(Ok(Ok(reply))) => lua.to_value(&reply),
					Some(Ok(Err(_))) => Err(external("the program exited".into())),
					Some(Err(_)) => Err(external("the program did not answer in time".into())),
				}
			}
		});
		methods.add_method("on_line", |_, this, f: Function| {
			*this.0.on_line.lock().expect("on_line lock") = Some(f);
			Ok(())
		});
		methods.add_method("kill", |_, this, ()| {
			if let Some(mut child) = this.0.child.lock().expect("child lock").take() {
				let _ = child.start_kill();
			}
			Ok(())
		});
	}
}
