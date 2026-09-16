use std::process::ExitCode;
use std::sync::Arc;

use cartridge::host::{socket, Host};
use cartridge::{Error, Result};
use serde_json::{json, Value};

use super::{client, fail, Project, FAILED};

pub(crate) fn host(project: &Project) -> Result<Arc<Host>> {
	let host = Host::new(&project.dir, &project.descriptor)?;
	stop_on_signals(host.stop_signal())?;
	Ok(host)
}

fn stop_on_signals(stop: tokio_util::sync::CancellationToken) -> Result<()> {
	#[cfg(unix)]
	let (mut interrupt, mut terminate) = {
		use tokio::signal::unix::{signal, SignalKind};
		(
			signal(SignalKind::interrupt())?,
			signal(SignalKind::terminate())?,
		)
	};
	#[cfg(windows)]
	let (mut interrupt, mut terminate) = {
		use tokio::signal::windows::{ctrl_c, ctrl_close};
		(ctrl_c()?, ctrl_close()?)
	};
	tokio::spawn(async move {
		tokio::select! {
			_ = terminate.recv() => {}
			_ = interrupt.recv() => {}
		}
		stop.cancel();
	});
	Ok(())
}

/// Keep running under the terminal's interrupt: it is meant for the agent in
/// the foreground, which shares this process group and gets it directly.
fn ignore_interrupt() -> Result<()> {
	#[cfg(unix)]
	{
		use tokio::signal::unix::{signal, SignalKind};
		let mut interrupt = signal(SignalKind::interrupt())?;
		tokio::spawn(async move { while interrupt.recv().await.is_some() {} });
	}
	Ok(())
}

fn serve_beside(host: &Arc<Host>) -> tokio::task::JoinHandle<()> {
	let host = host.clone();
	tokio::spawn(async move {
		if let Err(error) = socket::serve(host).await {
			tracing::warn!(target: "cartridge", "host socket: {error}");
		}
	})
}

type Attached = (
	cartridge::transport::rpc::Peer,
	tokio::sync::mpsc::Receiver<cartridge::transport::rpc::Incoming>,
);

/// The one host of this project, started detached when none answers, and
/// its composition settled before the connection is handed back.
pub(crate) async fn attach(project: &Project) -> Result<Attached> {
	let settings = cartridge::settings::host();
	let mut spawned = false;
	let started = tokio::time::Instant::now();
	let deadline = started + settings.startup_timeout();
	// A host replacing another takes the name back in a rename; this grace is
	// long enough that a command arriving mid-swap waits for it instead of
	// starting a competitor.
	let grace = std::time::Duration::from_millis(1500);
	loop {
		if let Some(found) = client::served(project).await {
			settle_remote(&found.0, settings.verify_timeout()).await;
			return Ok(found);
		}
		if !spawned && started.elapsed() >= grace {
			spawn_daemon(project)?;
			spawned = true;
		}
		if tokio::time::Instant::now() >= deadline {
			return Err(Error::Timeout(format!(
				"no host answered for {} within {}s; see {}",
				project.descriptor.display(),
				settings.startup_timeout_secs,
				daemon_log(project).display()
			)));
		}
		tokio::time::sleep(std::time::Duration::from_millis(100)).await;
	}
}

fn daemon_log(project: &Project) -> std::path::PathBuf {
	project.descriptor.join("daemon.log")
}

/// A host of its own: not this process's child, so it outlives the command
/// and the terminal, and every later command attaches to it.
fn spawn_daemon(project: &Project) -> Result<()> {
	let exe = std::env::current_exe().map_err(|e| Error::file("cartridge", e))?;
	let log = daemon_log(project);
	let file = std::fs::OpenOptions::new()
		.create(true)
		.append(true)
		.open(&log)
		.map_err(|e| Error::file(&log, e))?;
	let mut command = std::process::Command::new(&exe);
	command
		.arg("--dir")
		.arg(&project.dir)
		.arg("daemon")
		.stdin(std::process::Stdio::null())
		.stdout(file.try_clone().map_err(|e| Error::file(&log, e))?)
		.stderr(file);
	#[cfg(unix)]
	{
		use std::os::unix::process::CommandExt;
		command.process_group(0);
	}
	command
		.spawn()
		.map_err(|e| Error::process(exe.to_string_lossy().as_ref(), e))?;
	eprintln!(
		"starting the host for {} (log: {})",
		project.descriptor.display(),
		log.display()
	);
	Ok(())
}

/// Wait until the remote host's cartridges stop changing state: none
/// starting, and the same picture three polls in a row.
async fn settle_remote(peer: &cartridge::transport::rpc::Peer, timeout: std::time::Duration) {
	let deadline = tokio::time::Instant::now() + timeout;
	let mut last = Value::Null;
	let mut stable = 0;
	while tokio::time::Instant::now() < deadline {
		let Ok(status) = peer.call("status", Value::Null).await else {
			return;
		};
		let starting = status.as_array().is_some_and(|all| {
			!all.is_empty()
				&& all
					.iter()
					.any(|s| matches!(s["state"].as_str(), Some("starting" | "waiting")))
		});
		let empty = status.as_array().is_none_or(Vec::is_empty);
		if !starting && !empty {
			return;
		}
		stable = if status == last { stable + 1 } else { 0 };
		if stable >= 3 && !empty {
			return;
		}
		last = status;
		tokio::time::sleep(std::time::Duration::from_millis(100)).await;
	}
}

pub(crate) async fn run(project: &Project, key: &str, args: Value) -> Result<ExitCode> {
	if let Some((peer, _incoming)) = client::served(project).await {
		settle_remote(&peer, cartridge::settings::host().verify_timeout()).await;
		let value = client::bail(&peer, key, args).await?;
		if !value.is_null() {
			println!("{value}");
		}
		return Ok(ExitCode::SUCCESS);
	}
	let host = host(project)?.private();
	let served = serve_beside(&host);
	let result = host.run(key, args).await;
	served.abort();
	match result? {
		value if !value.is_null() => println!("{value}"),
		_ => {}
	}
	Ok(ExitCode::SUCCESS)
}

pub(crate) async fn launch(
	project: &Project,
	agent: String,
	model: String,
	args: Vec<String>,
) -> Result<ExitCode> {
	let request = json!({ "op": "launch", "agent": agent, "model": model, "args": args });
	let (peer, _incoming) = attach(project).await?;
	let launch = client::bail(&peer, "proxy", request).await?;
	ignore_interrupt()?;
	let status = spawn(launch).await?;
	Ok(ExitCode::from(
		status.as_i64().unwrap_or(1).clamp(0, 255) as u8
	))
}

async fn spawn(launch: Value) -> Result<Value> {
	let strings = |v: &Value| -> Vec<String> {
		v.as_array()
			.into_iter()
			.flatten()
			.filter_map(Value::as_str)
			.map(str::to_owned)
			.collect()
	};
	let program = launch["program"]
		.as_str()
		.ok_or(Error::Invalid("launch without program"))?;
	let mut command = tokio::process::Command::new(program);
	command.args(strings(&launch["args"]));
	if let Some(cwd) = launch["cwd"].as_str() {
		command.current_dir(cwd);
	}
	for name in strings(&launch["unset"]) {
		command.env_remove(name);
	}
	if let Some(env) = launch["env"].as_object() {
		for (k, v) in env {
			command.env(k, v.as_str().unwrap_or_default());
		}
	}
	command.kill_on_drop(true);
	let status = command
		.status()
		.await
		.map_err(|e| Error::process(program, e))?;
	Ok(json!(status.code().unwrap_or(1)))
}

pub(crate) async fn mcp(project: &Project) -> Result<ExitCode> {
	let attached = attach(project).await?;
	stdio(Backend {
		project: project.clone(),
		attached: Arc::new(tokio::sync::Mutex::new(attached)),
	})
	.await?;
	Ok(ExitCode::SUCCESS)
}

/// The host this bridge talks to. A host replaced under it (`daemon
/// --replace`, a rebuilt binary) closes the connection; the next message
/// attaches again and is sent once more.
#[derive(Clone)]
struct Backend {
	project: Project,
	attached: Arc<tokio::sync::Mutex<Attached>>,
}

impl Backend {
	async fn message(&self, line: &str) -> Result<Value> {
		let data = json!({ "op": "message", "line": line });
		let peer = self.attached.lock().await.0.clone();
		match client::bail(&peer, "mcp", data.clone()).await {
			Err(_) if peer.is_closed() => {
				let mut attached = self.attached.lock().await;
				if attached.0.is_closed() {
					*attached = attach(&self.project).await?;
				}
				let peer = attached.0.clone();
				drop(attached);
				client::bail(&peer, "mcp", data).await
			}
			other => other,
		}
	}
}

/// Never print to stdout here: it carries the protocol.
async fn stdio(backend: Backend) -> Result<Value> {
	use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
	let (replies, mut pending) =
		tokio::sync::mpsc::channel::<String>(cartridge::settings::host().mcp_reply_queue);
	let writer = tokio::spawn(async move {
		let mut out = tokio::io::stdout();
		while let Some(line) = pending.recv().await {
			if out.write_all(line.as_bytes()).await.is_err() || out.flush().await.is_err() {
				break;
			}
		}
	});
	let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
	let mut serving = tokio::task::JoinSet::new();
	while let Ok(Some(line)) = lines.next_line().await {
		if line.trim().is_empty() {
			continue;
		}
		while serving.try_join_next().is_some() {}
		let (backend, replies) = (backend.clone(), replies.clone());
		serving.spawn(async move {
			let result = backend.message(&line).await;
			if let Some(reply) = mcp_bridge_reply(&line, result) {
				let _ = replies.send(format!("{reply}\n")).await;
			}
		});
	}
	while serving.join_next().await.is_some() {}
	drop(replies);
	let _ = writer.await;
	Ok(Value::Null)
}

fn mcp_bridge_reply(line: &str, result: Result<Value>) -> Option<Value> {
	match result {
		Ok(reply) => (!reply.is_null()).then_some(reply),
		Err(error) => {
			tracing::warn!(target: "cartridge", "mcp: {error}");
			let message: Value = match serde_json::from_str(line) {
				Ok(message) => message,
				Err(_) => {
					return Some(
						json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"parse error"}}),
					)
				}
			};
			message.get("method")?.as_str()?;
			let id = message.get("id").filter(|id| !id.is_null())?;
			Some(
				json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":error.to_string()}}),
			)
		}
	}
}

pub(crate) async fn daemon(project: &Project, replace: bool) -> Result<ExitCode> {
	let existing = client::served(project).await;
	if existing.is_some() && !replace {
		return Err(Error::Descriptor(format!(
			"a host already serves {}; `cartridge daemon --replace` takes over from it",
			project.descriptor.display()
		)));
	}
	let host = host(project)?;
	match existing {
		Some((old, _incoming)) => {
			if let Err(error) = host.takeover(&old).await {
				host.stop().await;
				return Err(error);
			}
		}
		None => {
			if let Err(error) = host.reconcile().await {
				tracing::error!(target: "cartridge", cartridge = "init.lua", "{error}");
			}
		}
	}
	let watcher = host.watch();
	if let Err(error) = &watcher {
		tracing::error!(target: "cartridge", "watch: {error}");
	}
	tracing::info!(
		target: "cartridge",
		dir = %project.dir.display(),
		descriptor = %project.descriptor.display(),
		socket = %socket::path(&project.descriptor)?.display(),
		"serving"
	);
	host.stopped().await;
	if let Ok(watcher) = watcher {
		watcher.abort();
	}
	host.stop().await;
	Ok(ExitCode::SUCCESS)
}

pub(crate) async fn verify(project: &Project, cartridge: Option<&str>) -> Result<ExitCode> {
	let host = host(project)?.private();
	let (ran, failures) = match cartridge {
		Some(one) => host.verify_one(one).await?,
		None => host.verify().await?,
	};
	if failures.is_empty() {
		println!("{ran} contracts passed");
		return Ok(ExitCode::SUCCESS);
	}
	Ok(fail(FAILED, failures.join("\n")))
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/stdio.rs"]
mod stdio_tests;

#[cfg(all(test, unix))]
#[path = "../../.cartridge/tests/unit/src/cli/signals.rs"]
mod signal_tests;
