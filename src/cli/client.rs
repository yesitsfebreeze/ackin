use std::process::ExitCode;

use cartridge::transport::rpc::Incoming;
use cartridge::{Error, Result};
use serde_json::{json, Value};

use super::Project;

pub(crate) async fn ask(project: &Project, method: &str, params: Value) -> Result<ExitCode> {
	let (peer, _incoming) = cartridge::host::socket::client(&project.descriptor)
		.await
		.map_err(|error| unanswered(project, error))?;
	let answer = peer
		.call(method, params)
		.await
		.map_err(|e| Error::Remote(e.message))?;
	if !answer.is_null() {
		println!("{answer}");
	}
	Ok(ExitCode::SUCCESS)
}

pub(crate) async fn follow(
	project: &Project,
	channel: &str,
	since: Option<u64>,
) -> Result<ExitCode> {
	let (peer, mut incoming) = cartridge::host::socket::client(&project.descriptor)
		.await
		.map_err(|error| unanswered(project, error))?;
	peer.call("subscribe", json!({ "channel": channel, "since": since }))
		.await
		.map_err(|e| Error::Remote(e.message))?;
	while let Some(message) = incoming.recv().await {
		if let Incoming::Notification { params, .. } = message {
			println!("{params}");
		}
	}
	Ok(ExitCode::SUCCESS)
}

/// A socket that accepts but does not serve: name it and the way out. Any
/// other connect error (nothing there) passes through as it is.
pub(crate) fn unanswered(project: &Project, error: Error) -> Error {
	use cartridge::host::socket;
	let Ok(path) = socket::path(&project.descriptor) else {
		return error;
	};
	if !socket::answers(&path) {
		return error;
	}
	Error::Remote(format!(
		"{error}\nthe host socket {} answers but does not serve; `cartridge stop`, \
		 else `kill <pid>` where <pid> is the numeric directory beside it",
		path.display()
	))
}

/// `Ok(None)` only when nothing answers on the project's socket; a socket that
/// answers but fails to connect or authenticate is an error, never "no host".
#[cfg(unix)]
pub(crate) async fn served(
	project: &Project,
) -> Result<
	Option<(
		cartridge::transport::rpc::Peer,
		tokio::sync::mpsc::Receiver<Incoming>,
	)>,
> {
	use cartridge::host::socket;
	if !socket::answers(&socket::path(&project.descriptor)?) {
		return Ok(None);
	}
	socket::client(&project.descriptor).await.map(Some)
}

#[cfg(windows)]
pub(crate) async fn served(
	project: &Project,
) -> Result<
	Option<(
		cartridge::transport::rpc::Peer,
		tokio::sync::mpsc::Receiver<Incoming>,
	)>,
> {
	Ok(cartridge::host::socket::client(&project.descriptor)
		.await
		.ok())
}

pub(crate) async fn bail(
	peer: &cartridge::transport::rpc::Peer,
	name: &str,
	data: Value,
) -> Result<Value> {
	peer.call("bail", json!({ "name": name, "data": data }))
		.await
		.map_err(|e| Error::Remote(e.message))
}
