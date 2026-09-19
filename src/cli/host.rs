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

type Attached = (
	cartridge::transport::rpc::Peer,
	tokio::sync::mpsc::Receiver<cartridge::transport::rpc::Incoming>,
);

/// The one host of this project, started detached when none answers, and
/// its composition settled before the connection is handed back. `key` is
/// the event this caller is about to send: the wait ends when a cartridge
/// serving it is active, not when the picture merely stops changing.
pub(crate) async fn attach(project: &Project, key: &str) -> Result<Attached> {
	let settings = cartridge::settings::host();
	let mut spawned = false;
	let started = tokio::time::Instant::now();
	let deadline = started + settings.startup_timeout();
	// A host replacing another takes the name back in a rename; this grace is
	// long enough that a command arriving mid-swap waits for it instead of
	// starting a competitor.
	let grace = std::time::Duration::from_millis(1500);
	loop {
		// A socket that answers but errors is retried through the grace, as a
		// host mid-swap does; nothing is spawned while it answers or while a
		// replacement is staged beside it.
		match client::served(project).await {
			Ok(Some(found)) => {
				// `startup_timeout_secs` is the budget a cartridge already has
				// to serve and apply, so it is the composition's startup
				// deadline too; `verify_timeout_secs` (300 s) is `verify`'s.
				settle_remote(&found.0, key, settings.startup_timeout()).await;
				return Ok(found);
			}
			Err(error) => {
				if started.elapsed() >= grace && !socket::takeover_pending(&project.descriptor)? {
					return Err(client::unanswered(project, error));
				}
			}
			Ok(None) => {
				if !spawned
					&& started.elapsed() >= grace
					&& !socket::takeover_pending(&project.descriptor)?
				{
					spawn_daemon(project)?;
					spawned = true;
				}
			}
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

/// Stop a host nothing is using. A background host outlives the command that
/// started it on purpose — the next one attaches to it warm — but a project
/// that was a temporary directory has no next command, and what is left is a
/// node per cartridge serving nobody. The next command starts one again, so
/// the only cost of being wrong is one cold start.
fn stop_when_idle(
	host: &Arc<Host>,
	timeout: std::time::Duration,
) -> Option<tokio::task::JoinHandle<()>> {
	if timeout.is_zero() {
		return None;
	}
	let (host, stop) = (Arc::downgrade(host), host.stop_signal());
	Some(tokio::spawn(async move {
		// Checked often enough that the stop lands near the timeout, rarely
		// enough that an idle host costs nothing to keep watching.
		let mut tick = tokio::time::interval(timeout.min(std::time::Duration::from_secs(30)));
		loop {
			tick.tick().await;
			let Some(host) = host.upgrade() else { return };
			if host.idle_for() >= timeout {
				tracing::info!(
					target: "cartridge",
					seconds = timeout.as_secs(),
					"no client for the idle timeout; stopping"
				);
				stop.cancel();
				return;
			}
		}
	}))
}

fn daemon_log(project: &Project) -> std::path::PathBuf {
	project.descriptor.join("daemon.log")
}

/// A host of its own: not this process's child, so it outlives the command
/// and the terminal, and every later command attaches to it.
fn spawn_daemon(project: &Project) -> Result<()> {
	let exe = std::env::current_exe().map_err(|e| Error::file("cartridge", e))?;
	std::fs::create_dir_all(&project.descriptor)
		.map_err(|e| Error::file(&project.descriptor, e))?;
	let log = daemon_log(project);
	let file = std::fs::OpenOptions::new()
		.create(true)
		.append(true)
		.open(&log)
		.map_err(|e| Error::file(&log, e))?;
	let mut command = std::process::Command::new(&exe);
	if cartridge::settings::yolo() {
		command.arg("--yolo");
	}
	command
		.arg("--dir")
		.arg(&project.dir)
		.arg("daemon")
		.arg("--idle-timeout")
		.arg(cartridge::settings::host().idle_timeout_secs.to_string())
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

/// Wait until a cartridge serving `key` is active, or nothing is left
/// starting, or `timeout` passes. A repeated identical picture is not an
/// answer: a cold host holds one for a few hundred milliseconds while its
/// cartridges are still starting, and the caller's listener is not up yet.
async fn settle_remote(
	peer: &cartridge::transport::rpc::Peer,
	key: &str,
	timeout: std::time::Duration,
) {
	let deadline = tokio::time::Instant::now() + timeout;
	let mut reloaded = false;
	while let Some(left) = deadline.checked_duration_since(tokio::time::Instant::now()) {
		// A host that answers its socket but never its status would hang this
		// loop on an untimed oneshot, so the deadline bounds each call too.
		let Ok(Ok(status)) = tokio::time::timeout(left, peer.call("status", Value::Null)).await
		else {
			return;
		};
		// A daemon that refused a changed file keeps the cartridge failed after
		// the file is trusted (`--yolo`, `cartridge trust`); one reload lets it
		// read the new trust instead of serving without the cartridge.
		if !reloaded && !untrusted(&status).is_empty() {
			reloaded = true;
			if peer
				.call("reload", json!({ "cartridge": null }))
				.await
				.is_ok()
			{
				continue;
			}
		}
		if settled(&status, key) {
			return;
		}
		tokio::time::sleep(std::time::Duration::from_millis(100)).await;
	}
}

/// Whether the wait can end on this status: a cartridge serving `key` is
/// active, or nothing is left starting and waiting longer cannot change that.
fn settled(status: &Value, key: &str) -> bool {
	let Some(all) = status.as_array().filter(|all| !all.is_empty()) else {
		return false;
	};
	let serves = all.iter().any(|s| {
		s["state"] == "active"
			&& s["listen"]
				.as_array()
				.is_some_and(|l| l.iter().any(|k| k == key))
	});
	// Nothing left to start and `key` still unserved: waiting out the deadline
	// would not change that, so let the caller fail and name it.
	serves
		|| !all
			.iter()
			.any(|s| matches!(s["state"].as_str(), Some("starting" | "waiting")))
}

/// Failed cartridges whose error is a trust refusal.
fn untrusted(status: &Value) -> Vec<&Value> {
	status
		.as_array()
		.into_iter()
		.flatten()
		.filter(|s| s["state"] == "failed")
		.filter(|s| {
			s["error"]
				.as_str()
				.is_some_and(|e| e.contains("since it was trusted") || e.contains("not trusted"))
		})
		.collect()
}

/// `error` with the failed cartridge that serves `key`, when one does, so a
/// missing event names its cause and fix instead of only its absence.
async fn explain(peer: &cartridge::transport::rpc::Peer, key: &str, error: Error) -> Error {
	let Ok(status) = peer.call("status", Value::Null).await else {
		return error;
	};
	let failed = status.as_array().into_iter().flatten().find(|s| {
		s["state"] == "failed"
			&& s["listen"]
				.as_array()
				.is_some_and(|l| l.iter().any(|k| k == key))
	});
	match failed {
		Some(s) => Error::Remote(format!(
			"{error}: cartridge `{}` failed: {}",
			s["id"].as_str().unwrap_or("?"),
			s["error"].as_str().unwrap_or("no error recorded")
		)),
		None => error,
	}
}

pub(crate) async fn run(project: &Project, key: &str, args: Value) -> Result<ExitCode> {
	let (peer, _incoming) = attach(project, key).await?;
	let value = client::bail(&peer, key, args).await?;
	if !value.is_null() {
		println!("{value}");
	}
	Ok(ExitCode::SUCCESS)
}

pub(crate) async fn launch(
	project: &Project,
	agent: String,
	model: String,
	mut passthrough: bool,
	mut args: Vec<String>,
) -> Result<ExitCode> {
	super::bootstrap::launch(project).await?;
	// Trailing args swallow every flag after the agent, so the switch is
	// picked out of them here, before any `--`.
	let end = args.iter().position(|a| a == "--").unwrap_or(args.len());
	let before = args.len();
	let mut index = 0;
	args.retain(|a| {
		index += 1;
		index > end || !matches!(a.as_str(), "-ps" | "--passthrough")
	});
	passthrough |= args.len() != before;
	let bases: serde_json::Map<String, Value> = ["ANTHROPIC_BASE_URL", "OPENAI_BASE_URL"]
		.into_iter()
		.filter_map(|name| Some((name.to_owned(), json!(std::env::var(name).ok()?))))
		.collect();
	let request = json!({ "op": "launch", "agent": agent, "model": model, "args": args, "passthrough": passthrough, "bases": bases });
	let (peer, _incoming) = attach(project, "proxy").await?;
	let launch = match client::bail(&peer, "proxy", request).await {
		Ok(launch) => launch,
		Err(error) => return Err(explain(&peer, "proxy", error).await),
	};
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
	let attached = attach(project, "mcp").await?;
	stdio(Backend {
		project: project.clone(),
		instance: instance(),
		attached: Arc::new(tokio::sync::Mutex::new(attached)),
	})
	.await?;
	Ok(ExitCode::SUCCESS)
}

/// This bridge's identity to the one mcp node: the client on this stdio, not
/// this process. The node keys that client's session and its in-flight calls
/// by it, so it must not repeat on a pid the system hands out again.
fn instance() -> String {
	let since = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|d| d.as_nanos())
		.unwrap_or_default();
	format!("{}-{since}", std::process::id())
}

/// The host this bridge talks to. A host replaced under it (`daemon
/// --replace`, a rebuilt binary) closes the connection; the next message
/// attaches again and is sent once more.
#[derive(Clone)]
struct Backend {
	project: Project,
	/// The attached instance this stdio is; sent with every line, because one
	/// node serves every `cartridge mcp` on the daemon.
	instance: String,
	attached: Arc<tokio::sync::Mutex<Attached>>,
}

impl Backend {
	async fn message(&self, line: &str) -> Result<Value> {
		let data = json!({ "op": "message", "line": line, "instance": self.instance });
		let peer = self.attached.lock().await.0.clone();
		match client::bail(&peer, "mcp", data.clone()).await {
			Err(_) if peer.is_closed() => {
				let mut attached = self.attached.lock().await;
				if attached.0.is_closed() {
					*attached = attach(&self.project, "mcp").await?;
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

pub(crate) async fn daemon(
	project: &Project,
	replace: bool,
	idle_timeout: u64,
) -> Result<ExitCode> {
	let already = || {
		Error::Descriptor(format!(
			"a host already serves {}; `cartridge daemon --replace` takes over from it",
			project.descriptor.display()
		))
	};
	let existing = client::served(project)
		.await
		.map_err(|error| client::unanswered(project, error))?;
	if !replace && (existing.is_some() || socket::takeover_pending(&project.descriptor)?) {
		return Err(already());
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
			// Bound before composing: a daemon that lost the race exits here
			// instead of running with no nodes.
			if let Err(error) = host.listen().await {
				host.stop().await;
				return Err(if socket::answers(&host.socket_path()) {
					already()
				} else {
					error
				});
			}
			if let Err(error) = host.reconcile().await {
				tracing::error!(target: "cartridge", cartridge = "init.lua", "{error}");
			}
		}
	}
	let idle = stop_when_idle(&host, std::time::Duration::from_secs(idle_timeout));
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
	if let Some(idle) = idle {
		idle.abort();
	}
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
