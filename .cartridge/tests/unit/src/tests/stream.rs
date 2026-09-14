use std::time::Duration;

use super::{boot, settle, write};
use crate::socket::{self, Client};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::sync::Arc;

/// The stream's own contract, before any wire: publish, subscribe, replay.
#[tokio::test]
async fn publishing_without_a_listener_costs_one_append_and_keeps_the_log() {
	let stream = crate::stream::Stream::new();
	let first = stream.publish("build", "echo", crate::stream::Kind::Data, json!({"n": 1}));
	let second = stream.publish("build", "echo", crate::stream::Kind::Data, json!({"n": 2}));
	assert_eq!((first, second), (1, 2));
	let log = stream.replay("build", 0);
	assert_eq!(log.len(), 2);
	assert_eq!(log[0]["seq"], json!(1));
	assert_eq!(log[1]["data"], json!({"n": 2}));
	assert_eq!(log[1]["kind"], json!("data"));
	assert_eq!(log[1]["from"], json!("echo"));
	assert_eq!(log[1]["ch"], json!("build"));
	// An unrelated channel never saw anything.
	assert!(stream.replay("other", 0).is_empty());
}

#[tokio::test]
async fn a_subscriber_receives_everything_published_to_its_channel_in_order() {
	let stream = crate::stream::Stream::new();
	let sub = stream.subscribe("build", "watcher", None);
	for n in 1..=3 {
		stream.publish("build", "echo", crate::stream::Kind::Data, json!({"n": n}));
	}
	drop(sub);
	// Re-joined from the start: the whole log, in sequence order, join first.
	let log = stream.replay("build", 0);
	let seqs: Vec<u64> = log.iter().filter_map(|e| e["seq"].as_u64()).collect();
	assert_eq!(seqs, vec![1, 2, 3, 4]);
	assert_eq!(log[0]["kind"], json!("subscribe"));
	assert_eq!(log[0]["from"], json!("watcher"));
	assert_eq!(log[3]["data"], json!({"n": 3}));
}

#[tokio::test]
async fn a_subscriber_that_crashed_and_came_back_ends_up_where_it_was() {
	let stream = crate::stream::Stream::new();
	let sub = stream.subscribe("build", "watcher", None);
	// Two publishes and another node's join past the watcher's own join.
	for n in 1..=2 {
		stream.publish("build", "echo", crate::stream::Kind::Data, json!({"n": n}));
	}
	let late = stream.subscribe("build", "late", None);
	drop(sub.rx);
	stream.unsubscribe("build", sub.id);
	stream.publish("build", "echo", crate::stream::Kind::Data, json!({"n": 3}));
	// Resume from the last sequence the watcher saw (its own join is seq 1).
	let mut sub = stream.subscribe("build", "watcher", Some(1));
	let mut seen = Vec::new();
	while let Ok(Some(envelope)) =
		tokio::time::timeout(Duration::from_millis(50), sub.rx.recv()).await
	{
		seen.push(envelope["seq"].as_u64().unwrap());
	}
	// Everything after the resume point, oldest first, and then the resume's
	// own join — replay and live feed are one sequence.
	assert_eq!(seen, vec![2, 3, 4, 5, 6, 7]);
	drop(late);
}

#[tokio::test]
async fn leaving_is_an_event_everyone_still_on_the_channel_sees() {
	let stream = crate::stream::Stream::new();
	let a = stream.subscribe("build", "a", None);
	let mut b = stream.subscribe("build", "b", None);
	// b's queue holds its own join first, then a's leaving.
	assert_eq!(b.rx.recv().await.unwrap()["kind"], json!("subscribe"));
	stream.unsubscribe("build", a.id);
	let left = b.rx.recv().await.unwrap();
	assert_eq!(left["kind"], json!("unsubscribe"));
	assert_eq!(left["from"], json!("a"));
	// A leave for an unknown id is a no-op, not an envelope.
	stream.unsubscribe("build", 999);
	stream.unsubscribe("absent", 1);
	let after = stream.replay("build", 0);
	assert_eq!(after.len(), 3);
	assert_eq!(after[2]["kind"], json!("unsubscribe"));
	drop(b);
}

#[tokio::test]
async fn a_queue_nobody_reads_ends_at_the_next_publish() {
	let stream = crate::stream::Stream::new();
	let sub = stream.subscribe("build", "gone", None);
	drop(sub.rx);
	// The dead subscription is swept here and then; the sequence keeps moving.
	let seq = stream.publish("build", "echo", crate::stream::Kind::Data, json!(true));
	assert_eq!(seq, 2);
	// Unknown channels are quiet, not errors.
	stream.unsubscribe("never", 1);
	assert!(stream.replay("never", 0).is_empty());
}

/// Concurrent publishers, no coordination: the log is still gapless and
/// ordered, and every sequence is accounted for exactly once.
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_publishers_leave_a_gapless_ordered_log() {
	let stream = Arc::new(crate::stream::Stream::new());
	let seen = Arc::new(Mutex::new(Vec::new()));
	let mut tasks = tokio::task::JoinSet::new();
	for publisher in 0..4 {
		let (stream, seen) = (stream.clone(), seen.clone());
		tasks.spawn(async move {
			for n in 0..25 {
				let seq = stream.publish(
					"build",
					&format!("p{publisher}"),
					crate::stream::Kind::Data,
					json!(n),
				);
				seen.lock().push(seq);
			}
		});
	}
	while tasks.join_next().await.is_some() {}
	let mut seqs = seen.lock().clone();
	seqs.sort();
	assert_eq!(seqs, (1..=100).collect::<Vec<u64>>());
	assert_eq!(stream.replay("build", 0).len(), 100);
}

/// Next channel frame for `channel`, returning its envelope.
async fn next_envelope(client: &mut Client, channel: &str) -> Value {
	loop {
		let m = tokio::time::timeout(Duration::from_secs(5), client.next())
			.await
			.unwrap_or_else(|_| panic!("no event on {channel} within 5s"))
			.unwrap();
		if m["channel"] == channel {
			return m["event"].clone();
		}
	}
}

/// Next outbox frame named `event`, from the same client.
async fn next_outbox(client: &mut Client, event: &str) -> Value {
	loop {
		let m = tokio::time::timeout(Duration::from_secs(5), client.next())
			.await
			.unwrap_or_else(|_| panic!("no `{event}` within 5s"))
			.unwrap();
		if m["event"] == event {
			return m["data"].clone();
		}
	}
}

fn serve(
	host: std::sync::Arc<crate::lua::Host>,
) -> (tokio::task::JoinHandle<()>, std::path::PathBuf) {
	let path = socket::path(host.dir());
	let serve_path = path.clone();
	let server = tokio::spawn(async move {
		socket::serve(host, &serve_path).await.ok();
	});
	(server, path)
}

async fn connect_retry(path: &std::path::Path) -> Client {
	tokio::time::timeout(Duration::from_secs(5), async {
		loop {
			if let Ok(client) = Client::connect(path).await {
				break client;
			}
			tokio::task::yield_now().await;
		}
	})
	.await
	.expect("socket never came up")
}

/// A repeat subscribe on the same link is a no-op: one join in the log, one
/// pump, one copy of every event — never an orphaned duplicate.
#[tokio::test(flavor = "multi_thread")]
async fn a_repeat_subscribe_spawns_no_second_pump() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "init.lua", r#"return {}"#);
	let (host, _) = boot(dir.path()).await;
	let (server, path) = serve(host.clone());
	let mut client = connect_retry(&path).await;
	client
		.send(json!({ "subscribe": "build", "since": 0 }))
		.await
		.unwrap();
	client
		.send(json!({ "subscribe": "build", "since": 0 }))
		.await
		.unwrap();
	let join = next_envelope(&mut client, "build").await;
	assert_eq!(join["kind"], json!("subscribe"));
	// The second ask appended nothing: one join, one pump.
	assert_eq!(host.runtime().stream().replay("build", 0).len(), 1);
	client
		.send(json!({ "publish": "build", "data": { "n": 1 } }))
		.await
		.unwrap();
	let event = next_envelope(&mut client, "build").await;
	assert_eq!(event["kind"], json!("data"));
	// One copy, not two: nothing further arrives for the same publish.
	let mut extra = 0;
	while let Ok(Some(m)) = tokio::time::timeout(Duration::from_millis(100), client.next()).await {
		if m["channel"] == "build" && m["event"]["kind"] == json!("data") {
			extra += 1;
		}
	}
	assert_eq!(extra, 0, "a second pump delivered a duplicate");
	server.abort();
	let _ = std::fs::remove_file(path);
}

/// Errors are events: a listener that fails publishes the failure on the
/// cartridge's own channel, where a watcher sees it without calling in.
#[tokio::test(flavor = "multi_thread")]
async fn a_failing_listener_publishes_an_error_event_on_its_channel() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"failing.lua",
		r#"return { apply = function(ctx) ctx:on("boom", function() error("nope") end) end }"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return { { id = "failing", path = "failing.lua" } }"#,
	);
	let (host, _) = boot(dir.path()).await;
	// `reconcile` spawns the cartridge fiber and returns without awaiting
	// `apply`; wait for the listener's `ctx:on` to be registered before the
	// emit, or the emit can race the registration and win.
	host.fiber_of("failing").unwrap().settled().await;
	let (server, path) = serve(host.clone());
	let mut client = connect_retry(&path).await;
	client
		.send(json!({ "subscribe": "failing", "since": 0 }))
		.await
		.unwrap();
	client
		.send(json!({ "emit": "boom", "data": null }))
		.await
		.unwrap();
	// The join comes first, then the failure as an event on the same channel.
	let join = next_envelope(&mut client, "failing").await;
	assert_eq!(join["kind"], json!("subscribe"));
	let error = next_envelope(&mut client, "failing").await;
	assert_eq!(error["kind"], json!("error"));
	assert_eq!(error["from"], json!("failing"));
	assert!(error["data"].as_str().unwrap().contains("nope"));
	server.abort();
	let _ = std::fs::remove_file(path);
}

/// One line of Lua publishes, and a socket watcher that joins late replays the
/// whole channel from the start — replay and live feed are one sequence.
#[tokio::test(flavor = "multi_thread")]
async fn a_lua_cartridge_publishes_and_a_late_watcher_replays_the_log() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"builder.lua",
		r#"return { apply = function(ctx)
			ctx:on("build", function(d) ctx:publish("build", { step = d }) end)
		end }"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return { { id = "builder", path = "builder.lua" } }"#,
	);
	let (host, _) = boot(dir.path()).await;
	// `reconcile` spawns the cartridge fiber and returns without awaiting
	// `apply`; wait for the listener's `ctx:on` to be registered before the
	// emit, or the emit can race the registration and win.
	host.fiber_of("builder").unwrap().settled().await;
	let (_server, path) = serve(host.clone());
	let mut late = connect_retry(&path).await;
	late.send(json!({ "subscribe": "build", "since": 0 }))
		.await
		.unwrap();
	late.send(json!({ "emit": "build", "data": 1 }))
		.await
		.unwrap();
	// The subscriber's own join arrives first, then the published event.
	let join = next_envelope(&mut late, "build").await;
	assert_eq!(join["kind"], json!("subscribe"));
	let first = next_envelope(&mut late, "build").await;
	assert_eq!(first["data"], json!({"step": 1}));
	assert_eq!(first["from"], json!("builder"));
	assert_eq!(first["kind"], json!("data"));
	assert_eq!(first["seq"], json!(2));
	_server.abort();
	let _ = std::fs::remove_file(path);
}

/// A Lua cartridge subscribes to a channel and receives what a socket client
/// publishes, in order, join first.
#[tokio::test(flavor = "multi_thread")]
async fn a_lua_cartridge_watches_a_channel_a_socket_client_publishes_to() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"watcher.lua",
		r#"return { apply = function(ctx)
			ctx:subscribe("build", function(envelope) ctx:send("observed", envelope) end)
		end }"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return { { id = "watcher", path = "watcher.lua" } }"#,
	);
	let (host, mut rx) = boot(dir.path()).await;
	let (server, path) = serve(host.clone());
	let mut client = connect_retry(&path).await;
	// The watcher's join echo is the proof its subscription is live before the
	// publishes are sent, so no publish can land before it.
	super::next_event(&mut rx, "observed").await;
	client
		.send(json!({ "publish": "build", "data": {"n": 7} }))
		.await
		.unwrap();
	client
		.send(json!({ "publish": "build", "data": {"n": 8} }))
		.await
		.unwrap();
	// The watcher echoes both, in arrival order, after its own join.
	let second = super::next_event(&mut rx, "observed").await;
	assert_eq!(second["kind"], json!("data"));
	assert_eq!(second["data"], json!({"n": 7}));
	let third = super::next_event(&mut rx, "observed").await;
	assert_eq!(third["data"], json!({"n": 8}));
	server.abort();
	let _ = std::fs::remove_file(path);
}

/// The full composition over the wire: a process cartridge publishes and
/// watches through its SDK, a socket client publishes to the same channel, the
/// watcher's echo shows the sequence order, an unsubscribe is an event the
/// watcher receives, and a late subscriber replays the whole log.
#[tokio::test(flavor = "multi_thread")]
async fn a_process_cartridge_publishes_and_watches_over_the_wire() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"provider.lua",
		r#"return { provide = {"lua"}, apply = function(ctx)
			ctx:provide("lua", function(args) return args end)
		end }"#,
	);
	write(
		dir.path(),
		"child.lua",
		&format!(
			"return cartridge.process({})",
			json!(super::process::sdk_fixture())
		),
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="provider",path="provider.lua"},{id="child",path="child.lua",config={telemetry=true}}}"#,
	);
	let (host, _) = boot(dir.path()).await;
	host.fiber_of("child").unwrap().settled().await;
	let (server, path) = serve(host.clone());
	let mut client = connect_retry(&path).await;
	// The watcher joined first; a late socket subscriber replays from the start.
	client
		.send(json!({ "subscribe": "build", "since": 0 }))
		.await
		.unwrap();
	let child_join = next_envelope(&mut client, "build").await;
	assert_eq!(child_join["kind"], json!("subscribe"));
	assert_eq!(child_join["from"], json!("child"));
	let base = child_join["seq"].as_u64().unwrap();

	// Two published events from the process cartridge, through the wire.
	for (id, n) in [("one", 1), ("two", 2)] {
		client
			.send(json!({
				"call": "roundtrip",
				"args": { "publish": "build", "data": { "n": n } },
				"id": id
			}))
			.await
			.unwrap();
	}
	// The socket client publishes as itself; the cartridge watcher echoes it.
	client
		.send(json!({ "publish": "build", "data": { "n": "socket" } }))
		.await
		.unwrap();

	// The watcher echoed the three publishes in sequence order (its own join
	// went to the outbox before this client connected, so it is not here). The
	// wire delivers channel frames and echoes in either order, so read until
	// both sides of the story are in.
	let mut echo_seqs = Vec::new();
	let mut socket_event = None;
	while echo_seqs.len() < 3 || socket_event.is_none() {
		let m = tokio::time::timeout(Duration::from_secs(5), client.next())
			.await
			.unwrap_or_else(|_| panic!("no event on build within 5s"))
			.unwrap();
		if m["event"] == "watched" {
			let envelope = m["data"].clone();
			let seq = envelope["seq"].as_u64().unwrap();
			if seq > base && echo_seqs.len() < 3 {
				echo_seqs.push(seq);
			}
		} else if m["channel"] == "build" && m["event"]["from"] == "socket" {
			socket_event = Some(m["event"].clone());
		}
	}
	// Three publishes, one sequence — who published first is the wire's race,
	// not the channel's concern.
	assert_eq!(echo_seqs, vec![base + 1, base + 2, base + 3]);
	let socket_event = socket_event.unwrap();
	assert_eq!(socket_event["from"], json!("socket"));
	assert_eq!(socket_event["data"], json!({ "n": "socket" }));
	assert!(echo_seqs.contains(&socket_event["seq"].as_u64().unwrap()));

	// Unsubscribing is an event on the channel, and the watcher echoes it.
	client
		.send(json!({ "unsubscribe": "build" }))
		.await
		.unwrap();
	let leave = loop {
		let echo = next_outbox(&mut client, "watched").await;
		if echo["kind"] == "unsubscribe" {
			break echo;
		}
	};
	assert_eq!(leave["from"], json!("socket"));

	// A late subscriber replays the log and finds every join, event and leave —
	// the replay first, then its own join on the live feed.
	let mut late = connect_retry(&path).await;
	late.send(json!({ "subscribe": "build", "since": 0 }))
		.await
		.unwrap();
	let mut kinds = Vec::new();
	loop {
		let envelope = next_envelope(&mut late, "build").await;
		let kind = envelope["kind"].as_str().unwrap().to_owned();
		let done = kind == "unsubscribe";
		kinds.push(kind);
		if done {
			break;
		}
	}
	assert_eq!(
		kinds,
		vec![
			"subscribe",
			"subscribe",
			"data",
			"data",
			"data",
			"unsubscribe"
		]
	);
	server.abort();
	let _ = std::fs::remove_file(path);
	settle().await;
}

#[test]
fn history_is_bounded_and_an_old_cursor_receives_a_gap() {
	// Retention is a host setting, so the test reads the value that ships
	// rather than restating it: well past the bound, whatever the bound is.
	let history = crate::settings::host().stream_history_events;
	let total = history * 4;
	let stream = crate::stream::Stream::new();
	for n in 0..total {
		stream.publish("hot", "test", crate::stream::Kind::Data, json!(n));
	}
	let replay = stream.replay("hot", 0);
	// The gap notice, then everything still retained.
	assert_eq!(replay.len(), history + 1);
	assert_eq!(replay[0]["kind"], "error");
	assert_eq!(replay[1]["seq"], total - history + 1);
	assert_eq!(replay.last().unwrap()["seq"], total);
	assert_eq!(stream.replay("hot", total as u64 - 1).len(), 1);
}

#[tokio::test]
async fn a_slow_subscriber_gets_a_gap_and_closes_without_blocking_publishers() {
	// The queue depth is a host setting; overflow it by a wide margin so the
	// subscriber is closed whatever the setting says — a burst that fits would
	// leave `recv` waiting forever.
	let queue = crate::settings::host().subscriber_events();
	let stream = crate::stream::Stream::new();
	let mut sub = stream.subscribe("hot", "slow", None);
	for n in 0..queue * 4 {
		stream.publish("hot", "test", crate::stream::Kind::Data, json!(n));
	}
	let mut events = Vec::new();
	while let Some(event) = sub.rx.recv().await {
		events.push(event);
	}
	assert!(events.len() <= queue);
	assert_eq!(events.last().unwrap()["kind"], "error");
	let seq = stream.publish("other", "test", crate::stream::Kind::Data, json!(1));
	assert_eq!(seq, 1);
}

#[tokio::test]
async fn a_full_replay_and_gap_leave_the_subscription_live() {
	// Past retention by any margin: the replay is the gap notice, everything
	// retained, and the join — and the queue still has headroom for the live
	// event that follows.
	let history = crate::settings::host().stream_history_events;
	let stream = crate::stream::Stream::new();
	for n in 0..history + 44 {
		stream.publish("full", "test", crate::stream::Kind::Data, json!(n));
	}
	let mut sub = stream.subscribe("full", "reader", Some(0));
	let mut seen = Vec::new();
	while let Ok(v) = sub.rx.try_recv() {
		seen.push(v);
	}
	assert_eq!(seen.len(), history + 2);
	assert_eq!(seen[0]["kind"], "error");
	assert_eq!(seen.last().unwrap()["kind"], "subscribe");
	stream.publish("full", "test", crate::stream::Kind::Data, json!("live"));
	assert_eq!(sub.rx.recv().await.unwrap()["data"], "live");
}
