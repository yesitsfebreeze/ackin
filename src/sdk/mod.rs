//! The cartridge side of cartridge's process wire (see [`crate::cartridge`]). A cartridge
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
//!
//! [`Host`] is the cartridge's handle on the host: its [`api`] registers
//! services, listeners and stream watchers and asks the host questions;
//! [`nested`] spawns cartridges of its own under the same wire. [`Cartridge`]
//! declares and runs the loop that serves every frame the host sends.

mod api;
mod nested;

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
/// A stream subscription's handler, fed the envelope of every event the
/// cartridge receives on its channel, in channel order.
type Watcher = mpsc::Sender<Value>;
/// A watcher's queue holds a full replay of the channel plus the headroom a
/// join, a gap and a lag notice need — the same answer a host-side subscriber
/// gets, off the same settings, so neither side is the narrow one.
pub(super) fn watcher_events() -> usize {
	crate::settings::host().subscriber_events()
}

/// A cartridge hosted below this one, and what its `hello` declared: the
/// reload flag is what makes a frame the host above sends this cartridge the
/// child's to answer.
struct Child {
	link: Arc<Link>,
	reload: bool,
}

#[derive(Clone)]
pub struct Host {
	version_queries: bool,
	reload: Arc<Mutex<HashMap<String, Handler>>>,
	link: Arc<Link>,
	events: Arc<Mutex<HashMap<String, Handler>>>,
	services: Arc<Mutex<HashMap<String, Handler>>>,
	/// What this cartridge contributes to the graph beyond the tools it
	/// provides, set by [`Host::announce`].
	announce: Arc<Mutex<Option<Handler>>>,
	streams: Arc<Mutex<HashMap<String, Watcher>>>,
	finalizers: Arc<Mutex<Vec<Finalizer>>>,
	/// The provide keys this cartridge's own declaration carries — the only
	/// keys a child's may join, so an undeclared child key stays private.
	declared: Vec<String>,
	children: Arc<Mutex<Vec<Child>>>,
}

pub(super) fn boxed<F, Fut>(f: F) -> Handler
where
	F: Fn(Value) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Result<Value>> + Send + 'static,
{
	Arc::new(move |v| Box::pin(f(v)))
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
		// A cartridge process logs like the host does: stderr at the level
		// `CARTRIDGE_LOG` selects, which the host reads as diagnostic lines.
		crate::trace::subscribe();
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
		let mut host = Host {
			version_queries: false,
			reload: Arc::default(),
			link,
			events: Arc::default(),
			services: Arc::default(),
			announce: Arc::default(),
			streams: Arc::default(),
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
				host.version_queries = m["apply"]["capabilities"]["service_versions"] == true;
				let host = host.clone();
				let config = m["apply"]["config"].clone();
				tokio::spawn(async move {
					match apply(host.clone(), config).await {
						Ok(()) => {
							// Registered once the cartridge has applied, so the
							// first announce it can be asked already sees every
							// tool it provides.
							host.listen_announce();
							host.write(json!({ "ready": true }));
						}
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
					host.link.reply(id, Ok::<_, String>(Value::Null));
				}
			} else if let Some(name) = m["event"].as_str() {
				host.dispatch(&host.events, &m, name, m["data"].clone());
			} else if let Some(key) = m["call"].as_str() {
				host.dispatch(&host.services, &m, key, m["args"].clone());
			} else if let Some(channel) = m["channel"].as_str() {
				// Stream delivery: queued onto the channel's pump, which feeds the
				// watcher in arrival order — an event published before another is
				// handled before it, on this channel.
				host.deliver_stream(channel, m["event"].clone());
			} else if m["dispose"] == true {
				break;
			}
		}
		host.finish().await;
		let _ = writer.await;
	}
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/sdk/tests.rs"]
mod tests;
