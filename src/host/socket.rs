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
	let meta = std::fs::symlink_metadata(dir).map_err(|e| Error::file(dir, e))?;
	#[cfg(unix)]
	{
		use std::os::unix::fs::{MetadataExt, PermissionsExt};
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
	#[cfg(windows)]
	if !meta.is_dir() {
		return Err(Error::Descriptor(format!(
			"{} is not a directory owned by this user",
			dir.display()
		)));
	}
	Ok(())
}

pub fn base() -> Result<PathBuf> {
	#[cfg(unix)]
	let base = {
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

pub fn path(descriptor: &Path) -> Result<PathBuf> {
	Ok(run_dir(descriptor)?.join("host.sock"))
}

pub(crate) fn run_dir(descriptor: &Path) -> Result<PathBuf> {
	let dir = base()?.join(tag(descriptor));
	owner_only_dir(&dir)?;
	Ok(dir)
}

pub(crate) fn host_dir(descriptor: &Path) -> Result<PathBuf> {
	let run = run_dir(descriptor)?;
	sweep(&run);
	let dir = run.join(std::process::id().to_string());
	owner_only_dir(&dir)?;
	Ok(dir)
}

/// Collect what a killed host left in one run directory: node directories
/// of dead pids and socket files nobody answers on. `<id>.port` files stay;
/// they carry a node's address to the next host.
fn sweep(run: &Path) -> usize {
	let _names = lock_names(run);
	let Ok(entries) = std::fs::read_dir(run) else {
		return 0;
	};
	let mut removed = 0;
	for entry in entries.flatten() {
		let name = entry.file_name();
		let path = entry.path();
		if let Some(pid) = name.to_str().and_then(|n| n.parse::<u32>().ok()) {
			if pid != std::process::id() && !alive(pid) && std::fs::remove_dir_all(&path).is_ok() {
				removed += 1;
			}
			continue;
		}
		if is_socket(&entry) && !answers(&path) && std::fs::remove_file(&path).is_ok() {
			removed += 1;
		}
	}
	removed
}

/// Alive: a run directory whose host answers, or that holds a live pid.
fn run_alive(run: &Path) -> bool {
	answers(&run.join("host.sock"))
		|| std::fs::read_dir(run).is_ok_and(|entries| {
			entries.flatten().any(|entry| {
				entry
					.file_name()
					.to_str()
					.and_then(|n| n.parse::<u32>().ok())
					.is_some_and(alive)
			})
		})
}

/// `cartridge sweep`: every project's run directory under the base, plus the
/// files earlier layouts wrote there (`<tag>.sock`, `<tag>.token`,
/// `<tag>-<pid>/`). Returns how many entries went.
pub fn sweep_all() -> Result<usize> {
	let base = base()?;
	let entries = std::fs::read_dir(&base).map_err(|e| Error::file(&base, e))?;
	let mut removed = 0;
	for entry in entries.flatten() {
		let path = entry.path();
		let name = entry.file_name().to_string_lossy().into_owned();
		let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
		if is_dir && name.len() == 12 && name.chars().all(|c| c.is_ascii_hexdigit()) {
			removed += sweep(&path);
			if !run_alive(&path) && std::fs::remove_dir_all(&path).is_ok() {
				removed += 1;
			}
			continue;
		}
		// An earlier layout put `<tag>.sock` and `<tag>.token` at the base. The
		// token is the command line's way in, so it goes only once its socket
		// no longer answers.
		let unanswered = |tag: &str| !answers(&base.join(format!("{tag}.sock")));
		let gone = match name.rsplit_once('-') {
			Some((_, pid)) if is_dir => pid.parse::<u32>().is_ok_and(|pid| !alive(pid)),
			_ => match name.strip_suffix(".token") {
				Some(tag) => unanswered(tag),
				None => {
					(name.ends_with(".sock") && !answers(&path))
						|| (is_dir
							&& std::fs::read_dir(&path).is_ok_and(|mut d| d.next().is_none()))
				}
			},
		};
		if !gone {
			continue;
		}
		let ok = if is_dir {
			std::fs::remove_dir_all(&path).is_ok()
		} else {
			std::fs::remove_file(&path).is_ok()
		};
		if ok {
			removed += 1;
		}
	}
	Ok(removed)
}

#[cfg(unix)]
fn is_socket(entry: &std::fs::DirEntry) -> bool {
	use std::os::unix::fs::FileTypeExt;
	entry.file_type().is_ok_and(|t| t.is_socket())
}

#[cfg(windows)]
fn is_socket(_entry: &std::fs::DirEntry) -> bool {
	false
}

/// Device and inode of the socket file, the identity a stop compares before
/// unlinking by path.
#[cfg(unix)]
pub(crate) fn identity(path: &Path) -> std::io::Result<(u64, u64)> {
	use std::os::unix::fs::MetadataExt;
	let meta = std::fs::symlink_metadata(path)?;
	Ok((meta.dev(), meta.ino()))
}

#[cfg(windows)]
pub(crate) fn identity(_path: &Path) -> std::io::Result<(u64, u64)> {
	Err(std::io::Error::other("a named pipe has no file identity"))
}

#[cfg(unix)]
pub fn answers(path: &Path) -> bool {
	std::os::unix::net::UnixStream::connect(path).is_ok()
}

// ponytail: Windows leaves nothing to distinguish without the pipe peer, so a
// loose file there is never swept; the directory holds the pipe names anyway.
#[cfg(windows)]
pub fn answers(_path: &Path) -> bool {
	true
}

/// A replacing host has staged `host.sock.<pid>` beside the name and not yet
/// renamed it into place: the name may be missing for a moment, and nobody
/// should start a competitor meanwhile.
#[cfg(unix)]
pub fn takeover_pending(descriptor: &Path) -> Result<bool> {
	let run = run_dir(descriptor)?;
	let Ok(entries) = std::fs::read_dir(&run) else {
		return Ok(false);
	};
	Ok(entries.flatten().any(|entry| {
		entry
			.file_name()
			.to_str()
			.and_then(|n| n.strip_prefix("host.sock."))
			.and_then(|pid| pid.parse::<u32>().ok())
			.is_some_and(|pid| alive(pid) && answers(&entry.path()))
	}))
}

#[cfg(windows)]
pub fn takeover_pending(_descriptor: &Path) -> Result<bool> {
	Ok(false)
}

#[cfg(unix)]
pub(crate) fn alive(pid: u32) -> bool {
	let exists = unsafe { libc::kill(pid as libc::pid_t, 0) } == 0;
	exists || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

// ponytail: no liveness check on Windows, so nothing is swept there; a dead
// run's directory stays until the user clears it.
#[cfg(windows)]
pub(crate) fn alive(_pid: u32) -> bool {
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

/// Held across a bind and across a sweep of one directory. A socket is bound
/// before it listens, and a connect in between is refused, so without this a
/// second host (or a sweep) takes that for a stale file, unlinks the name and
/// binds its own: two daemons, each thinking it serves the project.
// ponytail: a blocking flock on the calling thread; it is held for a bind or a
// directory scan, microseconds, so no spawn_blocking.
#[cfg(unix)]
fn lock_names(dir: &Path) -> Option<std::fs::File> {
	use std::os::unix::fs::OpenOptionsExt;
	use std::os::unix::io::AsRawFd;
	let file = std::fs::OpenOptions::new()
		.create(true)
		.truncate(false)
		.write(true)
		.mode(0o600)
		.open(dir.join("host.lock"))
		.ok()?;
	(unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0).then_some(file)
}

#[cfg(windows)]
fn lock_names(_dir: &Path) -> Option<std::fs::File> {
	None
}

pub(crate) async fn listen(path: &Path) -> Result<crate::transport::typed::LocalListener> {
	let _names = path.parent().and_then(lock_names);
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

#[derive(Clone, PartialEq, Eq)]
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
			// ponytail: a node that omits its token passes as the command line; bind
			// the node role to the peer instead if nodes ever run untrusted.
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
	// Only a client keeps the host busy; a node is the host's own child.
	let client = caller == Caller::Host;
	if client {
		host.client_joined();
	}
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
	if client {
		host.client_left();
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
		(Caller::Host, _)
			| (
				Caller::Cartridge,
				"status" | "snapshot" | "cartridges" | "asp" | "event"
			)
	);
	if !granted {
		let message = format!("`{method}` is not granted to this token");
		return request.reply(Err(rpc::Error::new(rpc::UNAUTHORIZED, message)));
	}
	match method.as_str() {
		"status" => request.reply(Ok(json!(host.status()))),
		"snapshot" => request.reply(Ok(host.snapshot())),
		"cartridges" => request.reply(Ok(host.cartridges())),
		// A cartridge asks ASP what it may know. Running an action is a tool
		// call, and a cartridge makes those through its own dispatch, where
		// policy is asked; only the command line runs one from here.
		"asp" if caller == Caller::Cartridge && params["op"] == "act" => request.reply(Err(
			rpc::Error::new(rpc::UNAUTHORIZED, "`act` is not granted to this token"),
		)),
		"asp" => request.reply(host.asp(params).await.map_err(application)),
		"event" if params["name"] == crate::asp::TOOL => request.reply(
			host.asp_tool(params["data"].clone())
				.await
				.map_err(application),
		),
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
