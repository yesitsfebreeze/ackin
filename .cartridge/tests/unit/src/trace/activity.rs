use super::*;

#[tokio::test]
async fn credential_events_keep_metadata_without_their_payload() {
	let (tx, mut rx) = tokio::sync::mpsc::channel(1);
	let recorder = Recorder {
		tx,
		dropped: Arc::new(AtomicU64::new(0)),
	};
	recorder.record(
		json!({"event":"auth", "kind":"finished", "correlation":"one",
		"outcome":{"response":{"value":"credential-sentinel"}}}),
	);
	let record = rx.recv().await.unwrap();
	assert_eq!(record["correlation"], "one");
	assert_eq!(record["kind"], "finished");
	assert!(!record.to_string().contains("credential-sentinel"));
}

#[test]
fn writer_envelope_keeps_stable_timestamp_and_explicit_bounded_evidence() {
	let original = json!({"ts":12345,"kind":"finished","event":"router","origin":"host","operation":"request","request":{"input":"🦀".repeat(100000)},"outcome":{"state":"failed","error":"upstream failure","response":"x".repeat(100000)}});
	let envelope = append(original);
	assert!(envelope.to_string().len() < 65536);
	assert_eq!(envelope["ts"], 12345);
	assert_eq!(envelope["activity"]["ts"], 12345);
	assert_eq!(envelope["activity"]["kind"], "finished");
	assert_eq!(envelope["activity"]["event"], "router");
	assert_eq!(envelope["activity"]["origin"], "host");
	assert_eq!(envelope["activity"]["operation"], "request");
	assert_eq!(envelope["activity"]["outcome"]["state"], "failed");
	assert_eq!(envelope["activity"]["outcome"]["error"], "upstream failure");
	assert_eq!(envelope["activity"]["truncation"]["truncated"], true);
	assert_eq!(
		envelope["activity"]["truncation"]["sha256"]
			.as_str()
			.unwrap()
			.len(),
		64
	);
	assert_eq!(append(envelope["activity"].clone()), envelope);
	let asp = append(json!({"kind":"asp","operation":"search","origin":"host"}));
	assert!(asp["ts"].as_u64().unwrap() > 0);
	assert_eq!(asp["ts"], asp["activity"]["ts"]);
}

#[tokio::test]
async fn queued_oversize_is_bounded_at_delivery_and_permanent_failure_does_not_block_next() {
	let (tx, mut rx) = tokio::sync::mpsc::channel(4);
	let recorder = delivery(move |value| {
		let tx = tx.clone();
		async move {
			tx.send(value.clone()).await.unwrap();
			if value["event"] == "reject" {
				Err("activity exceeds 64 KiB".into())
			} else {
				Ok(())
			}
		}
	});
	// Bypass record() to exercise an already-queued oversized record.
	recorder
		.tx
		.send(json!({"ts":1,"event":"reject","request":"x".repeat(100000)}))
		.await
		.unwrap();
	recorder
		.tx
		.send(json!({"ts":2,"event":"next"}))
		.await
		.unwrap();
	let first = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
		.await
		.unwrap()
		.unwrap();
	assert!(append(first.clone()).to_string().len() < 65536);
	assert_eq!(first["truncation"]["truncated"], true);
	let next = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
		.await
		.unwrap()
		.unwrap();
	assert_eq!(next["event"], "next");
	assert_eq!(next["dropped_before"], 1);
}

#[tokio::test]
async fn transient_retry_reuses_identical_timestamp_and_digest() {
	let attempts = Arc::new(AtomicU64::new(0));
	let state = attempts.clone();
	let (tx, mut rx) = tokio::sync::mpsc::channel(3);
	let recorder = delivery(move |record| {
		let state = state.clone();
		let tx = tx.clone();
		async move {
			tx.send(record).await.unwrap();
			if state.fetch_add(1, Ordering::Relaxed) == 0 {
				Err("writer temporarily unavailable".into())
			} else {
				Ok(())
			}
		}
	});
	recorder.record(json!({"event":"model","request":"a".repeat(100000)}));
	let first = tokio::time::timeout(std::time::Duration::from_secs(3), rx.recv())
		.await
		.unwrap()
		.unwrap();
	let retry = tokio::time::timeout(std::time::Duration::from_secs(3), rx.recv())
		.await
		.unwrap()
		.unwrap();
	assert_eq!(first, retry);
	assert!(first["ts"].as_u64().unwrap() > 0);
}

#[test]
fn direct_append_exchange_and_outer_envelope_are_bounded_without_queue() {
	let original =
		json!({"action":"append","ts":123,"user":"🦀".repeat(40000),"response":"real result"});
	let bounded = writer_request(original);
	assert!(bounded.to_string().len() < 65536);
	assert_eq!(bounded["ts"], 123);
	assert_eq!(bounded["activity"]["outcome"]["response"], "real result");
	assert_eq!(bounded["activity"]["writer_truncation"]["truncated"], true);
	let outer = writer_request(
		json!({"action":"append","ts":123,"activity":{"event":"small"},"extra":"x".repeat(100000)}),
	);
	assert!(outer.to_string().len() < 65536);
	assert_eq!(outer["activity"]["event"], "small");
	assert_eq!(outer["activity"]["writer_truncation"]["truncated"], true);
	let small = json!({"action":"append","ts":123,"user":"question","response":"answer"});
	assert_eq!(writer_request(small.clone()), small);
	let read = json!({"action":"status"});
	assert_eq!(writer_request(read.clone()), read);
}

#[test]
fn measured_metrics_survive_redaction_but_untrusted_content_does_not() {
	let answer = json!({"usage":{"input_tokens":120,"output_tokens":15,"input_tokens_details":{"cached_tokens":100}},"metrics":{"retries":2,"tool_calls":3,"secret":"secret-sentinel","input_tokens":"token-sentinel","verification":{"passed":true,"reference":"checks/run-1.json"}}});
	let metrics = metrics(Some(&answer), std::time::Duration::from_millis(123));
	let record = append(json!({"metrics":metrics,"request":{"input":"private-sentinel"}}));
	assert_eq!(
		record["activity"]["metrics"],
		json!({"duration_ms":123,"input_tokens":120,"cached_input_tokens":100,"output_tokens":15,"retries":2,"tool_calls":3,"verification":{"passed":true,"reference":"checks/run-1.json"}})
	);
	for secret in ["secret-sentinel", "token-sentinel", "private-sentinel"] {
		assert!(!record.to_string().contains(secret));
	}
	assert_eq!(
		super::metrics(None, std::time::Duration::ZERO),
		json!({"duration_ms":0})
	);
	let invalid = append(
		json!({"metrics":{"duration_ms":-1,"output_tokens":2.5,"input_tokens":{"secret":"hidden"},"unknown":99,"verification":{"passed":true,"reference":"","secret":"hidden"}}}),
	);
	assert_eq!(invalid["activity"]["metrics"], json!({}));
}

#[test]
fn bounded_activity_retains_diagnosis_metrics_and_known_identities() {
	let context = context(
		&json!({"task_id":"task-1","context":{"run_id":"run-1","revision":"abc","parent_id":"parent"}}),
	);
	assert_eq!(
		context,
		json!({"task_id":"task-1","run_id":"run-1","revision":"abc","parent_id":"parent"})
	);
	let envelope = append(
		json!({"kind":"diagnostic","diagnostic":{"error":"intake capacity exceeded","password":"hidden"},"metrics":{"duration_ms":99,"task_id":"task-1"},"request":"x".repeat(100000)}),
	);
	assert_eq!(
		envelope["activity"]["diagnostic"]["error"],
		"intake capacity exceeded"
	);
	assert_eq!(envelope["activity"]["metrics"]["duration_ms"], 99);
	assert!(!envelope.to_string().contains("hidden"));
	assert!(envelope.to_string().len() < 65536);
}

#[path = "activity_baseline.rs"]
mod baseline;

/// Offline CPU comparison against the pre-change bounder, with alternating order.
#[test]
#[ignore = "run with --release --ignored --nocapture for timing evidence"]
fn telemetry_size_guard_benchmark() {
	fn run(f: fn(Value) -> Value, value: &Value, iterations: u32) -> u128 {
		let started = std::time::Instant::now();
		for _ in 0..iterations {
			std::hint::black_box(f(std::hint::black_box(value.clone())));
		}
		started.elapsed().as_nanos()
	}
	for (name, value, iterations) in [
		(
			"small",
			json!({"kind":"finished","event":"tool.read","outcome":{"state":"answered","response":{"count":42}},"metrics":{"duration_ms":15}}),
			100000,
		),
		("near_limit", json!({"request":"a".repeat(40000)}), 5000),
		("oversize", json!({"request":"a".repeat(100000)}), 2000),
	] {
		for round in 0..7 {
			let (baseline_ns, current_ns) = if round % 2 == 0 {
				(
					run(baseline::bound, &value, iterations),
					run(crate::trace::activity_limit::bound, &value, iterations),
				)
			} else {
				let current = run(crate::trace::activity_limit::bound, &value, iterations);
				(run(baseline::bound, &value, iterations), current)
			};
			println!("{name},{round},{iterations},{baseline_ns},{current_ns}");
		}
	}
}

#[test]
fn structured_diagnostic_keeps_its_message_and_redacts_credentials() {
	let diagnostic = diagnostic(
		r#"{"message":"intake capacity exceeded","password":"hidden","input":"private"}"#,
	);
	let record = append(json!({"kind":"diagnostic","diagnostic":diagnostic}));
	assert_eq!(
		record["activity"]["diagnostic"]["msg"],
		"intake capacity exceeded"
	);
	assert!(!record.to_string().contains("hidden"));
	assert!(!record.to_string().contains("private"));
}
