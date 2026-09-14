//! A process cartridge: spawned inside its grant with its socket path, reached
//! once it serves, applied, and stopped.

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::oneshot;
use transport::cartridge::{Directory, CONNECT_TIMEOUT_ENV, HOST_TOKEN_ENV, SOCKET_ENV};

use crate::error::{Error, Result};

use super::{Host, Plan, Running};

pub(super) async fn start(
	host: &Arc<Host>,
	plan: &Plan,
	command: &[String],
	directory: &Directory,
	generation: u64,
) -> Result<Running> {
	let settings = crate::settings::host();
	let socket = host.socket(&plan.id);
	let _ = std::fs::remove_file(&socket);
	let sockets = socket.parent().map(std::path::Path::to_path_buf);
	let mut child = tokio::process::Command::from(
		crate::sandbox::command(command, &plan.grant, &plan.root, sockets.as_deref())
			.map_err(|e| Error::process(&plan.id, e))?,
	)
	.env(SOCKET_ENV, &socket)
	.env(HOST_TOKEN_ENV, host.host_token())
	.env(
		CONNECT_TIMEOUT_ENV,
		settings.startup_timeout_secs.to_string(),
	)
	.stdin(Stdio::piped())
	.stdout(Stdio::null())
	.stderr(Stdio::piped())
	.kill_on_drop(true)
	.spawn()
	.map_err(|e| Error::process(&command[0], e))?;
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
			if let Ok(connected) = super::connect(&socket, host.host_token()).await {
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
