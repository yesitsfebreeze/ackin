//! A process cartridge from spawn to exit: the wire attached, the startup
//! frames read until `ready`, the read loop that serves it, and the
//! disposer that takes it down.

use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde_json::{json, Value as Json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::error::{Error, Result};
use crate::lua::Host;
use crate::runtime::{Ctx, Disposer, Error as RuntimeError};

use super::frames::handle;
use super::link::Link;

/// A running cartridge process with its wire attached: the child, the link
/// its frames travel on, and the reader that is still reading its startup.
struct Spawned {
	child: crate::process::Child,
	link: Arc<Link>,
	stdout: BufReader<tokio::process::ChildStdout>,
}

/// Spawn the process and attach the wire: stderr to the diagnostic stream,
/// stdin to the link's writer, and the `apply` frame already queued.
fn spawn(name: &str, cmd: &[String], config: Json, reload: bool) -> Result<Spawned> {
	let mut child = crate::process::Child::new(
		Command::new(&cmd[0])
			.args(&cmd[1..])
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.kill_on_drop(true)
			.spawn()
			.map_err(|e| Error::process(&cmd[0], e))?,
	);
	let mut stdin = child.stdin.take().expect("piped stdin");
	let stdout = child.stdout.take().expect("piped stdout");
	// The host owns every diagnostic byte: redaction and the byte cap are only
	// possible where it formats the writes, so a cartridge never inherits stderr.
	let stderr = child.stderr.take().expect("piped stderr");
	child.tasks.spawn({
		let name = name.to_owned();
		async move {
			let mut lines = BufReader::new(stderr).lines();
			while let Ok(Some(line)) = lines.next_line().await {
				crate::trace::diagnostic_line(&name, &line);
			}
		}
	});
	let (tx, mut rx) = mpsc::unbounded_channel::<Option<Json>>();
	let link = Link::new(tx, "cartridge is gone");
	link.reload.store(reload, Ordering::Relaxed);
	let writer_link = link.clone();
	child.tasks.spawn(async move {
		while let Some(Some(m)) = rx.recv().await {
			let mut line = m.to_string();
			line.push('\n');
			if stdin.write_all(line.as_bytes()).await.is_err() {
				writer_link.close();
				break;
			}
		}
	});
	link.send(
		json!({ "apply": { "name": name, "config": config, "capabilities":{"service_versions":true} } }),
	);
	Ok(Spawned {
		child,
		link,
		stdout: BufReader::new(stdout),
	})
}

/// Read the startup frames until the cartridge says `ready`, serving the
/// registrations it makes on the way. Bounded by the startup budget in bytes
/// and by the caller's deadline; an EOF before ready waits for the exit status
/// so the failure names it.
async fn await_ready(host: &Arc<Host>, ctx: &Ctx, name: &str, spawned: &mut Spawned) -> Result<()> {
	let mut remaining = crate::settings::host().startup_bytes;
	loop {
		let Some(line) = crate::process::startup_line(&mut spawned.stdout, &mut remaining).await?
		else {
			// EOF can precede process termination; the surrounding startup
			// deadline still bounds a child that closes stdout and stays alive.
			let status = spawned
				.child
				.wait()
				.await
				.map_err(|e| Error::process(name, format!("startup exit wait: {e}")))?;
			return Err(Error::process(
				name,
				format!("exited before ready: {status}"),
			));
		};
		let m: Json = serde_json::from_str(&line)?;
		if m["ready"] == true {
			return Ok(());
		}
		if let Some(e) = m["error"]
			.as_str()
			.filter(|_| m["reply"].as_u64().is_none())
		{
			return Err(Error::process(name, e));
		}
		handle(host, ctx, &spawned.link, name, m).map_err(|e| Error::process(name, e))?;
	}
}

pub(super) async fn start(
	host: Arc<Host>,
	ctx: Ctx,
	name: String,
	cmd: Vec<String>,
	config: Json,
	reload: bool,
) -> Result<Disposer> {
	let mut spawned = spawn(&name, &cmd, config, reload)?;
	let ready = tokio::time::timeout(
		crate::process::startup_timeout(),
		await_ready(&host, &ctx, &name, &mut spawned),
	)
	.await;
	match ready {
		Ok(Ok(())) => {}
		Ok(Err(error)) => {
			spawned.link.shutdown();
			return Err(error);
		}
		Err(_) => {
			spawned.link.shutdown();
			return Err(Error::Timeout(name));
		}
	}
	let Spawned {
		mut child,
		link,
		stdout,
	} = spawned;
	let stopping = Arc::new(AtomicBool::new(false));
	child.tasks.spawn(serve(
		host.clone(),
		ctx.clone(),
		link.clone(),
		name.clone(),
		stdout.lines(),
		stopping.clone(),
	));
	Ok(Box::new(move || {
		Box::pin(async move {
			stopping.store(true, Ordering::SeqCst);
			link.close();
			link.leave_subs(host.runtime().stream());
			link.send(json!({ "dispose": true }));
			link.stop();
			let mut child = child;
			if tokio::time::timeout(crate::settings::host().shutdown_timeout(), child.wait())
				.await
				.is_err()
			{
				let _ = child.kill().await;
			}
		})
	}))
}

/// The read loop after ready: every frame the cartridge writes is served, a
/// frame that cannot be is reported on the cartridge's channel, and EOF is
/// the cartridge's exit — a failure of the fiber unless the host is the one
/// stopping it.
async fn serve(
	host: Arc<Host>,
	ctx: Ctx,
	link: Arc<Link>,
	name: String,
	mut lines: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
	stopping: Arc<AtomicBool>,
) {
	while let Ok(Some(line)) = lines.next_line().await {
		let result = match serde_json::from_str::<Json>(&line) {
			Ok(m) => match m["error"].as_str() {
				Some(e) if m["reply"].as_u64().is_none() => Err(Error::remote(e)),
				_ => handle(&host, &ctx, &link, &name, m),
			},
			Err(e) => Err(Error::Json(e)),
		};
		if let Err(e) = result {
			host.report(&name, e);
		}
	}
	link.shutdown();
	link.leave_subs(host.runtime().stream());
	if !stopping.load(Ordering::SeqCst) {
		ctx.runtime()
			.fail(ctx.fiber(), RuntimeError::Apply("exited".into()));
	}
}
