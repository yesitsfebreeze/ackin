//! A cartridge hosting cartridges of its own: the host half of the wire,
//! spoken downward, so the child sees a host and the host above sees only
//! this cartridge's own declaration.

use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use crate::cartridge::Link;

use super::{boxed, Child, Host, Result};

impl Host {
	/// Spawn a cartridge of this cartridge's own and host it over the same wire
	/// this cartridge is hosted on. Every frame the child sends is answered
	/// here — a `provide` is served by forwarding the call down, a `call` of an
	/// injected key is answered by calling up — so the child sees a host and
	/// the host above sees nothing but this cartridge's own declaration.
	/// Disposing this cartridge disposes the child.
	pub async fn spawn(&self, name: &str, cmd: &[String], config: Value) -> Result<()> {
		let program = cmd.first().ok_or_else(|| "empty cmd".to_owned())?.clone();
		let manifest = crate::cartridge::manifest_async(cmd)
			.await
			.map_err(|e| format!("{program} hello: {e}"))?;
		let mut child = crate::process::Child::new(
			tokio::process::Command::new(&cmd[0])
				.args(&cmd[1..])
				.stdin(Stdio::piped())
				.stdout(Stdio::piped())
				.stderr(Stdio::piped())
				.kill_on_drop(true)
				.spawn()
				.map_err(|e| format!("{program}: {e}"))?,
		);
		let mut stdin = child.stdin.take().expect("piped stdin");
		let stdout = child.stdout.take().expect("piped stdout");
		let stderr = child.stderr.take().expect("piped stderr");
		let label = name.to_owned();
		child.tasks.spawn(async move {
			let mut lines = BufReader::new(stderr).lines();
			while let Ok(Some(line)) = lines.next_line().await {
				crate::trace::diagnostic_line(&label, &line);
			}
		});
		let (tx, mut rx) = mpsc::unbounded_channel::<Option<Value>>();
		let link = Link::new(tx, "child is gone");
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
		link.send(json!({ "apply": { "name": name, "config": config, "capabilities":{"service_versions":self.version_queries} } }));
		let mut startup = BufReader::new(stdout);
		let mut remaining = crate::settings::host().startup_bytes;
		let ready = tokio::time::timeout(crate::process::startup_timeout(), async {
			loop {
				let Some(line) = crate::process::startup_line(&mut startup, &mut remaining)
					.await
					.map_err(|error| {
						link.shutdown();
						error.to_string()
					})?
				else {
					link.shutdown();
					// EOF may arrive before the child becomes waitable. Keep the
					// wait inside this startup timeout, preserving its actual exit.
					let status = child
						.wait()
						.await
						.map_err(|error| format!("{name} startup exit wait: {error}"))?;
					return Err(format!("{name} exited before ready: {status}"));
				};
				let Ok(m) = serde_json::from_str::<Value>(&line) else {
					link.shutdown();
					return Err(format!("{name} sent an unreadable line: {line}"));
				};
				if m["ready"] == true {
					break;
				}
				if let Some(e) = m["error"]
					.as_str()
					.filter(|_| m["reply"].as_u64().is_none())
				{
					link.shutdown();
					return Err(format!("{name}: {e}"));
				}
				self.relay(&link, m).await;
			}
			Ok::<(), String>(())
		})
		.await;
		match ready {
			Ok(Ok(())) => {}
			Ok(Err(error)) => {
				link.shutdown();
				return Err(error);
			}
			Err(_) => {
				link.shutdown();
				return Err(format!("{name} timed out before ready"));
			}
		}
		let mut lines = startup.lines();
		let stopping = Arc::new(AtomicBool::new(false));
		let host = self.clone();
		let reader_link = link.clone();
		let reader_name = name.to_owned();
		let reader_stopping = stopping.clone();
		child.tasks.spawn(async move {
			while let Ok(Some(line)) = lines.next_line().await {
				if let Ok(m) = serde_json::from_str::<Value>(&line) {
					if let Some(e) = m["error"]
						.as_str()
						.filter(|_| m["reply"].as_u64().is_none())
					{
						host.write(json!({ "error": format!("{reader_name}: {e}") }));
						continue;
					}
					host.relay(&reader_link, m).await;
				}
			}
			reader_link.shutdown();
			host.children
				.lock()
				.retain(|child| !Arc::ptr_eq(&child.link, &reader_link));
			// A child that dies while it is still wanted is a fault of the
			// sub-host's own apply, reported above it; a death during disposal
			// is the disposal working.
			if !reader_stopping.load(Ordering::SeqCst) {
				host.write(json!({ "error": format!("{reader_name}: exited") }));
			}
		});
		let (finalizer_link, mut finalizer_child) = (link.clone(), child);
		self.children.lock().push(Child {
			link: finalizer_link.clone(),
			reload: manifest.reload,
		});
		let children = self.children.clone();
		let finalizer_stopping = stopping.clone();
		self.on_dispose(move || async move {
			finalizer_stopping.store(true, Ordering::SeqCst);
			children
				.lock()
				.retain(|child| !Arc::ptr_eq(&child.link, &finalizer_link));
			finalizer_link.send(json!({ "dispose": true }));
			finalizer_link.stop();
			if tokio::time::timeout(
				crate::settings::host().shutdown_timeout(),
				finalizer_child.wait(),
			)
			.await
			.is_err()
			{
				let _ = finalizer_child.kill().await;
			}
		});
		Ok(())
	}

	/// Serve one frame from a cartridge of this cartridge's own — the host half
	/// of the wire, the inverse of this cartridge's own run loop. Frames that
	/// carry a request upward (`call`, `meta`, the bridge queries) are spawned,
	/// so a slow answer above never stalls the child's next frame.
	pub(super) async fn relay(&self, link: &Arc<Link>, m: Value) {
		if let Some(key) = m["provide"].as_str().map(str::to_owned) {
			let child = link.clone();
			let served = key.clone();
			self.services.lock().insert(
				key.clone(),
				boxed(move |args| {
					let (child, key) = (child.clone(), served.clone());
					async move { Ok(child.request(json!({ "call": key, "args": args })).await?) }
				}),
			);
			// The key is served from below, but the host above only knows the
			// keys this cartridge's declaration already carries: a declared
			// child key is re-advertised, an undeclared one stays a private
			// service of the sub-host.
			if self.declared.contains(&key) {
				self.write(json!({ "provide": key.clone() }));
			}
			return;
		}
		if let Some(name) = m["on"].as_str().map(str::to_owned) {
			let child = link.clone();
			let registered = name.clone();
			self.on(&registered, move |data| {
				let (child, name) = (child.clone(), name.clone());
				async move {
					Ok(child
						.request(json!({ "event": name, "data": data }))
						.await?)
				}
			});
			return;
		}
		if let Some(name) = m["emit"].as_str() {
			self.emit(name, m["data"].clone());
			return;
		}
		if let Some(name) = m["send"].as_str() {
			self.send(name, m["data"].clone());
			return;
		}
		if link.accept(&m) {
			return;
		}
		let id = m["id"].as_u64().unwrap_or(0);
		let (host, link, trace) = (self.clone(), link.clone(), crate::trace::of(&m));
		if let Some(key) = m["meta"].as_str().map(str::to_owned) {
			tokio::spawn(crate::trace::scope(trace, async move {
				link.reply(id, host.meta(&key).await);
			}));
		} else if let Some(key) = m["call"].as_str().map(str::to_owned) {
			let args = m["args"].clone();
			tokio::spawn(crate::trace::scope(trace, async move {
				link.reply(id, host.call(&key, args).await);
			}));
		} else if let Some(keys) = m["versions"].as_array() {
			let keys = keys
				.iter()
				.filter_map(Value::as_str)
				.map(str::to_owned)
				.collect::<Vec<_>>();
			tokio::spawn(crate::trace::scope(trace, async move {
				link.reply(id, host.service_versions(&keys).await);
			}));
		} else if m["injections"] == true {
			tokio::spawn(crate::trace::scope(trace, async move {
				link.reply(id, host.injections().await.map(|keys| json!(keys)));
			}));
		} else if m["snapshot"] == true {
			tokio::spawn(crate::trace::scope(trace, async move {
				link.reply(id, host.snapshot().await);
			}));
		} else if !m["graph"].is_null() {
			let scope = m["graph"].clone();
			tokio::spawn(crate::trace::scope(trace, async move {
				link.reply(id, host.graph(scope).await);
			}));
		} else if m["cartridges"] == true {
			tokio::spawn(crate::trace::scope(trace, async move {
				link.reply(id, host.cartridges().await);
			}));
		} else if m["bridge"] == "status" {
			tokio::spawn(crate::trace::scope(trace, async move {
				link.reply(id, host.bridge_status().await);
			}));
		} else if m["bridge"] == "call" {
			let call = m.clone();
			tokio::spawn(crate::trace::scope(trace, async move {
				link.reply(id, host.bridge_call(call).await);
			}));
		} else if m["reload"].is_boolean() {
			let reload = m.clone();
			tokio::spawn(async move {
				if host.reload.lock().contains_key("reload") {
					host.dispatch(&host.reload, &reload, "reload", reload["reload"].clone());
				} else {
					link.reply(id, Ok::<_, String>(Value::Null));
				}
			});
		} else if m["id"].is_u64() {
			link.reply(id, Err(format!("unknown message {m}")));
		}
	}
}
