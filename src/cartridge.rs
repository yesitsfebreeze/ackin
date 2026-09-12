//! Process components registered by a Lua wrapper returning
//! `cartridge.process(command, {inject = {"extra.key"}})`. The command's `hello`
//! invocation declares static injections and provides; its normal invocation
//! is one fiber using JSON lines on stdin/stdout. Entry config arrives at apply.
//!
//! Host to cartridge: `{"apply":{"name","config"}}` first, then
//! `{"event",  "data", "id"}` for a listener the cartridge registered,
//! `{"call", "args", "id"}` for a key the cartridge provides, `{"dispose":true}`
//! last. Cartridge to host: `{"provide": key}`, `{"on": name}`,
//! `{"emit", "data"}`, `{"send", "data"}`, `{"call", "args", "id"}`,
//! `{"meta": key, "id"}`, `{"reply": id, "data" | "error"}`, `{"error": text}`,
//! and `{"ready": true}` when its apply is done. The stream rides the same
//! wire: `{"publish": channel, "data"}` from a cartridge, `{"subscribe":
//! channel}` and `{"unsubscribe": channel}` from a cartridge that watches one,
//! and delivery back as `{"channel": channel, "event": envelope}` lines. A
//! value a process provides
//! is a [`Remote`]: calling it from Lua or another process sends `call`.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use parking_lot::Mutex;
use serde_json::{json, Value as Json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

use crate::lua::Host;
use crate::runtime::{Component, Ctx, Disposer, Error, Listener, Value};

pub fn split(cmd: &str) -> Vec<String> {
	cmd.split_whitespace().map(str::to_owned).collect()
}

/// Resolve once so hello, apply and watches name the same executable. A bare
/// program name is searched for in the cartridge's own `bin/`, where a bundle
/// ships it, then beside the running cartridge — a dev-build convenience, since
/// `cargo build --workspace` drops every binary next to this one — then on
/// PATH. Relative explicit paths resolve inside the cartridge directory.
pub fn executable(program: &str, root: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
	use std::os::unix::fs::PermissionsExt;
	let path = std::path::Path::new(program);
	let paths = if path.components().count() > 1 || path.is_absolute() {
		vec![if path.is_absolute() {
			path.to_path_buf()
		} else {
			root.join(path)
		}]
	} else {
		let beside = std::env::current_exe()
			.ok()
			.and_then(|exe| exe.parent().map(|dir| dir.join(program)));
		std::iter::once(root.join("bin").join(program))
			.chain(beside)
			.chain(std::env::var_os("PATH").into_iter().flat_map(|paths| {
				std::env::split_paths(&paths)
					.map(|dir| dir.join(program))
					.collect::<Vec<_>>()
			}))
			.collect()
	};
	paths
		.into_iter()
		.find(|path| {
			path.metadata()
				.is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
		})
		.ok_or_else(|| {
			std::io::Error::new(
				std::io::ErrorKind::NotFound,
				format!("process executable `{program}` was not found or is not executable"),
			)
		})?
		.canonicalize()
}

type Pending = HashMap<u64, oneshot::Sender<Result<Json, String>>>;

/// One end of the wire: outgoing lines and the replies still owed. `None` on
/// the channel stops the writer. Both ends use it — the daemon's here, the
/// cartridge's in [`crate::sdk`] — so `gone` names whichever peer went away.
pub struct Link {
	reload: AtomicBool,
	tx: mpsc::UnboundedSender<Option<Json>>,
	pending: Mutex<Option<Pending>>,
	/// Stream subscriptions this end holds, by channel. The daemon's side uses
	/// it to answer `{"unsubscribe"}`; the cartridge's side never subscribes
	/// over its own link, so the map stays empty there.
	subs: Mutex<HashMap<String, u64>>,
	next: AtomicU64,
	gone: &'static str,
}

impl Link {
	pub fn new(tx: mpsc::UnboundedSender<Option<Json>>, gone: &'static str) -> Arc<Self> {
		Arc::new(Self {
			reload: AtomicBool::new(false),
			tx,
			pending: Mutex::new(Some(HashMap::new())),
			subs: Mutex::new(HashMap::new()),
			next: AtomicU64::new(1),
			gone,
		})
	}

	pub(crate) fn send(&self, m: Json) {
		let _ = self.tx.send(Some(m));
	}

	/// Like [`Link::send`], but tells the caller whether the pipe still took the
	/// frame — the stream forwarders exit on the first `false` and announce
	/// their own leaving, so a dead subscriber is never queued into.
	pub(crate) fn try_send(&self, m: Json) -> bool {
		self.tx.send(Some(m)).is_ok()
	}

	pub(crate) fn set_sub(&self, channel: &str, id: Option<u64>) {
		let mut subs = self.subs.lock();
		match id {
			Some(id) => subs.insert(channel.to_owned(), id),
			None => subs.remove(channel),
		};
	}

	pub(crate) fn sub_id(&self, channel: &str) -> Option<u64> {
		self.subs.lock().get(channel).copied()
	}

	/// Announce every stream subscription this end still holds. Called when the
	/// link is going away for good, so a dead watcher is a `leave` event on the
	/// channel, not a silence.
	pub(crate) fn leave_subs(&self, stream: &crate::stream::Stream) {
		for (channel, id) in std::mem::take(&mut *self.subs.lock()) {
			stream.unsubscribe(&channel, id);
		}
	}

	/// Stop the writer once everything already queued is out.
	pub(crate) fn stop(&self) {
		let _ = self.tx.send(None);
	}

	pub(crate) async fn request(&self, mut m: Json) -> Result<Json, String> {
		let id = self.next.fetch_add(1, Ordering::SeqCst);
		let (tx, rx) = oneshot::channel();
		{
			let mut pending = self.pending.lock();
			let Some(pending) = pending.as_mut() else {
				return Err(self.gone.into());
			};
			pending.insert(id, tx);
		}
		let _waiting = Waiting { link: self, id };
		m["id"] = json!(id);
		crate::turn::stamp(&mut m);
		if self.tx.send(Some(m)).is_err() {
			self.close();
			return Err(self.gone.into());
		}
		rx.await.unwrap_or_else(|_| Err(self.gone.into()))
	}

	pub(crate) fn answer(&self, id: u64, r: Result<Json, String>) {
		if let Some(tx) = self.pending.lock().as_mut().and_then(|p| p.remove(&id)) {
			let _ = tx.send(r);
		}
	}

	#[cfg(test)]
	pub(crate) fn pending_count(&self) -> usize {
		self.pending.lock().as_ref().map_or(0, HashMap::len)
	}

	pub(crate) fn reply(&self, id: u64, r: Result<Json, String>) {
		self.send(match r {
			Ok(data) => json!({ "reply": id, "data": data }),
			Err(error) => json!({ "reply": id, "error": error }),
		});
	}

	/// The inverse of [`Link::reply`]: hand a reply frame back to its waiter.
	/// Returns whether `m` was one, so both ends decode the envelope identically.
	pub fn accept(&self, m: &Json) -> bool {
		let Some(id) = m["reply"].as_u64() else {
			return false;
		};
		self.answer(
			id,
			match m["error"].as_str() {
				Some(e) => Err(e.to_owned()),
				None => Ok(m["data"].clone()),
			},
		);
		true
	}

	pub(crate) fn close(&self) {
		for (_, tx) in self.pending.lock().take().unwrap_or_default() {
			let _ = tx.send(Err(self.gone.into()));
		}
	}

	/// Fail every waiter, then stop the writer once the queue drains.
	pub fn shutdown(&self) {
		self.close();
		self.stop();
	}
}

// The registration belongs to the waiting future, not to its remote handler.
struct Waiting<'a> {
	link: &'a Link,
	id: u64,
}

impl Drop for Waiting<'_> {
	fn drop(&mut self) {
		if let Some(pending) = self.link.pending.lock().as_mut() {
			pending.remove(&self.id);
		}
	}
}

/// A key provided by a process cartridge.
#[derive(Clone)]
pub struct Remote {
	link: Arc<Link>,
	key: String,
}

impl Remote {
	/// A remote over a link this crate did not open itself: a chain node's
	/// client side to its dependency's socket speaks the same frames, so the
	/// need a document declares is a remote value the Lua surface wraps.
	pub fn over(link: Arc<Link>, key: String) -> Self {
		Self { link, key }
	}

	pub(crate) fn process_id(&self) -> usize {
		Arc::as_ptr(&self.link) as usize
	}
	pub(crate) async fn cancel_reload(&self) {
		if self.link.reload.load(Ordering::Relaxed) {
			let _ = self.link.request(json!({"reload":false})).await;
		}
	}
	pub(crate) async fn prepare_reload(&self) -> Result<(), String> {
		if self.link.reload.load(Ordering::Relaxed) {
			self.link.request(json!({"reload":true})).await?;
		}
		Ok(())
	}
	pub async fn call(&self, args: Json) -> Result<Json, String> {
		self.link
			.request(json!({ "call": self.key, "args": args }))
			.await
	}
}

/// What `cmd hello` prints; the shape [`crate::sdk::Cartridge::run`] writes.
#[derive(serde::Deserialize)]
pub struct Manifest {
	#[serde(default)]
	pub reload: bool,
	#[serde(default)]
	pub inject: Vec<String>,
	#[serde(default)]
	pub provide: Vec<String>,
}

pub(crate) fn manifest(cmd: &[String]) -> Result<Manifest, String> {
	// The synchronous composition API also works outside Tokio. Its bounded
	// discovery runtime never nests block_on inside the caller's executor.
	std::thread::scope(|scope| {
		scope
			.spawn(|| {
				tokio::runtime::Builder::new_current_thread()
					.enable_all()
					.build()
					.map_err(|e| e.to_string())?
					.block_on(manifest_async(cmd))
			})
			.join()
			.map_err(|_| "discovery worker panicked".to_owned())?
	})
}

pub(crate) async fn manifest_async(cmd: &[String]) -> Result<Manifest, String> {
	let out = crate::process::discover(cmd, crate::process::STARTUP_TIMEOUT).await?;
	let program = &cmd[0];
	for line in String::from_utf8_lossy(&out.stderr).lines() {
		crate::turn::diagnostic_line(program, line);
	}
	if !out.status.success() {
		return Err(format!("{program} hello: {}", out.status));
	}
	serde_json::from_slice(&out.stdout).map_err(|e| format!("{program} hello: {e}"))
}

pub fn component(
	host: Arc<Host>,
	name: String,
	cmd: Vec<String>,
	config: Json,
) -> Result<Component, Error> {
	let m = manifest(&cmd).map_err(Error::Apply)?;
	let cartridge = name.clone();
	let apply = Arc::new(move |ctx: Ctx| {
		let (host, name, cmd, config) =
			(host.clone(), cartridge.clone(), cmd.clone(), config.clone());
		futures::stream::once(async move { start(host, ctx, name, cmd, config, m.reload).await })
			.boxed()
	});
	Ok(Component::new(name, apply)
		.inject(m.inject)
		.provide(m.provide))
}

async fn start(
	host: Arc<Host>,
	ctx: Ctx,
	name: String,
	cmd: Vec<String>,
	config: Json,
	reload: bool,
) -> Result<Disposer, Error> {
	let mut child = crate::process::Child::new(
		Command::new(&cmd[0])
			.args(&cmd[1..])
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.kill_on_drop(true)
			.spawn()
			.map_err(|e| Error::Apply(format!("{}: {e}", cmd[0])))?,
	);
	let mut stdin = child.stdin.take().expect("piped stdin");
	let stdout = child.stdout.take().expect("piped stdout");
	// The host owns every diagnostic byte: redaction and the byte cap are only
	// possible where it formats the writes, so a cartridge never inherits stderr.
	let stderr = child.stderr.take().expect("piped stderr");
	child.tasks.spawn({
		let name = name.clone();
		async move {
			let mut lines = BufReader::new(stderr).lines();
			while let Ok(Some(line)) = lines.next_line().await {
				crate::turn::diagnostic_line(&name, &line);
			}
		}
	});
	let (tx, mut rx) = mpsc::unbounded_channel::<Option<Json>>();
	let link = Link::new(tx, "cartridge is gone");
	link.reload.store(reload, Ordering::Relaxed);
	let writer_link = link.clone();
	child.tasks.spawn(async move {
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
	let mut startup = BufReader::new(stdout);
	let mut remaining = 64 * 1024;
	let ready = tokio::time::timeout(crate::process::STARTUP_TIMEOUT, async {
		loop {
			let Some(line) = crate::process::startup_line(&mut startup, &mut remaining)
				.await
				.map_err(|error| {
					link.shutdown();
					Error::Apply(error.to_string())
				})?
			else {
				link.shutdown();
				let status = child
					.try_wait()
					.ok()
					.flatten()
					.map(|s| s.to_string())
					.unwrap_or_else(|| "closed stdout".into());
				return Err(Error::Apply(format!("exited before ready: {status}")));
			};
			let m: Json = serde_json::from_str(&line).map_err(|e| {
				link.shutdown();
				Error::Apply(e.to_string())
			})?;
			if m["ready"] == true {
				break;
			}
			if let Some(e) = m["error"]
				.as_str()
				.filter(|_| m["reply"].as_u64().is_none())
			{
				link.shutdown();
				return Err(Error::Apply(e.to_owned()));
			}
			if let Err(e) = handle(&host, &ctx, &link, &name, m) {
				link.shutdown();
				return Err(e);
			}
		}
		Ok::<(), Error>(())
	})
	.await;
	match ready {
		Ok(Ok(())) => {}
		Ok(Err(error)) => {
			link.shutdown();
			return Err(error);
		}
		Err(_) => {
			link.shutdown();
			return Err(Error::Apply(format!("{name} timed out before ready")));
		}
	}
	let mut lines = startup.lines();
	let stopping = Arc::new(AtomicBool::new(false));
	child.tasks.spawn({
		let (host, ctx, link, name, stopping) = (
			host.clone(),
			ctx.clone(),
			link.clone(),
			name.clone(),
			stopping.clone(),
		);
		async move {
			while let Ok(Some(line)) = lines.next_line().await {
				let result = match serde_json::from_str::<Json>(&line) {
					Ok(m) => match m["error"].as_str() {
						Some(e) if m["reply"].as_u64().is_none() => Err(Error::Apply(e.to_owned())),
						_ => handle(&host, &ctx, &link, &name, m),
					},
					Err(e) => Err(Error::Apply(e.to_string())),
				};
				if let Err(e) = result {
					host.report(&name, e);
				}
			}
			link.shutdown();
			link.leave_subs(host.runtime().stream());
			if !stopping.load(Ordering::SeqCst) {
				ctx.runtime()
					.fail(ctx.fiber(), Error::Apply("exited".into()));
			}
		}
	});
	Ok(Box::new(move || {
		Box::pin(async move {
			stopping.store(true, Ordering::SeqCst);
			link.close();
			link.leave_subs(host.runtime().stream());
			link.send(json!({ "dispose": true }));
			link.stop();
			let mut child = child;
			if tokio::time::timeout(Duration::from_secs(5), child.wait())
				.await
				.is_err()
			{
				let _ = child.kill().await;
			}
		})
	}))
}

fn handle(host: &Arc<Host>, ctx: &Ctx, link: &Arc<Link>, name: &str, m: Json) -> Result<(), Error> {
	if let Some(key) = m["provide"].as_str() {
		let remote = Remote {
			link: link.clone(),
			key: key.to_owned(),
		};
		return ctx.provide(key, Arc::new(remote));
	}
	if let Some(name) = m["on"].as_str() {
		let (host, link, event) = (host.clone(), link.clone(), name.to_owned());
		let listener: Listener = Arc::new(move |payload| {
			let (link, event, data) = (link.clone(), event.clone(), host.json_of(&payload));
			Box::pin(async move {
				link.request(json!({ "event": event, "data": data }))
					.await
					.map(|d| (!d.is_null()).then(|| Arc::new(d) as Value))
					.map_err(Error::Apply)
			})
		});
		ctx.on(name, listener);
		return Ok(());
	}
	if let Some(name) = m["emit"].as_str() {
		ctx.emit(name, Arc::new(m["data"].clone()));
		return Ok(());
	}
	if let Some(name) = m["send"].as_str() {
		host.send_event(name, m["data"].clone());
		return Ok(());
	}
	if let Some(channel) = m["publish"].as_str() {
		// Publishing knows nothing of its audience and asks nothing about it: the
		// stream takes the envelope, and an empty channel costs one append.
		host.runtime().stream().publish(
			channel,
			name,
			crate::stream::Kind::Data,
			m["data"].clone(),
		);
		return Ok(());
	}
	if let Some(channel) = m["subscribe"].as_str() {
		// One subscription per channel per link: a repeat asks for what it
		// already holds, and the old pump would keep delivering underneath it.
		if link.sub_id(channel).is_some() {
			return Ok(());
		}
		let stream = host.runtime().stream().clone();
		let sub = stream.subscribe(channel, name, None);
		link.set_sub(channel, Some(sub.id));
		let (link, channel, stream) = (link.clone(), channel.to_owned(), stream.clone());
		tokio::spawn(async move {
			let mut rx = sub.rx;
			let id = sub.id;
			while let Some(envelope) = rx.recv().await {
				if !link.try_send(json!({ "channel": channel, "event": envelope })) {
					// The pipe is gone, so the leaving is announced here and the
					// forwarder is over.
					stream.unsubscribe(&channel, id);
					break;
				}
			}
		});
		return Ok(());
	}
	if let Some(channel) = m["unsubscribe"].as_str() {
		if let Some(id) = link.sub_id(channel) {
			link.set_sub(channel, None);
			host.runtime().stream().unsubscribe(channel, id);
		}
		return Ok(());
	}
	if link.accept(&m) {
		return Ok(());
	}
	let id = m["id"].as_u64().unwrap_or(0);
	if m["bridge"] == "status" {
		link.reply(id, Ok(host.bridge_status()));
		return Ok(());
	}
	if m["landscape"] == true {
		link.reply(id, Ok(host.landscape()));
		return Ok(());
	}
	if m["cartridges"] == true {
		link.reply(id, Ok(host.cartridges()));
		return Ok(());
	}
	if m["injections"] == true {
		link.reply(id, Ok(json!(ctx.injections())));
		return Ok(());
	}
	if m["bridge"] == "call" {
		if !host.bridge_enabled(ctx.fiber()) {
			link.reply(id, Err("profile has not granted bridge access".into()));
			return Ok(());
		}
		let (host, link, call) = (host.clone(), link.clone(), m.clone());
		tokio::spawn(async move {
			let result = match (
				call["owner"].as_str(),
				call["generation"].as_u64(),
				call["key"].as_str(),
			) {
				(Some(owner), Some(generation), Some(key)) => {
					host.bridge_call(owner, generation, key, call["args"].clone())
						.await
				}
				_ => Err("invalid bridge service call".into()),
			};
			link.reply(id, result);
		});
		return Ok(());
	}
	if m["reload"] == true {
		host.request_reload(ctx.fiber());
		link.reply(id, Ok(Json::Null));
		return Ok(());
	}
	if let Some(key) = m["meta"].as_str() {
		link.reply(id, Ok(ctx.meta(key)));
		return Ok(());
	}
	if let Some(key) = m["call"].as_str() {
		let (host, ctx, link, key, args, turn) = (
			host.clone(),
			ctx.clone(),
			link.clone(),
			key.to_owned(),
			m["args"].clone(),
			crate::turn::of(&m),
		);
		tokio::spawn(crate::turn::scope(turn, async move {
			let r = match ctx.get(&key) {
				Ok(v) => host.invoke(v, args).await,
				Err(e) => Err(e.to_string()),
			};
			link.reply(id, r);
		}));
		return Ok(());
	}
	Err(Error::Apply(format!("unknown message {m}")))
}
