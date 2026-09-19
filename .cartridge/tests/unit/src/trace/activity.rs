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
