use std::process::ExitCode;

use cartridge::transport::rpc::Incoming;
use cartridge::{Error, Result};
use serde_json::{json, Value};

use super::Project;

pub(crate) async fn ask(project: &Project, method: &str, params: Value) -> Result<ExitCode> {
	let (peer, _incoming) = cartridge::host::socket::client(&project.descriptor).await?;
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
	let (peer, mut incoming) = cartridge::host::socket::client(&project.descriptor).await?;
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

pub(crate) async fn served(
	project: &Project,
) -> Option<(
	cartridge::transport::rpc::Peer,
	tokio::sync::mpsc::Receiver<Incoming>,
)> {
	cartridge::host::socket::client(&project.descriptor)
		.await
		.ok()
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
