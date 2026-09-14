//! The commands that talk to a running host over its socket: one request,
//! the answer printed, the connection closed.

use std::process::ExitCode;

use cartridge::socket::{self, Client};
use cartridge::{Error, Result};
use serde_json::{json, Value};

use super::{fail, Project, FAILED};

async fn connect(project: &Project) -> Result<Client> {
	Client::connect(&socket::path(&project.profile))
		.await
		.map_err(|e| Error::Unavailable {
			key: "daemon".into(),
			why: format!("no daemon for {}: {e}", project.profile.display()),
		})
}

/// One request that expects no answer.
pub(crate) async fn send(project: &Project, request: Value) -> Result<ExitCode> {
	connect(project).await?.send(request).await?;
	Ok(ExitCode::SUCCESS)
}

/// Watch one stream channel until the host closes the connection.
pub(crate) async fn follow(project: &Project, channel: &str) -> Result<ExitCode> {
	let mut client = connect(project).await?;
	client.send(json!({ "subscribe": channel })).await?;
	while let Some(m) = client.next().await {
		if m.get("channel").is_some() || m.get("error").is_some() {
			println!("{m}");
		}
	}
	Ok(ExitCode::SUCCESS)
}

/// Print every message the host puts on its socket.
pub(crate) async fn tail(project: &Project) -> Result<ExitCode> {
	let mut client = connect(project).await?;
	while let Some(m) = client.next().await {
		println!("{m}");
	}
	Ok(ExitCode::SUCCESS)
}

/// One request, one answer named `answer`. A request naming a trace gets it
/// back on the reply, so a probe report can cite the id of one specific call.
pub(crate) async fn ask(project: &Project, request: Value, answer: &str) -> Result<ExitCode> {
	let trace = request.get("trace").cloned();
	let mut client = connect(project).await?;
	client.send(request).await?;
	while let Some(mut m) = client.next().await {
		if m.get(answer).is_some() || m.get("error").is_some_and(|e| e.get("cartridge").is_some()) {
			if let (Some(trace), Some(o)) = (&trace, m.as_object_mut()) {
				o.insert("trace".into(), trace.clone());
			}
			println!("{m}");
			return Ok(ExitCode::SUCCESS);
		}
	}
	Ok(fail(
		FAILED,
		format!(
			"no {answer} from {}: the host closed the connection",
			project.profile.display()
		),
	))
}
