use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast;

use crate::lua::Host;

#[derive(Deserialize, Default)]
#[serde(default)]
struct Request {
	emit: Option<String>,
	data: Value,
	reload: bool,
	status: bool,
	call: Option<String>,
	args: Value,
	id: Value,
	/// A client that already names its turn keeps it; otherwise the host mints one.
	turn: Option<String>,
	/// Debug mode: `"on"`, `"off"`, or `"status"` to only report.
	debug: Option<String>,
	/// Watch a stream channel; delivery arrives as `{"channel", "event"}` lines.
	subscribe: Option<String>,
	/// With `subscribe`: resume from the sequence the watcher last saw, so a
	/// watcher that lost its connection ends up where it was.
	since: Option<u64>,
	/// Stop watching one.
	unsubscribe: Option<String>,
	/// Publish one event on a channel, as `{"publish": channel, "data": data}`.
	publish: Option<String>,
}

pub fn path(dir: &Path) -> PathBuf {
	sockpath(dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf()))
}

/// The socket one chain node serves, keyed by the node's ledger path under the
/// tree's root: any process that knows the root and the node can derive it, so
/// a dependent finds its dependency's socket without being told where it is,
/// and two nodes of one tree never share one.
pub fn node_path(root: &Path, node: &str) -> PathBuf {
	let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
	sockpath((canonical, node.to_owned()))
}

fn sockpath(what: impl Hash) -> PathBuf {
	let mut h = DefaultHasher::new();
	what.hash(&mut h);
	let hash = format!("{:016x}", h.finish());
	std::env::var_os("XDG_RUNTIME_DIR")
		.map(|d| PathBuf::from(d).join(format!("cartridge-{hash}.sock")))
		.filter(|p| p.as_os_str().len() < 100)
		.unwrap_or_else(|| {
			let user = std::env::var("USER").unwrap_or_else(|_| "default".into());
			PathBuf::from(format!("/tmp/cartridge-{hash}-{user}.sock"))
		})
}

pub fn status(host: &Host) -> Value {
	json!({ "status": host.runtime().fibers() })
}

pub async fn serve(host: Arc<Host>, path: &Path) -> std::io::Result<()> {
	if UnixStream::connect(path).await.is_ok() {
		return Err(std::io::Error::new(
			std::io::ErrorKind::AddrInUse,
			"a daemon already serves this directory",
		));
	}
	let _ = std::fs::remove_file(path);
	let listener = UnixListener::bind(path)?;
	let mut clients = tokio::task::JoinSet::new();
	loop {
		tokio::select! {
			accepted = listener.accept() => {
				let (stream, _) = accepted?;
				clients.spawn(client(host.clone(), stream));
			}
			_ = clients.join_next(), if !clients.is_empty() => {}
		}
	}
}

struct Subscriptions {
	stream: Arc<crate::stream::Stream>,
	active: HashMap<String, (u64, tokio::task::AbortHandle)>,
}

impl Subscriptions {
	fn remove(&mut self, channel: &str) {
		if let Some((id, task)) = self.active.remove(channel) {
			task.abort();
			self.stream.unsubscribe(channel, id);
		}
	}
}

impl Drop for Subscriptions {
	fn drop(&mut self) {
		for (channel, (id, task)) in self.active.drain() {
			task.abort();
			self.stream.unsubscribe(&channel, id);
		}
	}
}

async fn client(host: Arc<Host>, stream: UnixStream) {
	let (read, mut write) = stream.into_split();
	let (reply_tx, mut reply_rx) = tokio::sync::mpsc::unbounded_channel::<Value>();
	// Stream subscriptions of this one connection, by channel.
	let mut subs = Subscriptions {
		stream: host.runtime().stream().clone(),
		active: HashMap::new(),
	};
	let mut tasks = tokio::task::JoinSet::new();
	let (writer_done, mut writer_finished) = tokio::sync::oneshot::channel();
	let mut lifecycle = host.runtime().lifecycle();
	let mut outbox = host.outbox();
	tasks.spawn(async move {
		let mut peer_check = tokio::time::interval(std::time::Duration::from_millis(100));
		loop {
			let line = tokio::select! {
				_ = peer_check.tick() => {
					// A zero-byte write probes peer closure without sending a
					// protocol heartbeat. A read-half close still accepts writes.
					if let Err(error) = write.try_write(&[]) {
						if error.kind() != std::io::ErrorKind::WouldBlock { break; }
					}
					continue;
				}
				t = lifecycle.recv() => match t {
					Ok(t) => json!({ "fiber": t }),
					Err(broadcast::error::RecvError::Lagged(_)) => continue,
					Err(_) => break,
				},
				m = outbox.recv() => match m {
					Ok(v) => v,
					Err(broadcast::error::RecvError::Lagged(_)) => continue,
					Err(_) => break,
				},
				r = reply_rx.recv() => match r {
					Some(v) => v,
					None => break,
				},
			};
			if write
				.write_all(format!("{line}\n").as_bytes())
				.await
				.is_err()
			{
				break;
			}
		}
		let _ = writer_done.send(());
	});
	let mut writer_ended = false;
	let mut lines = BufReader::new(read).lines();
	loop {
		let line = tokio::select! {
			_ = &mut writer_finished => { writer_ended = true; break; }
			_ = tasks.join_next(), if !tasks.is_empty() => { continue; }
			line = lines.next_line() => match line { Ok(Some(line)) => line, _ => break },
		};
		let request: Request = match serde_json::from_str(&line) {
			Ok(r) => r,
			Err(e) => {
				let _ = reply_tx
					.send(json!({ "error": { "request": line, "message": e.to_string() } }));
				continue;
			}
		};
		if let Some(channel) = request.publish {
			host.runtime().stream().publish(
				&channel,
				"socket",
				crate::stream::Kind::Data,
				request.data.clone(),
			);
		}
		if let Some(name) = request.emit {
			host.emit(&name, request.data);
		}
		if request.reload {
			let transaction = host.start_reconcile();
			let reply_tx = reply_tx.clone();
			tasks.spawn(async move {
				let result = transaction
					.await
					.map_err(mlua::Error::external)
					.and_then(|result| result);
				let reply = match result {
					Ok(()) => json!({"reloaded":true}),
					Err(error) => {
						json!({"error":{"cartridge":"init.lua","message":error.to_string()}})
					}
				};
				let _ = reply_tx.send(reply);
			});
		}

		if request.status {
			let _ = reply_tx.send(status(&host));
		}
		if let Some(channel) = request.subscribe {
			// One subscription per channel per connection: a repeat asks for what
			// it already holds, and the old pump would keep delivering underneath.
			if subs.active.contains_key(&channel) {
				continue;
			}
			let stream = host.runtime().stream().clone();
			let sub = stream.subscribe(&channel, "socket", request.since);

			// The weak handle is the disconnect detector: once the writer is
			// gone its send fails, the leaving is announced, and the pump is over.
			let pump_tx = reply_tx.downgrade();
			let (pump_channel, pump_stream, id) = (channel.clone(), stream.clone(), sub.id);
			let registered = channel.clone();
			let pump = tasks.spawn(async move {
				let mut rx = sub.rx;
				while let Some(envelope) = rx.recv().await {
					let sent = pump_tx
						.upgrade()
						.map(|tx| tx.send(json!({ "channel": channel, "event": envelope })));
					if !matches!(sent, Some(Ok(()))) {
						pump_stream.unsubscribe(&pump_channel, id);
						break;
					}
				}
			});
			subs.active.insert(registered, (id, pump));
		}
		if let Some(channel) = request.unsubscribe {
			subs.remove(&channel);
		}
		if let Some(state) = request.debug {
			let reply = match state.as_str() {
				"on" => json!({ "debug": host.debug(Some(true)) }),
				"off" => json!({ "debug": host.debug(Some(false)) }),
				"status" => json!({ "debug": host.debug(None) }),
				other => json!({
					"error": {
						"cartridge": "debug",
						"message": format!("debug takes on, off or status, not {other}")
					}
				}),
			};
			let _ = reply_tx.send(reply);
		}
		if let Some(key) = request.call {
			let (host, reply_tx) = (host.clone(), reply_tx.clone());
			let turn = request
				.turn
				.map(std::sync::Arc::from)
				.unwrap_or_else(crate::turn::mint);
			tasks.spawn(crate::turn::scope(turn, async move {
				let reply = match host.call(&key, request.args).await {
					Ok(data) => json!({ "reply": request.id, "data": data }),
					Err(e) => json!({ "reply": request.id, "error": e }),
				};
				let _ = reply_tx.send(reply);
			}));
		}
	}
	// The writer ends once every in-flight `call` has dropped its sender, so a
	// reply computed just as the client hung up still reaches the socket.
	drop(subs);
	drop(reply_tx);
	if !writer_ended {
		let _ = writer_finished.await;
	}
	tasks.abort_all();
	while tasks.join_next().await.is_some() {}
}

pub struct Client {
	lines: tokio::io::Lines<BufReader<tokio::net::unix::OwnedReadHalf>>,
	write: tokio::net::unix::OwnedWriteHalf,
}

impl Client {
	pub async fn connect(path: &Path) -> std::io::Result<Self> {
		let (read, write) = UnixStream::connect(path).await?.into_split();
		Ok(Self {
			lines: BufReader::new(read).lines(),
			write,
		})
	}

	pub async fn send(&mut self, message: Value) -> std::io::Result<()> {
		self.write
			.write_all(format!("{message}\n").as_bytes())
			.await
	}

	pub async fn next(&mut self) -> Option<Value> {
		let line = self.lines.next_line().await.ok()??;
		serde_json::from_str(&line).ok()
	}
}

#[cfg(test)]
#[path = "../.cartridge/tests/unit/src/socket/tests.rs"]
mod tests;
