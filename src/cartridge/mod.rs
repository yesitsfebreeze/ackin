//! Process components registered by a Lua wrapper returning
//! `cartridge.process(command, {inject = {"extra.key"}})`. The command's `hello`
//! invocation declares static injections and provides; its normal invocation
//! is one fiber using JSON lines on stdin/stdout. Entry config arrives at apply.
//!
//! Host to cartridge: `{"apply":{"name","config"}}` first, then
//! `{"event",  "data", "id"}` for a listener the cartridge registered,
//! `{"call", "args", "id"}` for a key the cartridge provides, `{"dispose":true}`
//! last. Cartridge to host: `{"provide": key}`, `{"on": name}`,
//! `{"emit", "data"}`, `{"send", "data"}`, `{"call", "args", "id"}`,
//! `{"meta": key, "id"}`, `{"reply": id, "data" | "error"}`, `{"error": text}`,
//! and `{"ready": true}` when its apply is done. The stream rides the same
//! wire: `{"publish": channel, "data"}` from a cartridge, `{"subscribe":
//! channel}` and `{"unsubscribe": channel}` from a cartridge that watches one,
//! and delivery back as `{"channel": channel, "event": envelope}` lines. A
//! value a process provides
//! is a [`Remote`]: calling it from Lua or another process sends `call`.
//!
//! [`link`] is the wire itself, either end; [`start`] brings a process up and
//! reads it until it goes; [`frames`] serves each frame it writes.

mod frames;
mod link;
mod start;

use start::start;

use std::sync::Arc;

use futures::StreamExt;
use serde_json::Value as Json;

use crate::error::{Error, Result};
use crate::lua::Host;
use crate::runtime::{Component, Ctx, Error as RuntimeError};

pub use link::{Link, Remote};

pub fn split(cmd: &str) -> Vec<String> {
	cmd.split_whitespace().map(str::to_owned).collect()
}

/// Resolve once so hello, apply and watches name the same executable. A bare
/// program name is searched for in the cartridge's own `bin/`, where a bundle
/// ships it, then beside the running cartridge — a dev-build convenience, since
/// `cargo build --workspace` drops every binary next to this one — then on
/// PATH. Relative explicit paths resolve inside the cartridge directory.
pub fn executable(program: &str, root: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
	use std::os::unix::fs::PermissionsExt;
	let path = std::path::Path::new(program);
	let paths = if path.components().count() > 1 || path.is_absolute() {
		vec![if path.is_absolute() {
			path.to_path_buf()
		} else {
			root.join(path)
		}]
	} else {
		let beside = std::env::current_exe()
			.ok()
			.and_then(|exe| exe.parent().map(|dir| dir.join(program)));
		std::iter::once(root.join(".cartridge/bin").join(program))
			.chain(std::iter::once(root.join("bin").join(program)))
			.chain(beside)
			.chain(std::env::var_os("PATH").into_iter().flat_map(|paths| {
				std::env::split_paths(&paths)
					.map(|dir| dir.join(program))
					.collect::<Vec<_>>()
			}))
			.collect()
	};
	paths
		.into_iter()
		.find(|path| {
			path.metadata()
				.is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
		})
		.ok_or_else(|| {
			std::io::Error::new(
				std::io::ErrorKind::NotFound,
				format!("process executable `{program}` was not found or is not executable"),
			)
		})?
		.canonicalize()
}

/// What `cmd hello` prints; the shape [`crate::sdk::Cartridge::run`] writes.
#[derive(serde::Deserialize)]
pub struct Manifest {
	#[serde(default)]
	pub reload: bool,
	#[serde(default)]
	pub inject: Vec<String>,
	#[serde(default)]
	pub provide: Vec<String>,
}

pub(crate) fn manifest(cmd: &[String]) -> Result<Manifest> {
	// The synchronous composition API also works outside Tokio. Its bounded
	// discovery runtime never nests block_on inside the caller's executor.
	std::thread::scope(|scope| {
		scope
			.spawn(|| {
				tokio::runtime::Builder::new_current_thread()
					.enable_all()
					.build()?
					.block_on(manifest_async(cmd))
			})
			.join()
			.map_err(|_| Error::Invalid("discovery worker panicked"))?
	})
}

pub(crate) async fn manifest_async(cmd: &[String]) -> Result<Manifest> {
	let out = crate::process::discover(cmd, crate::process::startup_timeout()).await?;
	let program = &cmd[0];
	for line in String::from_utf8_lossy(&out.stderr).lines() {
		crate::trace::diagnostic_line(program, line);
	}
	if !out.status.success() {
		return Err(Error::process(program, format!("hello: {}", out.status)));
	}
	serde_json::from_slice(&out.stdout).map_err(|e| Error::process(program, format!("hello: {e}")))
}

pub fn component(
	host: Arc<Host>,
	name: String,
	cmd: Vec<String>,
	config: Json,
) -> Result<Component> {
	let m = manifest(&cmd)?;
	let cartridge = name.clone();
	let apply = Arc::new(move |ctx: Ctx| {
		let (host, name, cmd, config) =
			(host.clone(), cartridge.clone(), cmd.clone(), config.clone());
		futures::stream::once(async move {
			start(host, ctx, name, cmd, config, m.reload)
				.await
				.map_err(|e| RuntimeError::Apply(e.to_string()))
		})
		.boxed()
	});
	Ok(Component::new(name, apply)
		.inject(m.inject)
		.provide(m.provide))
}
