use super::*;
use std::sync::atomic::AtomicUsize;

const HOST: &str = "host-token";
const B_TO_A: &str = "b-to-a";

struct Running {
	ctx: Ctx,
	task: tokio::task::JoinHandle<Result<()>>,
}

async fn start(path: &std::path::Path, apply: Apply) -> Running {
	let listener = listen(path).await.unwrap();
	let ctx = Ctx::new(HOST, Duration::from_secs(2));
	let task = tokio::spawn(serve(listener, ctx.clone(), apply));
	Running { ctx, task }
}

async fn host(path: &std::path::Path) -> Peer {
	let adapter = crate::transport::typed::connect(&Endpoint::local(path))
		.await
		.unwrap();
	let (peer, _incoming) = Peer::spawn(adapter, None);
	peer.call("auth", json!({ "token": HOST })).await.unwrap();
	peer
}

async fn apply_directory(path: &std::path::Path, name: &str, directory: &Directory) {
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
	let a_address = |token: &str| Address {
		cartridge: "a".into(),
		socket: dir.join("a.sock"),
		token: token.into(),
	};
	let events = BTreeMap::from([
		(
			"a.echo".to_owned(),
			EventEntry {
				owner: "a".into(),
				schema: Some(json!({"type": "object"})),
				listeners: vec![a_address(B_TO_A)],
				..Default::default()
			},
		),
		(
			"ping".to_owned(),
			EventEntry {
				owner: "b".into(),
				listeners: vec![a_address(B_TO_A)],
				..Default::default()
			},
		),
		(
			"slow".to_owned(),
			EventEntry {
				owner: "a".into(),
				timeout_ms: 50,
				listeners: vec![a_address(B_TO_A)],
				..Default::default()
			},
		),
		(
			"a.private".to_owned(),
			EventEntry {
				owner: "a".into(),
				..Default::default()
			},
		),
	]);
	let a = Directory {
		events: events.clone(),
		sends: vec!["a.echo".into(), "slow".into(), "a.private".into()],
		accept: BTreeMap::from([(
			B_TO_A.to_owned(),
			Accept {
				from: "b".into(),
				events: vec!["a.echo".into(), "ping".into(), "slow".into()],
			},
		)]),
		..Directory::default()
	};
	let b = Directory {
		events,
		sends: vec!["a.echo".into(), "ping".into(), "slow".into()],
		needs: vec!["a.echo".into()],
		..Directory::default()
	};
	(a, b)
}

fn cartridge_a(disposed: Arc<AtomicUsize>) -> Apply {
	Box::new(move |ctx: Ctx, _config| {
		async move {
			ctx.on("a.echo", |args| async move { Ok(args) });
			ctx.on("ping", |data| async move { Ok(json!({ "pong": data })) });
			ctx.on("slow", |_| async move {
				tokio::time::sleep(Duration::from_millis(500)).await;
				Ok(json!("late"))
			});
			ctx.on("a.private", |_| async move { Ok(json!("secret")) });
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
	apply_directory(&dir.path().join("a.sock"), "a", &a_directory).await;
	apply_directory(&dir.path().join("b.sock"), "b", &b_directory).await;
	(dir, a, b, disposed)
}

#[tokio::test]
async fn a_cartridge_sends_a_declared_event_and_takes_the_answer() {
	let (_dir, _a, b, _) = pair().await;
	assert_eq!(
		b.ctx.bail("a.echo", json!({ "hi": 1 })).await,
		Ok(Some(json!({ "hi": 1 })))
	);
	assert_eq!(b.ctx.needs(), vec!["a.echo".to_owned()]);
	assert_eq!(b.ctx.events()["ping"].owner, "b");
}

#[tokio::test]
async fn an_undeclared_event_or_a_bad_payload_is_refused_before_sending() {
	let (_dir, _a, b, _) = pair().await;
	let error = b.ctx.bail("nobody", json!(1)).await.unwrap_err();
	assert!(
		error.contains("not an event any cartridge declares"),
		"{error}"
	);
	let error = b.ctx.bail("a.echo", json!(1)).await.unwrap_err();
	assert!(error.contains("rejected by its schema"), "{error}");
	assert!(b.ctx.emit("a.echo", json!("no")).is_err());
	let error = b.ctx.bail("a.private", json!(null)).await.unwrap_err();
	assert!(error.contains("defines or needs"), "{error}");
}

async fn as_b(dir: &std::path::Path) -> Peer {
	let adapter = crate::transport::typed::connect(&Endpoint::local(&dir.join("a.sock")))
		.await
		.unwrap();
	let (peer, _incoming) = Peer::spawn(adapter, None);
	peer.call("auth", json!({ "token": B_TO_A })).await.unwrap();
	peer
}

#[tokio::test]
async fn a_listener_checks_what_arrives_whatever_the_sender_checked() {
	let (dir, _a, _b, _) = pair().await;
	let peer = as_b(dir.path()).await;
	let error = peer
		.call("event", json!({ "name": "a.echo", "data": 1 }))
		.await
		.unwrap_err();
	assert_eq!(error.code, rpc::INVALID_PARAMS);
	assert!(
		error.message.contains("rejected by its schema"),
		"{error:?}"
	);
	let error = peer
		.call("event", json!({ "name": "a.private", "data": null }))
		.await
		.unwrap_err();
	assert_eq!(error.code, rpc::UNAUTHORIZED, "b may not send a.private");
}

#[tokio::test]
async fn gather_reports_every_listener_outcome() {
	let (dir, a, b, _) = pair().await;
	assert_eq!(
		b.ctx.gather("slow", json!(null)).await,
		Ok(vec![Outcome::TimedOut { from: "a".into() }])
	);
	a.ctx
		.on("ping", |_| async move { Err("it broke".to_owned()) });
	let outcomes = b.ctx.gather("ping", json!(1)).await.unwrap();
	assert_eq!(
		outcomes,
		vec![Outcome::Failed {
			from: "a".into(),
			error: "it broke".into()
		}]
	);
	a.ctx.on("ping", |_| async move { Ok(Value::Null) });
	assert_eq!(
		b.ctx.gather("ping", json!(1)).await,
		Ok(vec![Outcome::Declined { from: "a".into() }])
	);
	host(&dir.path().join("a.sock"))
		.await
		.call("dispose", json!({}))
		.await
		.unwrap();
	a.task.await.unwrap().unwrap();
	let quick = Ctx::new(HOST, Duration::from_millis(100));
	quick.set_directory(b.ctx.directory());
	assert!(matches!(
		&quick.gather("ping", json!(1)).await.unwrap()[..],
		[Outcome::Unavailable { from, .. }] if from == "a"
	));
}

#[tokio::test]
async fn a_connection_runs_at_most_its_in_flight_limit_at_once() {
	let dir = tempfile::tempdir().unwrap();
	let (a_directory, b_directory) = directories(dir.path());
	let running = Arc::new(AtomicUsize::new(0));
	let peak = Arc::new(AtomicUsize::new(0));
	let apply: Apply = {
		let (running, peak) = (running.clone(), peak.clone());
		Box::new(move |ctx: Ctx, _| {
			async move {
				ctx.on("ping", move |_| {
					let (running, peak) = (running.clone(), peak.clone());
					async move {
						let now = running.fetch_add(1, Ordering::SeqCst) + 1;
						peak.fetch_max(now, Ordering::SeqCst);
						tokio::time::sleep(Duration::from_millis(20)).await;
						running.fetch_sub(1, Ordering::SeqCst);
						Ok(json!(true))
					}
				});
				Ok(())
			}
			.boxed()
		})
	};
	let _a = start(&dir.path().join("a.sock"), apply).await;
	let b = start(&dir.path().join("b.sock"), cartridge_b()).await;
	apply_directory(&dir.path().join("a.sock"), "a", &a_directory).await;
	apply_directory(&dir.path().join("b.sock"), "b", &b_directory).await;
	let sends = (0..IN_FLIGHT * 2).map(|_| b.ctx.bail("ping", json!(null)));
	for answer in futures::future::join_all(sends).await {
		assert_eq!(answer, Ok(Some(json!(true))));
	}
	assert_eq!(peak.load(Ordering::SeqCst), IN_FLIGHT);
}

#[tokio::test]
async fn a_directory_update_replaces_the_schema_a_node_validates_against() {
	let (dir, _a, b, _) = pair().await;
	assert_eq!(
		b.ctx.bail("a.echo", json!({ "n": 1 })).await,
		Ok(Some(json!({ "n": 1 })))
	);

	let (_, mut narrowed) = directories(dir.path());
	narrowed.events.get_mut("a.echo").unwrap().schema = Some(json!({
		"type": "object",
		"required": ["n"],
		"properties": { "n": { "type": "string" } }
	}));
	host(&dir.path().join("b.sock"))
		.await
		.call("directory", json!({ "directory": narrowed }))
		.await
		.unwrap();

	let error = b.ctx.bail("a.echo", json!({ "n": 1 })).await.unwrap_err();
	assert!(error.contains("rejected by its schema"), "{error}");
	assert_eq!(
		b.ctx.bail("a.echo", json!({ "n": "one" })).await,
		Ok(Some(json!({ "n": "one" })))
	);
}

#[tokio::test]
async fn a_peer_token_sends_events_and_nothing_else() {
	let (dir, _a, _b, _) = pair().await;
	let peer = as_b(dir.path()).await;
	assert_eq!(
		peer.call("event", json!({ "name": "ping", "data": 1 }))
			.await
			.unwrap(),
		json!({ "pong": 1 })
	);
	let error = peer.call("dispose", json!({})).await.unwrap_err();
	assert_eq!(error.code, rpc::UNAUTHORIZED);
}

#[tokio::test]
async fn an_unknown_token_is_refused_and_disconnected() {
	let (dir, _a, _b, _) = pair().await;
	let adapter = crate::transport::typed::connect(&Endpoint::local(&dir.path().join("a.sock")))
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
		Ok(vec![Outcome::Answered {
			from: "a".into(),
			data: json!({ "pong": 2 })
		}])
	);
	assert_eq!(
		b.ctx.ask("a", "ping", json!(3)).await,
		Ok(Outcome::Answered {
			from: "a".into(),
			data: json!({ "pong": 3 })
		})
	);
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
async fn a_send_reconnects_after_the_listener_restarts() {
	let (dir, a, b, disposed) = pair().await;
	assert_eq!(b.ctx.bail("a.echo", json!({})).await, Ok(Some(json!({}))));

	host(&dir.path().join("a.sock"))
		.await
		.call("dispose", json!({}))
		.await
		.unwrap();
	tokio::time::timeout(Duration::from_secs(3), a.task)
		.await
		.unwrap()
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
			apply_directory(&path, "a", &directories(&dir).0).await;
			a
		})
	};
	assert_eq!(
		b.ctx.bail("a.echo", json!({"again": true})).await,
		Ok(Some(json!({"again": true})))
	);
	drop(restarted.await.unwrap());
}

#[tokio::test]
async fn a_listener_that_does_not_answer_reaches_the_log() {
	use std::sync::{Arc, Mutex};
	#[derive(Clone, Default)]
	struct Sink(Arc<Mutex<Vec<u8>>>);
	impl std::io::Write for Sink {
		fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
			self.0.lock().expect("sink lock").extend_from_slice(buf);
			Ok(buf.len())
		}
		fn flush(&mut self) -> std::io::Result<()> {
			Ok(())
		}
	}
	impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Sink {
		type Writer = Sink;
		fn make_writer(&'a self) -> Sink {
			self.clone()
		}
	}
	let sink = Sink::default();
	let subscriber = tracing_subscriber::fmt()
		.with_writer(sink.clone())
		.with_max_level(tracing::Level::WARN)
		.finish();
	use tracing::instrument::WithSubscriber;
	async {
		let (_dir, _a, b, _) = pair().await;
		assert_eq!(
			b.ctx.gather("slow", json!(null)).await,
			Ok(vec![Outcome::TimedOut { from: "a".into() }])
		);
	}
	.with_subscriber(subscriber)
	.await;
	let logged = String::from_utf8(sink.0.lock().expect("sink lock").clone()).expect("utf-8 log");
	assert!(
		logged.contains("a did not answer in time") && logged.contains("slow"),
		"the timeout must name the event in the log: {logged}"
	);
}

#[tokio::test]
async fn a_cancelled_call_drops_the_listener_and_frees_its_permit() {
	struct Bell(Arc<AtomicUsize>);
	impl Drop for Bell {
		fn drop(&mut self) {
			self.0.fetch_add(1, Ordering::SeqCst);
		}
	}
	let dir = tempfile::tempdir().unwrap();
	let (a_directory, b_directory) = directories(dir.path());
	let dropped = Arc::new(AtomicUsize::new(0));
	let apply: Apply = {
		let dropped = dropped.clone();
		Box::new(move |ctx: Ctx, _| {
			let dropped = dropped.clone();
			async move {
				ctx.on("slow", move |_| {
					let dropped = dropped.clone();
					async move {
						let _bell = Bell(dropped);
						tokio::time::sleep(Duration::from_secs(3)).await;
						Ok(json!("late"))
					}
				});
				ctx.on("ping", |data| async move { Ok(json!({ "pong": data })) });
				Ok(())
			}
			.boxed()
		})
	};
	let _a = start(&dir.path().join("a.sock"), apply).await;
	let b = start(&dir.path().join("b.sock"), cartridge_b()).await;
	apply_directory(&dir.path().join("a.sock"), "a", &a_directory).await;
	apply_directory(&dir.path().join("b.sock"), "b", &b_directory).await;

	// `slow` is bound at 50 ms by `directories`; every one of these expires.
	let sends = (0..IN_FLIGHT).map(|_| b.ctx.bail("slow", json!(null)));
	for outcome in futures::future::join_all(sends).await {
		assert!(outcome.is_err(), "the bound must expire: {outcome:?}");
	}
	tokio::time::sleep(Duration::from_millis(300)).await;
	assert_eq!(
		dropped.load(Ordering::SeqCst),
		IN_FLIGHT,
		"every expired call must have stopped the work it started"
	);
	// `ping` declares no bound, so it can only answer if the permits came back.
	let answered = tokio::time::timeout(Duration::from_secs(1), b.ctx.bail("ping", json!(1))).await;
	assert_eq!(answered, Ok(Ok(Some(json!({ "pong": 1 })))));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_cancelled_call_stops_a_listener_waiting_on_another_cartridge() {
	// The hazard this PRD owns is about a listener parked on *another
	// cartridge's* answer. This is that listener, cancelled mid-call.
	struct Bell(Arc<AtomicUsize>);
	impl Drop for Bell {
		fn drop(&mut self) {
			self.0.fetch_add(1, Ordering::SeqCst);
		}
	}
	let dir = tempfile::tempdir().unwrap();
	let (mut a_directory, b_directory) = directories(dir.path());
	a_directory.sends.push("ping".to_owned());
	let dropped = Arc::new(AtomicUsize::new(0));
	let entered = Arc::new(AtomicUsize::new(0));
	let apply: Apply = {
		let (dropped, entered) = (dropped.clone(), entered.clone());
		Box::new(move |ctx: Ctx, _| {
			let (dropped, entered, nested) = (dropped.clone(), entered.clone(), ctx.clone());
			async move {
				ctx.on("slow", move |_| {
					let (dropped, entered, nested) =
						(dropped.clone(), entered.clone(), nested.clone());
					async move {
						let _bell = Bell(dropped);
						// Parked on another cartridge's answer, which never comes.
						entered.fetch_add(1, Ordering::SeqCst);
						// The nested call is itself bounded, so a tree that never
						// cancels still finishes instead of wedging the gate.
						let _ = tokio::time::timeout(
							Duration::from_millis(900),
							nested.bail("ping", json!(null)),
						)
						.await;
						Ok(json!("late"))
					}
				});
				ctx.on("ping", |_| async move {
					tokio::time::sleep(Duration::from_millis(1200)).await;
					Ok(json!("eventually"))
				});
				Ok(())
			}
			.boxed()
		})
	};
	let _a = start(&dir.path().join("a.sock"), apply).await;
	let b = start(&dir.path().join("b.sock"), cartridge_b()).await;
	apply_directory(&dir.path().join("a.sock"), "a", &a_directory).await;
	apply_directory(&dir.path().join("b.sock"), "b", &b_directory).await;

	assert!(
		b.ctx.bail("slow", json!(null)).await.is_err(),
		"the bound must expire"
	);
	tokio::time::sleep(Duration::from_millis(400)).await;
	assert_eq!(
		entered.load(Ordering::SeqCst),
		1,
		"the listener must have reached its nested call"
	);
	assert_eq!(
		dropped.load(Ordering::SeqCst),
		1,
		"a listener parked on another cartridge must be released by the cancel"
	);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn a_listener_that_blocks_does_not_park_the_only_worker() {
	// The invariant the comment above the listener's spawn guards: a listener
	// blocks synchronously, so if it ever runs on a worker it takes the runtime
	// down with it. One worker makes that observable instead of argued.
	let dir = tempfile::tempdir().unwrap();
	let (a_directory, b_directory) = directories(dir.path());
	let apply: Apply = Box::new(move |ctx: Ctx, _| {
		async move {
			ctx.on("slow", |_| async move {
				std::thread::sleep(Duration::from_millis(600));
				Ok(json!("late"))
			});
			ctx.on("ping", |data| async move { Ok(json!({ "pong": data })) });
			Ok(())
		}
		.boxed()
	});
	let _a = start(&dir.path().join("a.sock"), apply).await;
	let b = start(&dir.path().join("b.sock"), cartridge_b()).await;
	apply_directory(&dir.path().join("a.sock"), "a", &a_directory).await;
	apply_directory(&dir.path().join("b.sock"), "b", &b_directory).await;

	let busy = tokio::spawn({
		let ctx = b.ctx.clone();
		async move { ctx.bail("slow", json!(null)).await }
	});
	tokio::time::sleep(Duration::from_millis(100)).await;
	let answered =
		tokio::time::timeout(Duration::from_millis(300), b.ctx.bail("ping", json!(2))).await;
	assert_eq!(
		answered,
		Ok(Ok(Some(json!({ "pong": 2 })))),
		"a blocking listener must not stop the runtime from serving anything else"
	);
	let _ = busy.await;
}
