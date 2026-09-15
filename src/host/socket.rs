use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::transport::rpc::{self, Incoming, Peer, Request};
use serde_json::{json, Value};
use tokio::sync::{broadcast, mpsc};

use crate::error::{Error, Result};

use super::Host;

fn owner_only_dir(dir: &Path) -> Result<()> {
	let mut builder = std::fs::DirBuilder::new();
	builder.recursive(true);
	#[cfg(unix)]
	{
		use std::os::unix::fs::DirBuilderExt;
		builder.mode(0o700);
	}
	builder.create(dir).map_err(|e| Error::file(dir, e))?;
	// Never followed: a link standing in for the directory is a substitution,
	// and what is asked of the name is asked of the name itself.
	let meta = std::fs::symlink_metadata(dir).map_err(|e| Error::file(dir, e))?;
	#[cfg(unix)]
	{
		use std::os::unix::fs::{MetadataExt, PermissionsExt};
		// SAFETY: `getuid` cannot fail and touches no memory.
		let uid = unsafe { libc::getuid() };
		if !meta.is_dir() || meta.uid() != uid {
			return Err(Error::Descriptor(format!(
				"{} is not a directory owned by this user",
				dir.display()
			)));
		}
		if meta.permissions().mode() & 0o077 != 0 {
			std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
				.map_err(|e| Error::file(dir, e))?;
		}
	}
	// Windows has no mode to narrow and no uid to compare, so only a name that
	// is not a directory is refused there.
	#[cfg(windows)]
	if !meta.is_dir() {
		return Err(Error::Descriptor(format!(
			"{} is not a directory owned by this user",
			dir.display()
		)));
	}
	Ok(())
}

/// A unix socket's path is capped near 100 bytes by the address family, so
/// `XDG_RUNTIME_DIR` is only taken when it is short too.
pub fn base() -> Result<PathBuf> {
	#[cfg(unix)]
	let base = {
		// SAFETY: `getuid` cannot fail and touches no memory.
		let uid = unsafe { libc::getuid() };
		std::env::var_os("XDG_RUNTIME_DIR")
			.map(|dir| PathBuf::from(dir).join("cartridge"))
			.filter(|dir| dir.as_os_str().len() < 48)
			.unwrap_or_else(|| PathBuf::from(format!("/tmp/cartridge-{uid}")))
	};
	#[cfg(windows)]
	let base = std::env::var_os("LOCALAPPDATA")
		.map(|dir| PathBuf::from(dir).join("cartridge"))
		.ok_or_else(|| Error::Descriptor("LOCALAPPDATA names no directory".into()))?;
	owner_only_dir(&base)?;
	Ok(base)
}

fn tag(descriptor: &Path) -> String {
	crate::transport::typed::path_tag(descriptor)[..12].to_owned()
}

/// Derived, not published: the command line computes the same address from
/// the same project.
pub fn path(descriptor: &Path) -> Result<PathBuf> {
	Ok(run_dir(descriptor)?.join("host.sock"))
}

pub(crate) fn run_dir(descriptor: &Path) -> Result<PathBuf> {
	let dir = base()?.join(tag(descriptor));
	owner_only_dir(&dir)?;
	Ok(dir)
}

/// Several bases of one project run at once and each unlinks its sockets when
/// it stops, so they must not share one directory or the second to start
/// takes the first's node sockets away. Named by pid, so a dead run's
/// directory is recognised and removed by the next one to start.
pub(crate) fn host_dir(descriptor: &Path) -> Result<PathBuf> {
	let run = run_dir(descriptor)?;
	sweep(&run);
	let dir = run.join(std::process::id().to_string());
	owner_only_dir(&dir)?;
	Ok(dir)
}

/// `host.sock` is always spared here; reclaiming it is the bind path's own
/// stale-sock probe. A loose socket that still answers a connect belongs to a
/// live base and stays.
fn sweep(run: &Path) {
	let Ok(entries) = std::fs::read_dir(run) else {
		return;
	};
	for entry in entries.flatten() {
		let name = entry.file_name();
		let path = entry.path();
		let Some(pid) = name.to_str().and_then(|n| n.parse::<u32>().ok()) else {
			if name != "host.sock" && entry.file_type().is_ok_and(|t| t.is_file()) && !served(&path)
			{
				let _ = std::fs::remove_file(&path);
			}
			continue;
		};
		if pid != std::process::id() && !alive(pid) {
			let _ = std::fs::remove_dir_all(&path);
		}
	}
}

#[cfg(unix)]
fn served(path: &Path) -> bool {
	std::os::unix::net::UnixStream::connect(path).is_ok()
}

// ponytail: Windows leaves nothing to distinguish without the pipe peer, so a
// loose file there is never swept; the directory holds the pipe names anyway.
#[cfg(windows)]
fn served(_path: &Path) -> bool {
	true
}

#[cfg(unix)]
fn alive(pid: u32) -> bool {
	// SAFETY: signal 0 delivers nothing; it only asks whether `pid` exists.
	let exists = unsafe { libc::kill(pid as libc::pid_t, 0) } == 0;
	exists || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

// ponytail: no liveness check on Windows, so nothing is swept there; a dead
// run's directory stays until the user clears it.
#[cfg(windows)]
fn alive(_pid: u32) -> bool {
	true
}

pub(crate) fn file_name(id: &str) -> String {
	let name: String = id
		.chars()
		.map(|c| {
			if c.is_ascii_alphanumeric() || c == '-' {
				c
			} else {
				'_'
			}
		})
		.collect();
	format!("{name}.sock")
}

pub(crate) async fn listen(path: &Path) -> Result<crate::transport::typed::LocalListener> {
	match crate::transport::typed::bind(&crate::transport::typed::Endpoint::local(path)).await {
		Ok(crate::transport::typed::BindOutcome::Bound(listener)) => Ok(listener),
		Ok(crate::transport::typed::BindOutcome::AlreadyRunning) => Err(Error::Remote(format!(
			"{} is already served",
			path.display()
		))),
		Err(error) => Err(Error::Remote(format!("{}: {error}", path.display()))),
	}
}

pub(crate) async fn accept(
	host: std::sync::Weak<Host>,
	mut listener: crate::transport::typed::LocalListener,
) {
	let Some(host) = host.upgrade() else { return };
	let stop = host.stop_signal();
	loop {
		tokio::select! {
			_ = stop.cancelled() => break,
			accepted = listener.accept() => match accepted {
				Ok(adapter) => {
					let stop = host.stop_signal();
					tokio::spawn(connection(host.clone(), adapter, stop));
				}
				// Transient (fd table exhausted, say): ending the loop here
				// would leave the base running with a socket nobody can
				// reach, so log and go on.
				Err(error) => tracing::warn!(
					target: "cartridge",
					"accept failed, continuing: {error}"
				),
			},
		}
	}
}

pub async fn serve(host: Arc<Host>) -> Result<bool> {
	host.stopped().await;
	Ok(true)
}

pub(crate) fn write_private(path: &Path, text: &str) -> Result<()> {
	use std::io::Write;
	let _ = std::fs::remove_file(path);
	let mut options = std::fs::OpenOptions::new();
	options.write(true).create_new(true);
	// Windows has no mode to open with; the socket directory's own privacy
	// covers it there.
	#[cfg(unix)]
	{
		use std::os::unix::fs::OpenOptionsExt;
		options.mode(0o600);
	}
	let mut file = options.open(path).map_err(|e| Error::file(path, e))?;
	file.write_all(text.as_bytes())
		.map_err(|e| Error::file(path, e))
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/host/socket.rs"]
mod tests;

#[derive(Clone)]
enum Caller {
	Host,
	Cartridge,
}

async fn connection(
	host: Arc<Host>,
	adapter: crate::transport::typed::LocalAdapter,
	stop: tokio_util::sync::CancellationToken,
) {
	let (peer, mut incoming) = Peer::spawn(adapter, Some(1 << 26));
	let caller = match incoming.recv().await {
		Some(Incoming::Request(request)) if request.method == "auth" => {
			let token = request.params["token"].as_str().unwrap_or_default();
			// The socket is owner-only, so whoever reaches it is the user: no
			// token means the command line.
			// ponytail: a node that omits its token passes as the command line;
			// bind the node role to the peer instead if nodes ever run untrusted.
			let caller = match token {
				"" => Some(Caller::Host),
				token if token == host.host_token() => Some(Caller::Host),
				token => host.caller(token).map(|_| Caller::Cartridge),
			};
			match caller {
				Some(caller) => {
					request.reply(Ok(json!({ "cartridge": "host" })));
					Some(caller)
				}
				None => {
					request.reply(Err(rpc::Error::new(rpc::UNAUTHORIZED, "unknown token")));
					None
				}
			}
		}
		Some(Incoming::Request(request)) => {
			request.reply(Err(rpc::Error::new(
				rpc::UNAUTHORIZED,
				"authenticate first",
			)));
			None
		}
		_ => None,
	};
	let Some(caller) = caller else {
		peer.close();
		peer.flushed().await;
		return;
	};
	while let Some(message) = incoming.recv().await {
		if let Incoming::Request(request) = message {
			tokio::spawn(handle(
				host.clone(),
				peer.clone(),
				caller.clone(),
				request,
				stop.clone(),
			));
		}
	}
}

fn handle(
	host: Arc<Host>,
	peer: Peer,
	caller: Caller,
	request: Request,
	stop: tokio_util::sync::CancellationToken,
) -> futures::future::BoxFuture<'static, ()> {
	Box::pin(answer(host, peer, caller, request, stop))
}

async fn answer(
	host: Arc<Host>,
	peer: Peer,
	caller: Caller,
	request: Request,
	stop: tokio_util::sync::CancellationToken,
) {
	let params = request.params.clone();
	let application = |e: Error| rpc::Error::application(e.to_string());
	let method = request.method.clone();
	let granted = matches!(
		(&caller, method.as_str()),
		(Caller::Host, _) | (Caller::Cartridge, "status" | "snapshot" | "cartridges")
	);
	if !granted {
		let message = format!("`{method}` is not granted to this token");
		return request.reply(Err(rpc::Error::new(rpc::UNAUTHORIZED, message)));
	}
	match method.as_str() {
		"status" => request.reply(Ok(json!(host.status()))),
		"snapshot" => request.reply(Ok(host.snapshot())),
		"cartridges" => request.reply(Ok(host.cartridges())),
		"bail" => {
			let name = params["name"].as_str().unwrap_or_default().to_owned();
			let result = host.bail(&name, params["data"].clone()).await;
			request.reply(
				result
					.map(|answer| answer.unwrap_or(Value::Null))
					.map_err(application),
			);
		}
		"gather" => {
			let name = params["name"].as_str().unwrap_or_default().to_owned();
			let result = host.gather(&name, params["data"].clone()).await;
			request.reply(result.map(|outcomes| json!(outcomes)).map_err(application));
		}
		"reload" => {
			let result = match params["cartridge"].as_str() {
				Some(id) => host.replace(id).await,
				None => host.reconcile().await,
			};
			request.reply(result.map(|()| json!({})).map_err(application));
		}
		"subscribe" => {
			let channel = params["channel"].as_str().unwrap_or_default().to_owned();
			match follow(&host, &peer, &channel, params["since"].as_u64()).await {
				Ok(()) => request.reply(Ok(json!({}))),
				Err(error) => request.reply(Err(application(error))),
			}
		}
		"stop" => {
			request.reply(Ok(json!({})));
			stop.cancel();
		}
		other => request.reply(Err(rpc::Error::new(
			rpc::METHOD_NOT_FOUND,
			format!("unknown method `{other}`"),
		))),
	}
}

async fn follow(host: &Arc<Host>, peer: &Peer, channel: &str, since: Option<u64>) -> Result<()> {
	if channel == "lifecycle" {
		tokio::spawn(forward(host.lifecycle(), peer.clone()));
		return Ok(());
	}
	let (cartridge, local) = channel.split_once('.').ok_or_else(|| {
		Error::Argument(format!(
			"`{channel}` is not `<cartridge>.<channel>` or `lifecycle`"
		))
	})?;
	let (upstream, mut incoming) = host.open(cartridge).await?;
	upstream
		.call("subscribe", json!({ "channel": local, "since": since }))
		.await
		.map_err(|e| Error::Remote(e.message))?;
	let (peer, full) = (peer.clone(), channel.to_owned());
	tokio::spawn(async move {
		// This connection is this task's alone; leaving either side open past
		// the other is a leak that only the next publish would surface.
		loop {
			tokio::select! {
				_ = peer.closed() => break,
				message = incoming.recv() => match message {
					Some(Incoming::Notification { mut params, .. }) => {
						params["channel"] = json!(full);
						if peer.notify("channel", params).is_err() {
							break;
						}
					}
					Some(_) | None => break,
				},
			}
		}
		drop(upstream);
	});
	Ok(())
}

/// A subscriber the broadcast outruns is disconnected: these frames carry no
/// `seq` to resume from, and a connection that silently hears nothing more is
/// worse than a closed one.
pub(crate) async fn forward(mut events: broadcast::Receiver<Value>, peer: Peer) {
	while let Ok(event) = events.recv().await {
		if peer
			.notify(
				"channel",
				json!({ "channel": "lifecycle", "kind": "data", "data": event }),
			)
			.is_err()
		{
			return;
		}
	}
	peer.close();
}

pub async fn client(descriptor: &Path) -> Result<(Peer, mpsc::Receiver<Incoming>)> {
	super::connect(&path(descriptor)?, "").await
}
