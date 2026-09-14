use super::*;
use std::sync::atomic::AtomicUsize;

const HOST: &str = "host-token";
const B_CALLS_A: &str = "b-calls-a";
const B_EVENTS_A: &str = "b-events-a";

struct Running {
	ctx: Ctx,
	task: tokio::task::JoinHandle<()>,
}

async fn start(path: &std::path::Path, apply: Apply) -> Running {
	let listener = listen(path).await.unwrap();
	let ctx = Ctx::new(HOST, Duration::from_secs(2));
	let task = tokio::spawn(serve(listener, ctx.clone(), apply));
	Running { ctx, task }
}

async fn host(path: &std::path::Path) -> Peer {
	let adapter = crate::typed::connect(&Endpoint::Unix(path.to_owned()))
		.await
		.unwrap();
	let (peer, _incoming) = Peer::spawn(adapter, None);
	peer.call("auth", json!({ "token": HOST })).await.unwrap();
	peer
}

async fn apply(path: &std::path::Path, name: &str, directory: &Directory) {
	host(path)
		.await
		.call(
			"apply",
			json!({ "name": name, "config": {}, "directory": directory }),
		)
		.await
		.unwrap();
}

fn directories(dir: &std::path::Path) -> (Directory, Directory) {
	let a = Directory {
		accept: BTreeMap::from([
			(
				B_CALLS_A.to_owned(),
				Grant {
					from: "b".into(),
					scope: Scope::Call,
					names: vec!["a.echo".into()],
				},
			),
			(
				B_EVENTS_A.to_owned(),
				Grant {
					from: "b".into(),
					scope: Scope::Event,
					names: vec!["ping".into()],
				},
			),
		]),
		..Directory::default()
	};
	let b = Directory {
		needs: BTreeMap::from([(
			"a.echo".to_owned(),
			Address {
				cartridge: "a".into(),
				socket: dir.join("a.sock"),
				token: B_CALLS_A.into(),
			},
		)]),
		events: BTreeMap::from([(
			"ping".to_owned(),
			vec![Address {
				cartridge: "a".into(),
				socket: dir.join("a.sock"),
				token: B_EVENTS_A.into(),
			}],
		)]),
		..Directory::default()
	};
	(a, b)
}

fn cartridge_a(disposed: Arc<AtomicUsize>) -> Apply {
	Box::new(move |ctx: Ctx, _config| {
		async move {
			ctx.provide("a.echo", |args| async move { Ok(args) });
			ctx.provide("a.secret", |_| async move { Ok(json!("secret")) });
			ctx.on("ping", |data| async move { Ok(json!({ "pong": data })) });
			ctx.on_dispose(move || async move {
				disposed.fetch_add(1, Ordering::SeqCst);
			});
			Ok(())
		}
		.boxed()
	})
}

fn cartridge_b() -> Apply {
	Box::new(|_ctx, _config| async { Ok(()) }.boxed())
}

async fn pair() -> (tempfile::TempDir, Running, Running, Arc<AtomicUsize>) {
	let dir = tempfile::tempdir().unwrap();
	let (a_directory, b_directory) = directories(dir.path());
	let disposed = Arc::new(AtomicUsize::new(0));
	let a = start(&dir.path().join("a.sock"), cartridge_a(disposed.clone())).await;
	let b = start(&dir.path().join("b.sock"), cartridge_b()).await;
	apply(&dir.path().join("a.sock"), "a", &a_directory).await;
	apply(&dir.path().join("b.sock"), "b", &b_directory).await;
	(dir, a, b, disposed)
}

#[tokio::test]
async fn a_cartridge_calls_what_it_needs() {
	let (_dir, _a, b, _) = pair().await;
	assert_eq!(
		b.ctx.call("a.echo", json!({ "hi": 1 })).await,
		Ok(json!({ "hi": 1 }))
	);
	assert_eq!(b.ctx.needs(), vec!["a.echo".to_owned()]);
}

#[tokio::test]
async fn a_key_outside_the_grant_is_refused() {
	let (dir, _a, _b, _) = pair().await;
	let adapter = crate::typed::connect(&Endpoint::Unix(dir.path().join("a.sock")))
		.await
		.unwrap();
	let (peer, _incoming) = Peer::spawn(adapter, None);
	peer.call("auth", json!({ "token": B_CALLS_A }))
		.await
		.unwrap();
	let error = peer
		.call("call", json!({ "key": "a.secret" }))
		.await
		.unwrap_err();
	assert_eq!(error.code, rpc::UNAUTHORIZED);
	let error = peer.call("dispose", json!({})).await.unwrap_err();
	assert_eq!(error.code, rpc::UNAUTHORIZED);
	let error = peer
		.call("event", json!({ "name": "ping" }))
		.await
		.unwrap_err();
	assert_eq!(
		error.code,
		rpc::UNAUTHORIZED,
		"a call token does not send events"
	);
}

#[tokio::test]
async fn an_unknown_token_is_refused_and_disconnected() {
	let (dir, _a, _b, _) = pair().await;
	let adapter = crate::typed::connect(&Endpoint::Unix(dir.path().join("a.sock")))
		.await
		.unwrap();
	let (peer, _incoming) = Peer::spawn(adapter, None);
	let error = peer
		.call("auth", json!({ "token": "guess" }))
		.await
		.unwrap_err();
	assert_eq!(error.code, rpc::UNAUTHORIZED);
	tokio::time::timeout(Duration::from_secs(1), peer.closed())
		.await
		.unwrap();

	let adapter = crate::typed::connect(&Endpoint::Unix(dir.path().join("a.sock")))
		.await
		.unwrap();
	let (peer, _incoming) = Peer::spawn(adapter, None);
	let error = peer
		.call("call", json!({ "key": "a.echo" }))
		.await
		.unwrap_err();
	assert_eq!(
		error.code,
		rpc::UNAUTHORIZED,
		"nothing is served before auth"
	);
}

#[tokio::test]
async fn events_reach_listeners_directly() {
	let (_dir, _a, b, _) = pair().await;
	assert_eq!(
		b.ctx.bail("ping", json!(1)).await,
		Ok(Some(json!({ "pong": 1 })))
	);
	assert_eq!(
		b.ctx.gather("ping", json!(2)).await,
		vec![("a".to_owned(), json!({ "pong": 2 }))]
	);
	assert_eq!(b.ctx.parallel("ping", json!(3)).await, Ok(()));
	assert_eq!(b.ctx.bail("nobody", json!(1)).await, Ok(None));
}

#[tokio::test]
async fn a_subscriber_gets_the_replay_and_then_live_events() {
	let (_dir, a, b, _) = pair().await;
	a.ctx.publish("news", json!("first"));
	let (tx, mut rx) = mpsc::unbounded_channel();
	b.ctx
		.subscribe("a", "news", Some(0), move |envelope| {
			let tx = tx.clone();
			async move {
				let _ = tx.send(envelope);
			}
		})
		.await
		.unwrap();
	a.ctx.publish("news", json!("second"));
	let mut data = Vec::new();
	while data.len() < 2 {
		let envelope = tokio::time::timeout(Duration::from_secs(1), rx.recv())
			.await
			.unwrap()
			.unwrap();
		if envelope["kind"] == "data" {
			data.push(envelope["data"].clone());
		}
	}
	assert_eq!(data, vec![json!("first"), json!("second")]);
}

#[tokio::test]
async fn a_call_reconnects_after_the_provider_restarts() {
	let (dir, a, b, disposed) = pair().await;
	assert_eq!(b.ctx.call("a.echo", json!(1)).await, Ok(json!(1)));

	host(&dir.path().join("a.sock"))
		.await
		.call("dispose", json!({}))
		.await
		.unwrap();
	tokio::time::timeout(Duration::from_secs(3), a.task)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(
		disposed.load(Ordering::SeqCst),
		1,
		"dispose runs the finalizers"
	);

	let restarted = {
		let path = dir.path().join("a.sock");
		let dir = dir.path().to_owned();
		tokio::spawn(async move {
			tokio::time::sleep(Duration::from_millis(100)).await;
			let a = start(&path, cartridge_a(Arc::new(AtomicUsize::new(0)))).await;
			apply(&path, "a", &directories(&dir).0).await;
			a
		})
	};
	assert_eq!(b.ctx.call("a.echo", json!(2)).await, Ok(json!(2)));
	drop(restarted.await.unwrap());
}
