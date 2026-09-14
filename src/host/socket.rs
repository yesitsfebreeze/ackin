//! Where sockets live, and the host's own socket: the same JSON-RPC wire the
//! cartridges speak, for the command line.
//!
//! Methods, after `auth {token}`. A cartridge's token (from its directory) is
//! granted `status`, `snapshot` and `cartridges`, and `bridge.*` where its
//! profile entry sets `bridge = true`; the host token is granted everything.
//!   status                        -> [{id, state, error, waiting, provide, needs, on, socket}]
//!   snapshot                      -> {host_pid, profile, cartridge_root, entries}
//!   cartridges                    -> [{id, dir}]
//!   bridge.status                 -> [{id, generation, module, services}]
//!   bridge.call {owner, generation, key, args}
//!   call {key, args, trace?}      -> the provider's answer
//!   emit {name, data}             -> [{from, data} | {from, error}]
//!   reload {cartridge?}           -> {}
//!   subscribe {channel, since?}   -> {}; `lifecycle`, or `<cartridge>.<channel>`
//!   stop                          -> {}

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::sync::mpsc;
use transport::rpc::{self, Incoming, Peer, Request};

use crate::error::{Error, Result};

use super::Host;

/// The per-user directory every host of this user keeps its sockets in.
pub fn base() -> Result<PathBuf> {
	#[cfg(unix)]
	{
		use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
		// SAFETY: `getuid` cannot fail and touches no memory.
		let uid = unsafe { libc::getuid() };
		let base = std::env::var_os("XDG_RUNTIME_DIR")
			.map(|dir| PathBuf::from(dir).join("cartridge"))
			.filter(|dir| dir.as_os_str().len() < 48)
			.unwrap_or_else(|| PathBuf::from(format!("/tmp/cartridge-{uid}")));
		std::fs::DirBuilder::new()
			.recursive(true)
			.mode(0o700)
			.create(&base)
			.map_err(|e| Error::file(&base, e))?;
		let meta = std::fs::symlink_metadata(&base).map_err(|e| Error::file(&base, e))?;
		if !meta.is_dir() || meta.uid() != uid {
			return Err(Error::Profile(format!(
				"{} is not a directory owned by this user",
				base.display()
			)));
		}
		if meta.permissions().mode() & 0o077 != 0 {
			std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o700))
				.map_err(|e| Error::file(&base, e))?;
		}
		Ok(base)
	}
}

fn tag(profile: &Path) -> String {
	transport::typed::path_tag(profile)[..12].to_owned()
}

/// The host socket of the project whose profile is `profile`.
pub fn path(profile: &Path) -> Result<PathBuf> {
	Ok(base()?.join(format!("{}.sock", tag(profile))))
}

fn token_path(profile: &Path) -> Result<PathBuf> {
	Ok(base()?.join(format!("{}.token", tag(profile))))
}

/// A directory for the cartridge sockets of one host run. Directories of runs
/// whose process is gone are removed.
pub(crate) fn run_dir(profile: &Path) -> Result<PathBuf> {
	let base = base()?;
	let prefix = format!("{}-", tag(profile));
	if let Ok(read) = std::fs::read_dir(&base) {
		for entry in read.flatten() {
			let name = entry.file_name().to_string_lossy().into_owned();
			let Some(pid) = name
				.strip_prefix(&prefix)
				.and_then(|pid| pid.parse::<i32>().ok())
			else {
				continue;
			};
			// SAFETY: signal 0 only checks that the process exists.
			if unsafe { libc::kill(pid, 0) } != 0 {
				let _ = std::fs::remove_dir_all(entry.path());
			}
		}
	}
	let dir = base.join(format!("{prefix}{}", std::process::id()));
	std::fs::create_dir_all(&dir).map_err(|e| Error::file(&dir, e))?;
	Ok(dir)
}

/// A socket file name for an entry id: readable, and unique even when ids differ only in punctuation.
pub(crate) fn file_name(id: &str) -> String {
	let readable: String = id
		.chars()
		.map(|c| {
			if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
				c
			} else {
				'_'
			}
		})
		.take(24)
		.collect();
	let hash = transport::typed::path_tag(Path::new(id));
	format!("{readable}-{}.sock", &hash[..6])
}

pub(crate) async fn listen(path: &Path) -> Result<transport::typed::LocalListener> {
	let _ = std::fs::remove_file(path);
	match transport::typed::bind(&transport::typed::Endpoint::Unix(path.to_path_buf())).await {
		Ok(transport::typed::BindOutcome::Bound(listener)) => Ok(listener),
		Ok(transport::typed::BindOutcome::AlreadyRunning) => Err(Error::Remote(format!(
			"{} is already served",
			path.display()
		))),
		Err(error) => Err(Error::Remote(format!("{}: {error}", path.display()))),
	}
}

/// Answer cartridges on the host's inner socket for as long as the host lives.
pub(crate) async fn accept(
	host: std::sync::Weak<Host>,
	mut listener: transport::typed::LocalListener,
) {
	while let Ok(adapter) = listener.accept().await {
		let Some(host) = host.upgrade() else { break };
		let (stop, _) = mpsc::channel(1);
		tokio::spawn(connection(host, adapter, stop));
	}
}

/// Serve the project's host socket for the command line until asked to stop.
/// Returns `Ok(false)` when another host already serves this project.
pub async fn serve(host: Arc<Host>) -> Result<bool> {
	let path = path(&host.profile)?;
	let mut listener =
		match transport::typed::bind(&transport::typed::Endpoint::Unix(path.clone())).await {
			Ok(transport::typed::BindOutcome::Bound(listener)) => listener,
			Ok(transport::typed::BindOutcome::AlreadyRunning) => return Ok(false),
			Err(error) => return Err(Error::Remote(format!("{}: {error}", path.display()))),
		};
	let token = token_path(&host.profile)?;
	write_private(&token, host.host_token())?;
	let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
	loop {
		tokio::select! {
			accepted = listener.accept() => {
				let Ok(adapter) = accepted else { break };
				tokio::spawn(connection(host.clone(), adapter, stop_tx.clone()));
			}
			_ = stop_rx.recv() => break,
		}
	}
	let _ = std::fs::remove_file(&token);
	Ok(true)
}

fn write_private(path: &Path, text: &str) -> Result<()> {
	use std::io::Write;
	use std::os::unix::fs::OpenOptionsExt;
	let _ = std::fs::remove_file(path);
	let mut file = std::fs::OpenOptions::new()
		.write(true)
		.create_new(true)
		.mode(0o600)
		.open(path)
		.map_err(|e| Error::file(path, e))?;
	file.write_all(text.as_bytes())
		.map_err(|e| Error::file(path, e))
}

#[derive(Clone)]
enum Caller {
	Host,
	Cartridge(String),
}

async fn connection(
	host: Arc<Host>,
	adapter: transport::typed::LocalAdapter,
	stop: mpsc::Sender<()>,
) {
	let (peer, mut incoming) = Peer::spawn(adapter, Some(1 << 26));
	let caller = match incoming.recv().await {
		Some(Incoming::Request(request)) if request.method == "auth" => {
			let token = request.params["token"].as_str().unwrap_or_default();
			let caller = match token == host.host_token() {
				true => Some(Caller::Host),
				false => host.caller(token).map(Caller::Cartridge),
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
	stop: mpsc::Sender<()>,
) -> futures::future::BoxFuture<'static, ()> {
	Box::pin(answer(host, peer, caller, request, stop))
}

async fn answer(
	host: Arc<Host>,
	peer: Peer,
	caller: Caller,
	request: Request,
	stop: mpsc::Sender<()>,
) {
	let params = request.params.clone();
	let application = |e: Error| rpc::Error::application(e.to_string());
	let method = request.method.clone();
	let granted = match (&caller, method.as_str()) {
		(Caller::Host, _) => true,
		(Caller::Cartridge(_), "status" | "snapshot" | "cartridges") => true,
		(Caller::Cartridge(id), "bridge.status" | "bridge.call") => host.bridged(id),
		_ => false,
	};
	if !granted {
		let message = format!("`{method}` is not granted to this token");
		return request.reply(Err(rpc::Error::new(rpc::UNAUTHORIZED, message)));
	}
	match method.as_str() {
		"status" => request.reply(Ok(json!(host.status()))),
		"snapshot" => request.reply(Ok(host.snapshot())),
		"cartridges" => request.reply(Ok(host.cartridges())),
		"bridge.status" => request.reply(Ok(host.bridge_status())),
		"bridge.call" => {
			let result = host
				.bridge_call(
					params["owner"].as_str().unwrap_or_default(),
					params["generation"].as_u64().unwrap_or_default(),
					params["key"].as_str().unwrap_or_default(),
					params["args"].clone(),
				)
				.await;
			request.reply(result.map_err(application));
		}
		"call" => {
			let key = params["key"].as_str().unwrap_or_default().to_owned();
			let result = host.call(&key, params["args"].clone()).await;
			request.reply(result.map_err(application));
		}
		"emit" => {
			let name = params["name"].as_str().unwrap_or_default().to_owned();
			let answers: Vec<Value> = host
				.emit(&name, params["data"].clone())
				.await
				.into_iter()
				.map(|(from, answer)| match answer {
					Ok(data) => json!({ "from": from, "data": data }),
					Err(error) => json!({ "from": from, "error": error }),
				})
				.collect();
			request.reply(Ok(json!(answers)));
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
			let _ = stop.send(()).await;
		}
		other => request.reply(Err(rpc::Error::new(
			rpc::METHOD_NOT_FOUND,
			format!("unknown method `{other}`"),
		))),
	}
}

/// Forward a channel to the connection as `channel` notifications.
async fn follow(host: &Arc<Host>, peer: &Peer, channel: &str, since: Option<u64>) -> Result<()> {
	if channel == "lifecycle" {
		let mut events = host.lifecycle();
		let peer = peer.clone();
		tokio::spawn(async move {
			while let Ok(event) = events.recv().await {
				if peer
					.notify(
						"channel",
						json!({ "channel": "lifecycle", "kind": "data", "data": event }),
					)
					.is_err()
				{
					break;
				}
			}
		});
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
		while let Some(message) = incoming.recv().await {
			if let Incoming::Notification { mut params, .. } = message {
				params["channel"] = json!(full);
				if peer.notify("channel", params).is_err() {
					break;
				}
			}
		}
		drop(upstream);
	});
	Ok(())
}

/// A connection to the host serving `profile`.
pub async fn client(profile: &Path) -> Result<(Peer, mpsc::UnboundedReceiver<Incoming>)> {
	let token = std::fs::read_to_string(token_path(profile)?).map_err(|e| Error::Unavailable {
		key: "host".into(),
		why: format!("no host serves {}: {e}", profile.display()),
	})?;
	super::connect(&path(profile)?, token.trim()).await
}
