//! One bounded delivery queue per node, independent of the node's Lua lock.
use crate::transport::cartridge::Ctx;
use serde_json::{json, Value};
use std::sync::{
	atomic::{AtomicU64, Ordering},
	Arc,
};

#[derive(Clone)]
pub(crate) struct Recorder {
	tx: tokio::sync::mpsc::Sender<Value>,
	dropped: Arc<AtomicU64>,
}

impl Recorder {
	pub(crate) fn record(&self, mut record: Value) {
		if record["event"].as_str().is_some_and(super::sensitive) {
			for field in ["request", "outcome", "envelope"] {
				if record.get(field).is_some() {
					record[field] = json!("<omitted>");
				}
			}
		}
		super::redact(&mut record, &mut Vec::new());
		record["ts"] = json!(super::now_ms());
		record = super::activity_limit::bound(record);
		if self.tx.try_send(record).is_err() {
			self.dropped.fetch_add(1, Ordering::Relaxed);
		}
	}
}

/// Prepare the writer envelope once; retries reuse its stable timestamp.
pub(crate) fn append(mut activity: Value) -> Value {
	if !activity["ts"].is_u64() {
		activity["ts"] = json!(super::now_ms());
	}
	super::redact(&mut activity, &mut Vec::new());
	let activity = super::activity_limit::bound(activity);
	json!({"action":"append","ts":activity["ts"],"activity":activity})
}

/// Direct trace events bypass the telemetry queue. Enforce the same contract
/// at dispatch, including completed user/assistant exchanges.
pub(crate) fn writer_request(mut data: Value) -> Value {
	if data["action"] != "append" {
		return data;
	}
	if !data["ts"].is_u64() {
		data["ts"] = json!(super::now_ms());
	}
	super::redact(&mut data, &mut Vec::new());
	let bytes = serde_json::to_vec(&data).expect("JSON serializes");
	if bytes.len() <= 48 * 1024 {
		return data;
	}
	let mut activity = if data["activity"].is_object() {
		data["activity"].clone()
	} else {
		json!({"kind":"exchange","event":"exchange","origin":"proxy","request":data["user"],"outcome":{"state":"recorded","response":data["response"]}})
	};
	activity["ts"] = data["ts"].clone();
	let mut envelope = append(activity);
	envelope["activity"]["writer_truncation"] = json!({"truncated":true,"original_bytes":bytes.len(),"sha256":crate::trust::digest_bytes(&bytes),"scope":"redacted append envelope; unlisted fields omitted"});
	envelope
}

pub(crate) fn install(ctx: &Ctx) {
	let sender = ctx.clone();
	ctx.record_into(delivery(move |record| {
		let sender = sender.clone();
		async move { sender.host("trace", record).await.map(|_| ()) }
	}));
}

pub(crate) fn delivery<F, Fut>(send: F) -> Recorder
where
	F: Fn(Value) -> Fut + Send + 'static,
	Fut: std::future::Future<Output = Result<(), String>> + Send,
{
	let (tx, mut rx) = tokio::sync::mpsc::channel::<Value>(1024);
	let dropped = Arc::new(AtomicU64::new(0));
	let recorder = Recorder {
		tx,
		dropped: dropped.clone(),
	};
	tokio::spawn(async move {
		while let Some(mut record) = rx.recv().await {
			let missing = dropped.swap(0, Ordering::Relaxed);
			if missing > 0 {
				record["dropped_before"] = json!(missing);
			}
			record = super::activity_limit::bound(record);
			loop {
				match send(record.clone()).await {
					Ok(_) => break,
					Err(error)
						if error.contains("activity exceeds 64 KiB")
							|| error.contains("activity append requires stable ts") =>
					{
						dropped.fetch_add(1, Ordering::Relaxed);
						eprintln!("cartridge trace delivery: permanent rejection: {error}; record dropped");
						break;
					}
					Err(error) => {
						eprintln!("cartridge trace delivery: {error}; retrying");
						tokio::time::sleep(std::time::Duration::from_secs(1)).await;
					}
				}
			}
		}
	});
	recorder
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/trace/activity.rs"]
mod tests;
