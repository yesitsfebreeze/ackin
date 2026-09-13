use super::*;
use crate::cartridge::{Link, Remote};
use crate::runtime::Runtime;
use tokio::time::{timeout, Duration};

fn host(root: &Path) -> Arc<Host> {
	Host::new(Runtime::new(), root, root)
}

#[tokio::test]
async fn idle_disconnect_unsubscribes_without_a_future_publish() {
	let dir = tempfile::tempdir().unwrap();
	let host = host(dir.path());
	let (server, mut peer) = UnixStream::pair().unwrap();
	let task = tokio::spawn(client(host.clone(), server));
	peer.write_all(b"{\"subscribe\":\"idle\"}\n").await.unwrap();
	let mut peer = BufReader::new(peer);
	let mut line = String::new();
	timeout(Duration::from_secs(1), peer.read_line(&mut line))
		.await
		.unwrap()
		.unwrap();
	drop(peer);
	timeout(Duration::from_secs(1), task)
		.await
		.unwrap()
		.unwrap();
	let history = host.runtime().stream().replay("idle", 0);
	assert_eq!(
		history
			.iter()
			.filter(|m| m["kind"] == "unsubscribe")
			.count(),
		1
	);
}

#[tokio::test]
async fn explicit_unsubscribe_removes_idle_pump_once() {
	let dir = tempfile::tempdir().unwrap();
	let host = host(dir.path());
	let (server, mut peer) = UnixStream::pair().unwrap();
	let task = tokio::spawn(client(host.clone(), server));
	peer.write_all(b"{\"subscribe\":\"idle\"}\n{\"unsubscribe\":\"idle\"}\n{\"status\":true}\n")
		.await
		.unwrap();
	let mut peer = BufReader::new(peer);
	loop {
		let mut line = String::new();
		timeout(Duration::from_secs(1), peer.read_line(&mut line))
			.await
			.unwrap()
			.unwrap();
		if serde_json::from_str::<Value>(&line)
			.unwrap()
			.get("status")
			.is_some()
		{
			break;
		}
	}
	assert_eq!(
		host.runtime()
			.stream()
			.replay("idle", 0)
			.iter()
			.filter(|m| m["kind"] == "unsubscribe")
			.count(),
		1
	);
	drop(peer);
	timeout(Duration::from_secs(1), task)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(
		host.runtime()
			.stream()
			.replay("idle", 0)
			.iter()
			.filter(|m| m["kind"] == "unsubscribe")
			.count(),
		1
	);
}

fn pending_service(
	host: &Host,
) -> (
	Arc<Link>,
	tokio::sync::mpsc::UnboundedReceiver<Option<Value>>,
) {
	let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
	let link = Link::new(tx, "gone");
	// Install a root-owned remote directly, avoiding a fixture subprocess.
	use futures::StreamExt;
	let remote = Remote::over(link.clone(), "slow".into());
	let root = host.runtime().ctx();
	root.cartridge(
		crate::runtime::Component::new(
			"slow",
			Arc::new(move |ctx| {
				let remote = remote.clone();
				futures::stream::once(async move {
					ctx.provide("slow", Arc::new(remote))?;
					let dispose: crate::runtime::Disposer = Box::new(|| Box::pin(async {}));
					Ok(dispose)
				})
				.boxed()
			}),
		)
		.provide(["slow"]),
	);
	(link, rx)
}

#[tokio::test]
async fn writer_failure_cancels_a_pending_call_and_unsubscribes() {
	let dir = tempfile::tempdir().unwrap();
	let host = host(dir.path());
	let (link, mut requests) = pending_service(&host);
	tokio::task::yield_now().await;
	let (server, mut peer) = UnixStream::pair().unwrap();
	let task = tokio::spawn(client(host.clone(), server));
	peer.write_all(b"{\"subscribe\":\"idle\"}\n{\"call\":\"slow\",\"id\":1}\n")
		.await
		.unwrap();
	timeout(Duration::from_secs(1), requests.recv())
		.await
		.unwrap()
		.unwrap();
	assert_eq!(link.pending_count(), 1);
	drop(peer);
	timeout(Duration::from_secs(1), task)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(link.pending_count(), 0);
	assert!(host
		.runtime()
		.stream()
		.replay("idle", 0)
		.iter()
		.any(|m| m["kind"] == "unsubscribe"));
}

#[tokio::test]
async fn a_write_half_close_still_receives_its_pending_reply() {
	let dir = tempfile::tempdir().unwrap();
	let host = host(dir.path());
	let (link, mut requests) = pending_service(&host);
	tokio::task::yield_now().await;
	let (server, mut peer) = UnixStream::pair().unwrap();
	let task = tokio::spawn(client(host, server));
	peer.write_all(b"{\"call\":\"slow\",\"id\":7}\n")
		.await
		.unwrap();
	let request = timeout(Duration::from_secs(1), requests.recv())
		.await
		.unwrap()
		.unwrap()
		.unwrap();
	peer.shutdown().await.unwrap();
	// Wait across several disconnect probes: half-close is not cancellation.
	tokio::time::sleep(Duration::from_millis(250)).await;
	assert_eq!(link.pending_count(), 1);
	link.accept(&json!({"reply":request["id"],"data":42}));
	let mut peer = BufReader::new(peer);
	loop {
		let mut line = String::new();
		assert!(
			timeout(Duration::from_secs(1), peer.read_line(&mut line))
				.await
				.unwrap()
				.unwrap() > 0
		);
		let frame: Value = serde_json::from_str(&line).unwrap();
		if frame["reply"] == 7 {
			assert_eq!(frame["data"], 42);
			break;
		}
	}
	timeout(Duration::from_secs(1), task)
		.await
		.unwrap()
		.unwrap();
}
#[tokio::test]
async fn disconnect_cleans_subscriptions_while_host_owned_reconcile_waits() {
	let dir = tempfile::tempdir().unwrap();
	std::fs::write(
		dir.path().join("added.lua"),
		"return {apply=function() end}",
	)
	.unwrap();
	std::fs::write(
		dir.path().join("init.lua"),
		"return {{id='added',path='added.lua'}}",
	)
	.unwrap();
	let host = host(dir.path());
	let reload = host.reload_lock.lock().await;
	let (server, mut peer) = UnixStream::pair().unwrap();
	let task = tokio::spawn(client(host.clone(), server));
	peer.write_all(b"{\"subscribe\":\"idle\"}\n{\"reload\":true}\n{\"status\":true}\n")
		.await
		.unwrap();
	let mut peer = BufReader::new(peer);
	loop {
		let mut line = String::new();
		timeout(Duration::from_secs(1), peer.read_line(&mut line))
			.await
			.unwrap()
			.unwrap();
		if serde_json::from_str::<Value>(&line)
			.unwrap()
			.get("status")
			.is_some()
		{
			break;
		}
	}
	drop(peer);
	timeout(Duration::from_secs(1), task)
		.await
		.unwrap()
		.unwrap();
	assert!(host
		.runtime()
		.stream()
		.replay("idle", 0)
		.iter()
		.any(|m| m["kind"] == "unsubscribe"));
	assert!(host.fiber_of("added").is_none());
	drop(reload);
	// Disconnect ended the waiter, not the host's composition transaction.
	timeout(Duration::from_secs(1), async {
		while host.fiber_of("added").is_none() {
			tokio::task::yield_now().await;
		}
	})
	.await
	.unwrap();
	host.fiber_of("added").unwrap().settled().await;
	host.fiber_of("added").unwrap().dispose().await;
}
