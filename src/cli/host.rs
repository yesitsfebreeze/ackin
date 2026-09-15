//! Commands that start a host in this process: `run`, `launch`, `mcp`,
//! `daemon` and `verify`.

use std::process::ExitCode;
use std::sync::Arc;

use cartridge::host::{socket, Host};
use cartridge::{Error, Result};
use serde_json::{json, Value};

use super::{fail, Project, FAILED};

/// Nodes lead their own process groups, so the terminal signals this process
/// alone: a terminate stops the host, and so does an interrupt unless a program
/// holds the terminal (`launch`'s agent), which the terminal signals itself.
pub(crate) fn host(
	project: &Project,
	foreground: Option<Arc<std::sync::atomic::AtomicBool>>,
) -> Result<Arc<Host>> {
	let host = Host::new(&project.dir, &project.descriptor)?;
	stop_on_signals(host.stop_signal(), foreground)?;
	Ok(host)
}

fn stop_on_signals(
	stop: tokio_util::sync::CancellationToken,
	foreground: Option<Arc<std::sync::atomic::AtomicBool>>,
) -> Result<()> {
	use std::sync::atomic::Ordering;
	use tokio::signal::unix::{signal, SignalKind};
	let mut interrupt = signal(SignalKind::interrupt())?;
	let mut terminate = signal(SignalKind::terminate())?;
	tokio::spawn(async move {
		loop {
			tokio::select! {
				_ = terminate.recv() => break,
				_ = interrupt.recv() => {
					if !foreground.as_ref().is_some_and(|held| held.load(Ordering::SeqCst)) {
						break;
					}
				}
			}
		}
		stop.cancel();
	});
	Ok(())
}

/// Serve the host socket beside a foreground run, when no other host serves this project.
fn serve_beside(host: &Arc<Host>) -> tokio::task::JoinHandle<()> {
	let host = host.clone();
	tokio::spawn(async move {
		if let Err(error) = socket::serve(host).await {
			tracing::warn!(target: "cartridge", "host socket: {error}");
		}
	})
}

pub(crate) async fn run(project: &Project, key: &str, args: Value) -> Result<ExitCode> {
	let host = host(project, None)?;
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
	let foreground = Arc::new(std::sync::atomic::AtomicBool::new(false));
	let host = host(project, Some(foreground.clone()))?;
	let request = json!({ "op": "launch", "agent": agent, "model": model, "args": args });
	let result = host
		.run_then("proxy", request, move |launch| {
			let foreground = foreground.clone();
			async move {
				foreground.store(true, std::sync::atomic::Ordering::SeqCst);
				let status = spawn(launch).await;
				foreground.store(false, std::sync::atomic::Ordering::SeqCst);
				status
			}
		})
		.await;
	let status = result?;
	Ok(ExitCode::from(
		status.as_i64().unwrap_or(1).clamp(0, 255) as u8
	))
}

/// The variable [`minted_proxy_key`] fills.
pub(crate) const PROXY_KEY_ENV: &str = "CARTRIDGE_PROXY_KEY";

/// A fresh proxy key, or `None` when the environment already carries one. The
/// caller installs it before the runtime exists: `setenv` races every other
/// thread's `getenv`, and `config.lua` reads it in this process.
pub(crate) fn minted_proxy_key() -> Result<Option<String>> {
	match std::env::var(PROXY_KEY_ENV) {
		Ok(key) if !key.is_empty() => Ok(None),
		_ => random_hex(cartridge::settings::host().proxy_key_bytes).map(Some),
	}
}

fn random_hex(bytes: usize) -> Result<String> {
	use std::io::Read;
	let mut buf = vec![0u8; bytes];
	std::fs::File::open("/dev/urandom")
		.and_then(|mut f| f.read_exact(&mut buf))
		.map_err(|e| Error::file("/dev/urandom", e))?;
	Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

/// Run a rendered launch `{program, args, env, unset, cwd}` on this terminal
/// and resolve to its exit status.
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
	// A launch stopped under a running agent takes it down, rather than leave
	// it on the terminal without its proxy.
	command.kill_on_drop(true);
	let status = command
		.status()
		.await
		.map_err(|e| Error::process(program, e))?;
	Ok(json!(status.code().unwrap_or(1)))
}

pub(crate) async fn mcp(project: &Project) -> Result<ExitCode> {
	// The client owns this process's lifetime: the pump ends at end of
	// input and the descriptor is disposed on the way out.
	let host = host(project, None)?;
	let serve = {
		let host = host.clone();
		|_| async move { stdio(host).await }
	};
	host.run_then("mcp", json!({ "op": "ready" }), serve)
		.await?;
	Ok(ExitCode::SUCCESS)
}

/// Newline-delimited JSON-RPC between an MCP client and the `mcp` service:
/// every line of stdin is one message, every non-null reply one line of stdout.
/// One task per message, so a cancellation notification is read and acted on
/// while the call it cancels is still running. Diagnostics stay on stderr;
/// stdout carries the protocol and nothing else.
async fn stdio(host: Arc<Host>) -> Result<Value> {
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
		let (host, replies) = (host.clone(), replies.clone());
		serving.spawn(async move {
			let result = host
				.bail("mcp", json!({ "op": "message", "line": line }))
				.await
				.map(|answer| answer.unwrap_or(Value::Null));
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

/// Bridge failures still owe a request its correlated protocol response.
/// Notifications and client responses never receive an error response.
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

pub(crate) async fn daemon(project: &Project) -> Result<ExitCode> {
	let host = host(project, None)?;
	if let Err(error) = host.reconcile().await {
		tracing::error!(target: "cartridge", cartridge = "init.lua", "{error}");
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
	let result = tokio::select! {
		served = socket::serve(host.clone()) => match served {
			Ok(true) => Ok(()),
			Ok(false) => Err(Error::Descriptor(format!("a host already serves {}", project.descriptor.display()))),
			Err(error) => Err(error),
		},
		_ = host.stopped() => Ok(()),
	};
	if let Ok(watcher) = watcher {
		watcher.abort();
	}
	host.stop().await;
	result.map(|()| ExitCode::SUCCESS)
}

pub(crate) async fn verify(project: &Project, cartridge: Option<&str>) -> Result<ExitCode> {
	let host = host(project, None)?;
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

// Signals are POSIX. What this covers — an interrupt stops the base unless a
// program holds the terminal — has no counterpart to raise on Windows.
#[cfg(all(test, unix))]
#[path = "../../.cartridge/tests/unit/src/cli/signals.rs"]
mod signal_tests;
