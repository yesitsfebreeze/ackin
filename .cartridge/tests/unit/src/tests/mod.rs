mod bridge;
mod cartridges;
mod composition;
mod contracts;
mod fabric;
mod folders;
mod foreground;
mod ledger;
mod lifecycle;
mod manifest;
mod node;
mod process;
mod reload;
mod resolver;
mod root;
mod rpc_contract;
mod settings;
mod socket;
mod stream;
mod wire;

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::lua::Host;
use crate::runtime::Runtime;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast::Receiver;

fn write(dir: &Path, name: &str, body: &str) {
	std::fs::write(dir.join(name), body).unwrap();
}

/// Build a workspace target through cargo and return the executable it produced.
/// Every fixture binary the tests spawn comes from here, so the build invocation
/// and the `--message-format=json` parse exist once.
fn built(args: &[&str]) -> PathBuf {
	let output = std::process::Command::new(env!("CARGO"))
		.arg("build")
		.args(args)
		.arg("--message-format=json")
		.current_dir(env!("CARGO_MANIFEST_DIR"))
		.output()
		.unwrap();
	assert!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
	String::from_utf8(output.stdout)
		.unwrap()
		.lines()
		.filter_map(|line| serde_json::from_str::<Value>(line).ok())
		.find_map(|m| m["executable"].as_str().map(PathBuf::from))
		.unwrap_or_else(|| panic!("cargo build {args:?} produced no executable"))
}

/// Give the runtime's background reconciliation a moment to quiesce.
// ponytail: fixed sleep, swap for a lifecycle-event await if it ever flakes.
async fn settle() {
	tokio::time::sleep(Duration::from_millis(60)).await;
}

/// A host over `dir` with its outbox already subscribed, reconciled once.
async fn boot(dir: &Path) -> (Arc<Host>, Receiver<Value>) {
	let host = Host::new(Runtime::new(), dir, dir);
	let rx = host.outbox();
	host.reconcile().await.unwrap();
	(host, rx)
}

/// Next outbox message named `name`, returning its data. Panics on any error frame.
async fn next_event(rx: &mut Receiver<Value>, name: &str) -> Value {
	loop {
		let m = tokio::time::timeout(Duration::from_secs(5), rx.recv())
			.await
			.unwrap_or_else(|_| panic!("no `{name}` event within 5s"))
			.unwrap();
		if m["event"] == name {
			return m["data"].clone();
		}
		if m.get("error").is_some() {
			panic!("{m}");
		}
	}
}
