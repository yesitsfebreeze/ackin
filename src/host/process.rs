//! A node: the base binary re-run on a cartridge's `init.lua`, inside its
//! grant, reached once it serves, applied, and stopped.

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::oneshot;

use crate::error::{Error, Result};
use crate::transport::cartridge::{Directory, CONNECT_TIMEOUT_ENV, NODE_TOKEN_ENV, SOCKET_ENV};

use super::{Host, Plan, Running};

/// The binary a node runs as; this one unless a test names the built CLI.
pub const NODE_BIN_ENV: &str = "CARTRIDGE_NODE_BIN";

/// What a node may inherit from the base's environment: nothing it needs to
/// do its job is missing, and nothing it needs not to see is present. The six
/// `CARTRIDGE_*` variables are set explicitly below. `PATH` resolves the
/// helpers `grant.exec` names; `HOME` reaches the global `config.lua` and the
/// trust store; `XDG_RUNTIME_DIR` names the sockets base; the rest keep a
/// node's timestamps, messages and temp files from changing with the base's
/// shell. Everything else — secrets the person's shell held — is dropped.
const PASSTHROUGH: &[&str] = &[
	"PATH",
	"HOME",
	"TMPDIR",
	"TZ",
	"LANG",
	"LC_ALL",
	"USER",
	"LOGNAME",
	"SHELL",
	"XDG_RUNTIME_DIR",
];

pub(super) async fn start(
	host: &Arc<Host>,
	plan: &Plan,
	directory: &Directory,
	generation: u64,
) -> Result<Running> {
	let settings = crate::settings::host();
	// The node's own credential, minted per start: worth that node's socket
	// only, never the host's. `replace` starts the same id again and overwrites.
	let token = crate::transport::token();
	host.node_tokens
		.lock()
		.insert(plan.id.clone(), token.clone());
	let socket = host.socket(&plan.id);
	let _ = std::fs::remove_file(&socket);
	let sockets = socket.parent().map(std::path::Path::to_path_buf);
	let exe = match std::env::var_os(NODE_BIN_ENV) {
		Some(exe) => std::path::PathBuf::from(exe),
		None => std::env::current_exe().map_err(|e| Error::file("cartridge", e))?,
	};
	let command = vec![exe.to_string_lossy().into_owned(), "node".to_owned()];
	let mut child = tokio::process::Command::from(
		crate::sandbox::command(&command, &plan.grant, &plan.root, sockets.as_deref())
			.map_err(|e| Error::process(&plan.id, e))?,
	)
	.env_clear()
	.envs(
		PASSTHROUGH
			.iter()
			.filter_map(|key| std::env::var_os(key).map(|value| (key, value))),
	)
	.env(SOCKET_ENV, &socket)
	.env(NODE_TOKEN_ENV, &token)
	.env(
		CONNECT_TIMEOUT_ENV,
		settings.startup_timeout_secs.to_string(),
	)
	.env(crate::node::ENTRY_ENV, &plan.entry)
	.env(crate::node::ENTRY_SHA256_ENV, &plan.entry_sha256)
	.env(crate::node::ROOT_ENV, &plan.root)
	.env(
		crate::node::LISTEN_ENV,
		serde_json::to_string(&plan.listen)?,
	)
	.stdin(Stdio::piped())
	.stdout(Stdio::null())
	.stderr(Stdio::piped())
	.process_group(0)
	.kill_on_drop(true)
	.spawn()
	.map_err(|e| Error::process(&plan.id, e))?;
	let group = Group(
		child
			.id()
			.ok_or_else(|| Error::process(&plan.id, "exited as it was spawned"))? as libc::pid_t,
	);
	let stdin = child.stdin.take();
	let tail = Arc::new(std::sync::Mutex::new(
		std::collections::VecDeque::<String>::new(),
	));
	if let Some(stderr) = child.stderr.take() {
		let (id, tail) = (plan.id.clone(), tail.clone());
		tokio::spawn(async move {
			let mut lines = BufReader::new(stderr).lines();
			while let Ok(Some(line)) = lines.next_line().await {
				crate::trace::diagnostic_line(&id, &line);
				let mut tail = tail.lock().expect("stderr tail");
				tail.push_back(line);
				if tail.len() > 20 {
					tail.pop_front();
				}
			}
		});
	}
	let said = move || {
		let tail = tail.lock().expect("stderr tail");
		match tail.is_empty() {
			true => String::new(),
			false => format!(": {}", tail.iter().cloned().collect::<Vec<_>>().join(" | ")),
		}
	};
	let deadline = tokio::time::Instant::now() + settings.startup_timeout();
	let (peer, _incoming) = loop {
		if let Some(status) = child.try_wait()? {
			tokio::time::sleep(Duration::from_millis(50)).await;
			return Err(Error::process(
				&plan.id,
				format!("exited before serving: {status}{}", said()),
			));
		}
		if socket.exists() {
			if let Ok(connected) = super::connect(&socket, &token).await {
				break connected;
			}
		}
		if tokio::time::Instant::now() >= deadline {
			let _ = child.kill().await;
			return Err(Error::Timeout(plan.id.clone()));
		}
		tokio::time::sleep(Duration::from_millis(20)).await;
	};
	let apply = peer.call(
		"apply",
		json!({ "name": plan.name, "config": plan.config, "directory": directory }),
	);
	match tokio::time::timeout(settings.startup_timeout(), apply).await {
		Ok(Ok(_)) => {}
		Ok(Err(error)) => {
			let _ = child.kill().await;
			return Err(Error::process(
				&plan.id,
				format!("{}{}", error.message, said()),
			));
		}
		Err(_) => {
			let _ = child.kill().await;
			return Err(Error::Timeout(plan.id.clone()));
		}
	}
	let (stop_tx, stop_rx) = oneshot::channel::<()>();
	let (id, weak) = (plan.id.clone(), Arc::downgrade(host));
	let monitor = tokio::spawn(async move {
		let _group = group;
		let exited = tokio::select! {
			status = child.wait() => Some(status),
			_ = stop_rx => None,
		};
		match exited {
			Some(status) => {
				if let Some(host) = weak.upgrade() {
					let why = match status {
						Ok(status) => format!("exited: {status}"),
						Err(error) => format!("lost: {error}"),
					};
					host.exited(&id, generation, why);
				}
			}
			None => {
				drop(stdin);
				let shutdown = crate::settings::host().shutdown_timeout();
				if tokio::time::timeout(shutdown, child.wait()).await.is_err() {
					let _ = child.kill().await;
				}
			}
		}
		let _ = std::fs::remove_file(&socket);
	});
	Ok(Running {
		peer,
		stop: Box::new(move |peer| {
			Box::pin(async move {
				let shutdown = crate::settings::host().shutdown_timeout();
				let _ = tokio::time::timeout(shutdown, peer.call("dispose", json!({}))).await;
				let _ = stop_tx.send(());
				let _ = monitor.await;
			})
		}),
	})
}

/// A node's process group: the node leads it and what it spawns joins it.
/// Dropped, it kills every member; killing the node alone leaves its programs
/// running with the node's stderr open.
struct Group(libc::pid_t);

impl Drop for Group {
	fn drop(&mut self) {
		// SAFETY: killpg only sends a signal. The id names no other group while a
		// member lives, and the guard drops as soon as its leader is reaped.
		unsafe { libc::killpg(self.0, libc::SIGKILL) };
	}
}
