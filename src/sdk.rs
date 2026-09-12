//! The cartridge side of zirkle's process wire (see [`crate::cartridge`]). A cartridge
//! declares what it injects and provides, then runs an `apply` that registers
//! listeners and services on the [`Host`]; when `apply` returns the fiber is
//! active. `argv[1] == "hello"` prints the declaration and exits.
//!
//! The wire is symmetric enough to nest: [`Host::spawn`] speaks the host half
//! of the same protocol to cartridges of the cartridge's own, so a cartridge
//! that hosts cartridges is itself a sub-host and the binary above it cannot
//! tell that it nested. A child key reaches the top only through the keys the
//! sub-host's own declaration already carries — the daemon never learns of a
//! generation below the one it applied. A reload the daemon asks of the
//! sub-host is carried down to a child whose `hello` declared reload, and a
//! child's bridge call is forwarded up, where the profile grant lives.

use crate::cartridge::Link;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

pub type Result<T> = std::result::Result<T, String>;
type Fut<T> = Pin<Box<dyn Future<Output = T> + Send>>;
type Handler = Arc<dyn Fn(Value) -> Fut<Result<Value>> + Send + Sync>;
type Finalizer = Box<dyn FnOnce() -> Fut<()> + Send>;

/// A cartridge hosted below this one, and what its `hello` declared: the
/// reload flag is what makes a frame the host above sends this cartridge the
/// child's to answer.
struct Child {
	link: Arc<Link>,
	reload: bool,
}

#[derive(Clone)]
pub struct Host {
	reload: Arc<Mutex<HashMap<String, Handler>>>,
	link: Arc<Link>,
	events: Arc<Mutex<HashMap<String, Handler>>>,
	services: Arc<Mutex<HashMap<String, Handler>>>,
	finalizers: Arc<Mutex<Vec<Finalizer>>>,
	/// The provide keys this cartridge's own declaration carries — the only
	/// keys a child's may join, so an undeclared child key stays private.
	declared: Vec<String>,
	children: Arc<Mutex<Vec<Child>>>,
}

fn boxed<F, Fut>(f: F) -> Handler
where
	F: Fn(Value) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Result<Value>> + Send + 'static,
{
	Arc::new(move |v| Box::pin(f(v)))
}

impl Host {
	/// Client modules of active cartridge generations, via the bridge.
	pub async fn bridge_status(&self) -> Result<Value> {
		self.link.request(json!({"bridge":"status"})).await
	}

	/// A nested cartridge's bridge call, forwarded to the host above: the
	/// sub-host holds no bridge of its own, so the frame is answered where it
	/// already is for a first-generation cartridge — the profile grant and the
	/// owner checks live in the daemon.
	pub async fn bridge_call(&self, call: Value) -> Result<Value> {
		self.link.request(call).await
	}

	/// Enabled cartridges as `{"id","dir"}` rows; `dir` is the cartridge folder.
	pub async fn cartridges(&self) -> Result<Value> {
		self.link.request(json!({"cartridges":true})).await
	}

	/// The keys the profile injects into this cartridge, in declaration order.
	/// A cartridge composed with a glob (`inject = {"tool.*"}`) reads its own
	/// effective surface here rather than being told it twice.
	pub async fn injections(&self) -> Result<Vec<String>> {
		let value = self.link.request(json!({"injections":true})).await?;
		serde_json::from_value(value).map_err(|e| format!("injections: {e}"))
	}

	pub async fn request_reload(&self) -> Result<()> {
		self.link.request(json!({"reload":true})).await.map(|_| ())
	}

	/// Explicitly resume a service call after a generation switch. Ordinary
	/// replies and errors are never replayed by the core.
	pub fn resume(args: Value) -> Value {
		json!({"$zirkle_resume":args})
	}
	fn write(&self, m: Value) {
		self.link.send(m);
	}

	/// Only `on` listeners in the daemon: Lua fibers and other cartridges,
	/// including ones running in their own process. Socket clients see nothing.
	pub fn emit(&self, name: &str, data: Value) {
		self.write(json!({ "emit": name, "data": data }));
	}

	/// Only socket clients. `on` listeners, Lua or cartridge, see nothing.
	pub fn send(&self, name: &str, data: Value) {
		self.write(json!({ "send": name, "data": data }));
	}

	/// Both audiences, and the default for an event a cartridge publishes: the
	/// publisher cannot know whether a consumer listens via `on` or off the
	/// socket, so picking one silently drops the other half.
	pub fn notify(&self, name: &str, data: Value) {
		self.emit(name, data.clone());
		self.send(name, data);
	}

	/// Call a key this cartridge injects.
	pub async fn call(&self, key: &str, args: Value) -> Result<Value> {
		self
			.link
			.request(json!({ "call": key, "args": args }))
			.await
	}

	pub async fn meta(&self, key: &str) -> Result<Value> {
		self.link.request(json!({ "meta": key })).await
	}

	pub fn on<F, Fut>(&self, name: &str, f: F)
	where
		F: Fn(Value) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<Value>> + Send + 'static,
	{
		self.events.lock().insert(name.into(), boxed(f));
		self.write(json!({ "on": name }));
	}

	/// Serve a key this cartridge declared in `provide`.
	pub fn provide<F, Fut>(&self, key: &str, f: F)
	where
		F: Fn(Value) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<Value>> + Send + 'static,
	{
		self.services.lock().insert(key.into(), boxed(f));
		self.write(json!({ "provide": key }));
	}

	/// Runs on dispose, last registered first.
	pub fn on_dispose<F, Fut>(&self, f: F)
	where
		F: FnOnce() -> Fut + Send + 'static,
		Fut: Future<Output = ()> + Send + 'static,
	{
		self.finalizers.lock().push(Box::new(move || Box::pin(f())));
	}

	/// Spawn a cartridge of this cartridge's own and host it over the same wire
	/// this cartridge is hosted on. Every frame the child sends is answered
	/// here — a `provide` is served by forwarding the call down, a `call` of an
	/// injected key is answered by calling up — so the child sees a host and
	/// the host above sees nothing but this cartridge's own declaration.
	/// Disposing this cartridge disposes the child.
	pub async fn spawn(&self, name: &str, cmd: &[String], config: Value) -> Result<()> {
		let program = cmd
			.first()
			.ok_or_else(|| "empty cmd".to_owned())?
			.clone();
		let manifest = crate::cartridge::manifest(cmd)
			.map_err(|e| format!("{program} hello: {e}"))?;
		let mut child = tokio::process::Command::new(&cmd[0])
			.args(&cmd[1..])
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.kill_on_drop(true)
			.spawn()
			.map_err(|e| format!("{program}: {e}"))?;
		let mut stdin = child.stdin.take().expect("piped stdin");
		let stdout = child.stdout.take().expect("piped stdout");
		let stderr = child.stderr.take().expect("piped stderr");
		let label = name.to_owned();
		tokio::spawn(async move {
			let mut lines = BufReader::new(stderr).lines();
			while let Ok(Some(line)) = lines.next_line().await {
				crate::turn::diagnostic_line(&label, &line);
			}
		});
		let (tx, mut rx) = mpsc::unbounded_channel::<Option<Value>>();
		let link = Link::new(tx, "child is gone");
		let writer_link = link.clone();
		tokio::spawn(async move {
			while let Some(Some(m)) = rx.recv().await {
				let mut line = m.to_string();
				line.push('\n');
				if stdin.write_all(line.as_bytes()).await.is_err() {
					writer_link.close();
					break;
				}
			}
		});
		link.send(json!({ "apply": { "name": name, "config": config } }));
		let mut lines = BufReader::new(stdout).lines();
		loop {
			let Ok(Some(line)) = lines.next_line().await else {
				link.shutdown();
				let status = child
					.try_wait()
					.ok()
					.flatten()
					.map(|s| s.to_string())
					.unwrap_or_else(|| "closed stdout".into());
				return Err(format!("{name} exited before ready: {status}"));
			};
			let Ok(m) = serde_json::from_str::<Value>(&line) else {
				link.shutdown();
				return Err(format!("{name} sent an unreadable line: {line}"));
			};
			if m["ready"] == true {
				break;
			}
			if let Some(e) = m["error"]
				.as_str()
				.filter(|_| m["reply"].as_u64().is_none())
			{
				link.shutdown();
				return Err(format!("{name}: {e}"));
			}
			self.relay(&link, m).await;
		}
		let stopping = Arc::new(AtomicBool::new(false));
		let host = self.clone();
		let reader_link = link.clone();
		let reader_name = name.to_owned();
		let reader_stopping = stopping.clone();
		tokio::spawn(async move {
			while let Ok(Some(line)) = lines.next_line().await {
				if let Ok(m) = serde_json::from_str::<Value>(&line) {
					if let Some(e) = m["error"]
						.as_str()
						.filter(|_| m["reply"].as_u64().is_none())
					{
						host.write(json!({ "error": format!("{reader_name}: {e}") }));
						continue;
					}
					host.relay(&reader_link, m).await;
				}
			}
			reader_link.shutdown();
			host
				.children
				.lock()
				.retain(|child| !Arc::ptr_eq(&child.link, &reader_link));
			// A child that dies while it is still wanted is a fault of the
			// sub-host's own apply, reported above it; a death during disposal
			// is the disposal working.
			if !reader_stopping.load(Ordering::SeqCst) {
				host.write(json!({ "error": format!("{reader_name}: exited") }));
			}
		});
		let (finalizer_link, mut finalizer_child) = (link.clone(), child);
		self.children.lock().push(Child {
			link: finalizer_link.clone(),
			reload: manifest.reload,
		});
		let children = self.children.clone();
		let finalizer_stopping = stopping.clone();
		self.on_dispose(move || async move {
			finalizer_stopping.store(true, Ordering::SeqCst);
			children
				.lock()
				.retain(|child| !Arc::ptr_eq(&child.link, &finalizer_link));
			finalizer_link.send(json!({ "dispose": true }));
			finalizer_link.stop();
			if tokio::time::timeout(std::time::Duration::from_secs(5), finalizer_child.wait())
				.await
				.is_err()
			{
				let _ = finalizer_child.kill().await;
			}
		});
		Ok(())
	}

	/// Serve one frame from a cartridge of this cartridge's own — the host half
	/// of the wire, the inverse of this cartridge's own run loop. Frames that
	/// carry a request upward (`call`, `meta`, the bridge queries) are spawned,
	/// so a slow answer above never stalls the child's next frame.
	async fn relay(&self, link: &Arc<Link>, m: Value) {
		if let Some(key) = m["provide"].as_str().map(str::to_owned) {
			let child = link.clone();
			let served = key.clone();
			self.services.lock().insert(
				key.clone(),
				boxed(move |args| {
					let (child, key) = (child.clone(), served.clone());
					async move { child.request(json!({ "call": key, "args": args })).await }
				}),
			);
			// The key is served from below, but the host above only knows the
			// keys this cartridge's declaration already carries: a declared
			// child key is re-advertised, an undeclared one stays a private
			// service of the sub-host.
			if self.declared.iter().any(|declared| *declared == key) {
				self.write(json!({ "provide": key.clone() }));
			}
			return;
		}
		if let Some(name) = m["on"].as_str().map(str::to_owned) {
			let child = link.clone();
			let registered = name.clone();
			self.on(&registered, move |data| {
				let (child, name) = (child.clone(), name.clone());
				async move { child.request(json!({ "event": name, "data": data })).await }
			});
			return;
		}
		if let Some(name) = m["emit"].as_str() {
			self.emit(name, m["data"].clone());
			return;
		}
		if let Some(name) = m["send"].as_str() {
			self.send(name, m["data"].clone());
			return;
		}
		if link.accept(&m) {
			return;
		}
		let id = m["id"].as_u64().unwrap_or(0);
		let (host, link, turn) = (self.clone(), link.clone(), crate::turn::of(&m));
		if let Some(key) = m["meta"].as_str().map(str::to_owned) {
			tokio::spawn(crate::turn::scope(turn, async move {
				link.reply(id, host.meta(&key).await);
			}));
		} else if let Some(key) = m["call"].as_str().map(str::to_owned) {
			let args = m["args"].clone();
			tokio::spawn(crate::turn::scope(turn, async move {
				link.reply(id, host.call(&key, args).await);
			}));
		} else if m["injections"] == true {
			tokio::spawn(crate::turn::scope(turn, async move {
				link.reply(
					id,
					host.injections()
						.await
						.map(|keys| json!(keys))
						.map_err(|e| e),
				);
			}));
		} else if m["cartridges"] == true {
			tokio::spawn(crate::turn::scope(turn, async move {
				link.reply(id, host.cartridges().await);
			}));
		} else if m["bridge"] == "status" {
			tokio::spawn(crate::turn::scope(turn, async move {
				link.reply(id, host.bridge_status().await);
			}));
		} else if m["bridge"] == "call" {
			let call = m.clone();
			tokio::spawn(crate::turn::scope(turn, async move {
				link.reply(id, host.bridge_call(call).await);
			}));
		} else if m["reload"].is_boolean() {
			let reload = m.clone();
			tokio::spawn(async move {
				if host.reload.lock().contains_key("reload") {
					host.dispatch(&host.reload, &reload, "reload", reload["reload"].clone());
				} else {
					link.reply(id, Ok(Value::Null));
				}
			});
		} else if m["id"].is_u64() {
			link.reply(id, Err(format!("unknown message {m}")));
		}
	}

	fn dispatch(&self, table: &Mutex<HashMap<String, Handler>>, m: &Value, name: &str, data: Value) {
		let id = m["id"].as_u64().unwrap_or(0);
		let handler = table.lock().get(name).cloned();
		let link = self.link.clone();
		let name = name.to_owned();
		// The host's turn continues inside this cartridge, so a nested call or a
		// diagnostic written here names the turn that asked for the work.
		tokio::spawn(crate::turn::scope(crate::turn::of(m), async move {
			let reply = match handler {
				Some(h) => h(data).await,
				None => Err(format!("no handler for {name}")),
			};
			link.reply(id, reply);
		}));
	}

	/// The turn this handler runs in, for a cartridge writing a diagnostic.
	pub fn turn() -> Option<String> {
		crate::turn::current().map(|id| id.to_string())
	}

	async fn finish(&self) {
		self.link.close();
		let finalizers = std::mem::take(&mut *self.finalizers.lock());
		for f in finalizers.into_iter().rev() {
			f().await;
		}
		self.link.stop();
	}
}

#[derive(Default)]
pub struct Cartridge {
	inject: Vec<String>,
	provide: Vec<String>,
}

impl Cartridge {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn inject(mut self, keys: &[&str]) -> Self {
		self.inject.extend(keys.iter().map(|k| k.to_string()));
		self
	}

	pub fn provide(mut self, keys: &[&str]) -> Self {
		self.provide.extend(keys.iter().map(|k| k.to_string()));
		self
	}

	/// Serve until the host disposes this cartridge or closes its stdin.
	pub async fn run<F, Fut>(self, apply: F)
	where
		F: FnOnce(Host, Value) -> Fut + Send + 'static,
		Fut: Future<Output = Result<()>> + Send + 'static,
	{
		if std::env::args().nth(1).as_deref() == Some("hello") {
			println!(
				"{}",
				json!({ "inject": self.inject, "provide": self.provide, "reload": true })
			);
			return;
		}
		let (tx, mut rx) = mpsc::unbounded_channel::<Option<Value>>();
		let link = Link::new(tx, "host is gone");
		let writer_link = link.clone();
		let writer = tokio::spawn(async move {
			let mut out = tokio::io::stdout();
			while let Some(Some(m)) = rx.recv().await {
				let mut line = m.to_string();
				line.push('\n');
				if out.write_all(line.as_bytes()).await.is_err() {
					writer_link.close();
					break;
				}
				if out.flush().await.is_err() {
					writer_link.close();
					break;
				}
			}
		});
		let host = Host {
			reload: Arc::default(),
			link,
			events: Arc::default(),
			services: Arc::default(),
			finalizers: Arc::default(),
			declared: self.provide.clone(),
			children: Arc::default(),
		};
		let mut apply = Some(apply);
		let mut lines = BufReader::new(tokio::io::stdin()).lines();
		while let Ok(Some(line)) = lines.next_line().await {
			let Ok(m) = serde_json::from_str::<Value>(&line) else {
				continue;
			};
			if host.link.accept(&m) {
				continue;
			}
			let id = m["id"].as_u64().unwrap_or(0);
			if m["apply"].is_object() {
				let Some(apply) = apply.take() else { continue };
				let host = host.clone();
				let config = m["apply"]["config"].clone();
				tokio::spawn(async move {
					match apply(host.clone(), config).await {
						Ok(()) => host.write(json!({ "ready": true })),
						Err(e) => {
							host.write(json!({ "error": e }));
							host.finish().await;
						}
					}
				});
			} else if m["reload"].is_boolean() {
				if host.reload.lock().contains_key("reload") {
					host.dispatch(&host.reload, &m, "reload", m["reload"].clone());
				} else if let Some(child) = host
					.children
					.lock()
					.iter()
					.find(|child| child.reload)
					.map(|child| child.link.clone())
				{
					// The child's `hello` declared reload: the frame the host
					// above sent is the child's to answer, and its reply is
					// what this cartridge answers above with.
					let frame = json!({ "reload": m["reload"].clone() });
					let up = host.link.clone();
					tokio::spawn(async move {
						up.reply(id, child.request(frame).await);
					});
				} else {
					host.link.reply(id, Ok(Value::Null));
				}
			} else if let Some(name) = m["event"].as_str() {
				host.dispatch(&host.events, &m, name, m["data"].clone());
			} else if let Some(key) = m["call"].as_str() {
				host.dispatch(&host.services, &m, key, m["args"].clone());
			} else if m["dispose"] == true {
				break;
			}
		}
		host.finish().await;
		let _ = writer.await;
	}
}
