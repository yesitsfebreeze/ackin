//! A chain node for the resolver probe: the program a cartridge's `binary`
//! names, standing in for whatever a real cartridge would run. It is up the
//! moment it starts — its work is to stay up — and it re-enters the binary
//! for the next link of the chain it was launched into, holding the child it
//! spawned, so its own death takes its dependent with it. Its stdin closes
//! when the dependency that launched it goes away, and EOF is the second
//! half of that cascade.
//!
//! The pid it writes under `CARTRIDGE_ROOT/.nodes/` is the probe's observation
//! handle, not part of the mechanism: a test reads it to name the process it
//! is asserting about.
use tokio::io::AsyncReadExt;
use tokio::process::Command;

#[tokio::main]
async fn main() {
	let node = std::env::var("CARTRIDGE_NODE").unwrap_or_default();
	let chain: Vec<String> =
		serde_json::from_str(&std::env::var("CARTRIDGE_CHAIN").unwrap_or_default())
			.unwrap_or_default();
	let cartridge = std::env::var("CARTRIDGE_CARTRIDGE").expect("CARTRIDGE_CARTRIDGE");
	let root = std::env::var("CARTRIDGE_ROOT").expect("CARTRIDGE_ROOT");
	eprintln!("{node} up");

	if let Some(dir) = std::env::var_os("CARTRIDGE_NODES") {
		let marker = node.replace('/', "_");
		let _ = std::fs::write(
			std::path::Path::new(&dir).join(marker),
			std::process::id().to_string(),
		);
	}

	// The dependency launches its dependent: one link, one re-entry. The
	// child is held, so dropping it — on exit or on kill — takes the rest of
	// the tree along, and its stdin is the pipe this node's death closes.
	let mut dependent: Option<tokio::process::Child> = None;
	if let Some(next) = chain.first() {
		let rest = serde_json::to_string(&chain[1..]).expect("node paths serialize");
		let mut reentry = Command::new(&cartridge);
		reentry
			.args(["enter", next, "--rest", &rest, "--dir", &root])
			.env("CARTRIDGE_CARTRIDGE", &cartridge)
			.env("CARTRIDGE_ROOT", &root)
			.stdin(std::process::Stdio::piped())
			.kill_on_drop(true);
		dependent = Some(reentry.spawn().expect("re-enter the resolver"));
	}

	// Stay up until the parent's side of the pipe closes, then exit. The
	// drop of the child handle above is what carries the tree down with it.
	// The far end of the chain is different: nothing launched it that owns
	// it, so it has no pipe to watch and dies only when it is killed.
	if std::env::var_os("CARTRIDGE_BOTTOM").is_none() {
		let mut stdin = tokio::io::stdin();
		let mut buffer = [0u8; 64];
		while stdin.read(&mut buffer).await.unwrap_or(0) > 0 {}
	} else {
		futures::future::pending::<()>().await;
	}
	// The held child goes with this node: the drop is what kills it.
	drop(dependent);
	eprintln!("{node} gone");
}
