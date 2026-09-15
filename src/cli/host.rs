use std::process::ExitCode;
use std::sync::Arc;

use cartridge::host::{socket, Host};
use cartridge::{Error, Result};
use serde_json::{json, Value};

use super::{client, fail, Project, FAILED};

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
		loop {
			tokio::select! {
				_ = terminate.recv() => break,
				// A foreground-held program (launch's agent) gets the terminal's
				// interrupt directly; ignore ours or the host stops under it.
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

fn serve_beside(host: &Arc<Host>) -> tokio::task::JoinHandle<()> {
	let host = host.clone();
	tokio::spawn(async move {
		if let Err(error) = socket::serve(host).await {
			tracing::warn!(target: "cartridge", "host socket: {error}");
		}
	})
}

pub(crate) async fn run(project: &Project, key: &str, args: Value) -> Result<ExitCode> {
	if let Some((peer, _incoming)) = client::served(project).await {
		let value = client::bail(&peer, key, args).await?;
		if !value.is_null() {
			println!("{value}");
		}
		return Ok(ExitCode::SUCCESS);
	}
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
	let request = json!({ "op": "launch", "agent": agent, "model": model, "args": args });
	if let Some((peer, _incoming)) = client::served(project).await {
		// A proxy-less composition errors here; that's not a refusal, the
		// local host below takes over instead.
		if let Ok(launch) = client::bail(&peer, "proxy", request.clone()).await {
			let status = spawn(launch).await?;
			return Ok(ExitCode::from(
				status.as_i64().unwrap_or(1).clamp(0, 255) as u8
			));
		}
	}
	let foreground = Arc::new(std::sync::atomic::AtomicBool::new(false));
	let host = host(project, Some(foreground.clone()))?;
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
	// Dropping this without kill_on_drop would leave the agent running
	// unproxied.
	command.kill_on_drop(true);
	let status = command
		.status()
		.await
		.map_err(|e| Error::process(program, e))?;
	Ok(json!(status.code().unwrap_or(1)))
}

pub(crate) async fn mcp(project: &Project) -> Result<ExitCode> {
	if let Some((peer, _incoming)) = client::served(project).await {
		stdio(Backend::Remote(peer)).await?;
		return Ok(ExitCode::SUCCESS);
	}
	let host = host(project, None)?;
	let serve = {
		let host = host.clone();
		|_| async move { stdio(Backend::Local(host)).await }
	};
	host.run_then("mcp", json!({ "op": "ready" }), serve)
		.await?;
	Ok(ExitCode::SUCCESS)
}

#[derive(Clone)]
enum Backend {
	Local(Arc<Host>),
	Remote(cartridge::transport::rpc::Peer),
}

impl Backend {
	async fn message(&self, line: &str) -> Result<Value> {
		let data = json!({ "op": "message", "line": line });
		match self {
			Backend::Local(host) => host
				.bail("mcp", data)
				.await
				.map(|answer| answer.unwrap_or(Value::Null)),
			Backend::Remote(peer) => client::bail(peer, "mcp", data).await,
		}
	}
}

/// One task per message: sequential processing would block a cancellation
/// notification behind the call it cancels. Never print to stdout here — it
/// carries the protocol.
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
		// Without this a long session keeps every finished task alive.
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

/// Returns None (no reply sent) for a notification or an unparseable line,
/// not just for a successful call: only a request with an id owes an error.
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

// No Windows counterpart: ctrl_c/ctrl_close carry no foreground-held signal.
#[cfg(all(test, unix))]
#[path = "../../.cartridge/tests/unit/src/cli/signals.rs"]
mod signal_tests;
