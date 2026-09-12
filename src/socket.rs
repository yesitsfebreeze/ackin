use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
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
		.map(|d| PathBuf::from(d).join(format!("zirkle-{hash}.sock")))
		.filter(|p| p.as_os_str().len() < 100)
		.unwrap_or_else(|| {
			let user = std::env::var("USER").unwrap_or_else(|_| "default".into());
			PathBuf::from(format!("/tmp/zirkle-{hash}-{user}.sock"))
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
	loop {
		let (stream, _) = listener.accept().await?;
		tokio::spawn(client(host.clone(), stream));
	}
}

async fn client(host: Arc<Host>, stream: UnixStream) {
	let (read, mut write) = stream.into_split();
	let (reply_tx, mut reply_rx) = tokio::sync::mpsc::unbounded_channel::<Value>();
	let mut lifecycle = host.runtime().lifecycle();
	let mut outbox = host.outbox();
	let writer = tokio::spawn(async move {
		loop {
			let line = tokio::select! {
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
	});
	let mut lines = BufReader::new(read).lines();
	while let Ok(Some(line)) = lines.next_line().await {
		let request: Request = match serde_json::from_str(&line) {
			Ok(r) => r,
			Err(e) => {
				let _ = reply_tx.send(json!({ "error": { "request": line, "message": e.to_string() } }));
				continue;
			}
		};
		if let Some(name) = request.emit {
			host.emit(&name, request.data);
		}
		if request.reload {
			let reply = match host.reconcile().await {
				Ok(()) => json!({ "reloaded": true }),
				Err(e) => json!({ "error": { "cartridge": "init.lua", "message": e.to_string() } }),
			};
			let _ = reply_tx.send(reply);
		}
		if request.status {
			let _ = reply_tx.send(status(&host));
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
			tokio::spawn(crate::turn::scope(turn, async move {
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
	drop(reply_tx);
	let _ = writer.await;
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
		self
			.write
			.write_all(format!("{message}\n").as_bytes())
			.await
	}

	pub async fn next(&mut self) -> Option<Value> {
		let line = self.lines.next_line().await.ok()??;
		serde_json::from_str(&line).ok()
	}
}
