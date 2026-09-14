//! What a cartridge asks of its host and registers with it: services,
//! listeners, stream watchers, events, and the questions the wire answers.

use std::future::Future;

use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::mpsc;

use super::{boxed, watcher_events, Handler, Host, Result};

impl Host {
	/// Read-only committed composition, without configuration values.
	pub async fn snapshot(&self) -> Result<Value> {
		Ok(self.link.request(json!({"snapshot":true})).await?)
	}

	/// The graph over the live composition, as `{"nodes","edges","announced"}`.
	/// Asking for it fires the announce, so what comes back is what the
	/// composition says about itself now — not a cached copy of what it said
	/// when something was last loaded. `scope` reaches every contributor; pass
	/// the asking side's context in it, such as the session cwd a record is
	/// read from.
	pub async fn graph(&self, scope: Value) -> Result<Value> {
		Ok(self.link.request(json!({"graph": scope})).await?)
	}

	/// Contribute to the graph. The handler is given the announce payload and
	/// answers `{"nodes":[…],"edges":[…]}`, or `Value::Null` to contribute
	/// nothing this time. Every cartridge already announces the tools it
	/// provides; this is for what only the cartridge itself knows — a record's
	/// memos, a session roster, whatever it holds that the graph should reach.
	pub fn announce<F, Fut>(&self, f: F)
	where
		F: Fn(Value) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<Value>> + Send + 'static,
	{
		*self.announce.lock() = Some(boxed(f));
	}

	/// Answer the announce: the tools this cartridge provides, as each one
	/// describes itself right now, plus whatever its own hook contributes.
	///
	/// The descriptors are read from this cartridge's own service handlers, in
	/// process — nobody has to inject a tool to learn what it is, and a tool
	/// that fails or answers nonsense is left out rather than failing the
	/// graph. A cartridge with no tools and no hook answers null and is not
	/// counted among the contributors.
	pub(super) async fn announcement(&self, scope: Value) -> Result<Value> {
		let tools: Vec<(String, Handler)> = {
			let services = self.services.lock();
			services
				.iter()
				.filter(|(key, _)| key.starts_with("tool."))
				.map(|(key, handler)| (key.clone(), handler.clone()))
				.collect()
		};
		let mut nodes = Vec::new();
		for (key, handler) in tools {
			let Ok(answer) = handler(json!({"op":"describe"})).await else {
				continue;
			};
			// A tool may answer describe inside its own result envelope.
			let descriptor = match answer["content"].as_str() {
				Some(text) => serde_json::from_str(text).unwrap_or(Value::Null),
				None => answer,
			};
			let Some(name) = descriptor["name"].as_str() else {
				continue;
			};
			nodes.push(json!({"kind":"tool","key":key,"name":name,
			                  "description":descriptor["description"].as_str().unwrap_or("")}));
		}
		let hook = self.announce.lock().clone();
		let mut contribution = match hook {
			Some(hook) => hook(scope).await?,
			None => Value::Null,
		};
		if nodes.is_empty() && contribution.is_null() {
			return Ok(Value::Null);
		}
		if !contribution.is_object() {
			contribution = json!({});
		}
		let rows = contribution["nodes"]
			.as_array()
			.cloned()
			.unwrap_or_default();
		contribution["nodes"] = json!(nodes.into_iter().chain(rows).collect::<Vec<_>>());
		Ok(contribution)
	}

	/// Listen for the announce. Every cartridge does this, whether or not it
	/// has anything to say, because what it has to say can change after it
	/// applies — a tool added by a later generation announces itself without
	/// the cartridge having to remember to register anything.
	pub(super) fn listen_announce(&self) {
		let host = self.clone();
		self.on(crate::fabric::ANNOUNCE, move |announce| {
			let host = host.clone();
			async move { host.announcement(announce).await }
		});
	}

	/// Cooperatively prepare or cancel a generation replacement.
	pub fn on_reload<F, Fut>(&self, f: F)
	where
		F: Fn(Value) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<Value>> + Send + 'static,
	{
		self.reload.lock().insert("reload".into(), boxed(f));
	}

	/// Client modules of active cartridge generations, via the bridge.
	pub async fn bridge_status(&self) -> Result<Value> {
		Ok(self.link.request(json!({"bridge":"status"})).await?)
	}

	/// A nested cartridge's bridge call, forwarded to the host above: the
	/// sub-host holds no bridge of its own, so the frame is answered where it
	/// already is for a first-generation cartridge — the profile grant and the
	/// owner checks live in the daemon.
	pub async fn bridge_call(&self, call: Value) -> Result<Value> {
		Ok(self.link.request(call).await?)
	}

	/// Enabled cartridges as `{"id","dir"}` rows; `dir` is the cartridge folder.
	pub async fn cartridges(&self) -> Result<Value> {
		Ok(self.link.request(json!({"cartridges":true})).await?)
	}

	/// The keys the profile injects into this cartridge, in declaration order.
	/// A cartridge composed with a glob (`inject = {"tool.*"}`) reads its own
	/// effective surface here rather than being told it twice.
	pub async fn injections(&self) -> Result<Vec<String>> {
		let value = self.link.request(json!({"injections":true})).await?;
		serde_json::from_value(value).map_err(|e| format!("injections: {e}"))
	}

	pub async fn request_reload(&self) -> Result<()> {
		self.link.request(json!({"reload":true})).await?;
		Ok(())
	}

	/// Explicitly resume a service call after a generation switch. Ordinary
	/// replies and errors are never replayed by the core.
	pub fn resume(args: Value) -> Value {
		json!({"$cartridge_resume":args})
	}
	pub(super) fn write(&self, m: Value) {
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

	/// Publish one event on a named channel of the stream. One line, no setup,
	/// no audience: a channel nobody listens to costs the host one append. The
	/// event's envelope is built on the host — sequence, publisher and kind are
	/// the stream's business, not the publisher's.
	pub fn publish(&self, channel: &str, data: Value) {
		self.write(json!({ "publish": channel, "data": data }));
	}

	/// Watch a channel: every event published to it from here on arrives at
	/// `f` in channel order, each as its full envelope (channel, sequence,
	/// publisher, kind, data), so a watcher can tell a join from a leave and
	/// name how far it has read.
	pub fn subscribe<F, Fut>(&self, channel: &str, f: F)
	where
		F: Fn(Value) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<Value>> + Send + 'static,
	{
		let (tx, mut rx) = mpsc::channel::<Value>(watcher_events());
		let boxed = boxed(f);
		// One pump per subscription: the wire is read in order and the pump
		// handles in order, so two events never race inside the handler.
		tokio::spawn(async move {
			while let Some(envelope) = rx.recv().await {
				let _ = boxed(envelope).await;
			}
		});
		self.streams.lock().insert(channel.to_owned(), tx);
		self.write(json!({ "subscribe": channel }));
	}

	pub(super) fn deliver_stream(&self, channel: &str, envelope: Value) {
		let mut streams = self.streams.lock();
		let Some(tx) = streams.get(channel) else {
			return;
		};
		if tx.capacity() <= 1 {
			let _ = tx.try_send(json!({"ch":channel,"seq":envelope["seq"],
				"from":"sdk","kind":"error","data":{"error":"slow stream handler; resubscribe and resynchronize"}}));
			streams.remove(channel);
			self.write(json!({"unsubscribe":channel}));
		} else if tx.try_send(envelope).is_err() {
			streams.remove(channel);
			self.write(json!({"unsubscribe":channel}));
		}
	}

	/// Stop watching a channel. Both ends of it are themselves events on the
	/// channel, so everyone watching knows who joined and who left.
	pub fn unsubscribe(&self, channel: &str) {
		self.streams.lock().remove(channel);
		self.write(json!({ "unsubscribe": channel }));
	}

	/// Call a key this cartridge injects.
	pub async fn call(&self, key: &str, args: Value) -> Result<Value> {
		Ok(self
			.link
			.request(json!({ "call": key, "args": args }))
			.await?)
	}

	/// Cheap invalidation tokens for service descriptors; changes on replacement.
	pub async fn service_versions(&self, keys: &[String]) -> Result<Value> {
		// Older daemons do not reply to unknown wire messages. Keep hot
		// replacement compatible: uncached discovery remains available.
		if !self.version_queries {
			return Ok(Value::Null);
		}
		Ok(self.link.request(json!({"versions":keys})).await?)
	}
	pub async fn meta(&self, key: &str) -> Result<Value> {
		Ok(self.link.request(json!({ "meta": key })).await?)
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

	pub(super) fn dispatch(
		&self,
		table: &Mutex<HashMap<String, Handler>>,
		m: &Value,
		name: &str,
		data: Value,
	) {
		let id = m["id"].as_u64().unwrap_or(0);
		let handler = table.lock().get(name).cloned();
		let link = self.link.clone();
		let name = name.to_owned();
		// The host's trace continues inside this cartridge, so a nested call or a
		// diagnostic written here names the trace that asked for the work.
		tokio::spawn(crate::trace::scope(crate::trace::of(m), async move {
			let reply = match handler {
				Some(h) => h(data).await,
				None => Err(format!("no handler for {name}")),
			};
			link.reply(id, reply);
		}));
	}

	/// The trace this handler runs in, for a cartridge writing a diagnostic.
	pub fn trace() -> Option<String> {
		crate::trace::current().map(|id| id.to_string())
	}

	pub(super) async fn finish(&self) {
		self.link.close();
		let finalizers = std::mem::take(&mut *self.finalizers.lock());
		for f in finalizers.into_iter().rev() {
			f().await;
		}
		self.link.stop();
	}
}
