//! The cartridge side of zirkle's process wire (see [`crate::cartridge`]). A cartridge
//! declares what it injects and provides, then runs an `apply` that registers
//! listeners and services on the [`Host`]; when `apply` returns the fiber is
//! active. `argv[1] == "hello"` prints the declaration and exits.

use crate::cartridge::Link;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

pub type Result<T> = std::result::Result<T, String>;
type Fut<T> = Pin<Box<dyn Future<Output = T> + Send>>;
type Handler = Arc<dyn Fn(Value) -> Fut<Result<Value>> + Send + Sync>;
type Finalizer = Box<dyn FnOnce() -> Fut<()> + Send>;

#[derive(Clone)]
pub struct Host {
	reload: Arc<Mutex<HashMap<String, Handler>>>,
	link: Arc<Link>,
	events: Arc<Mutex<HashMap<String, Handler>>>,
	services: Arc<Mutex<HashMap<String, Handler>>>,
	finalizers: Arc<Mutex<Vec<Finalizer>>>,
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

	/// Cooperatively yield long-running calls before this cartridge is replaced.
	pub fn on_reload<F, Fut>(&self, f: F)
	where
		F: Fn(Value) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<Value>> + Send + 'static,
	{
		self.reload.lock().insert("reload".into(), boxed(f));
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
