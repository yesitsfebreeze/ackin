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
	if super::activity_limit::fits(&data, 48 * 1024) {
		return data;
	}
	let bytes = serde_json::to_vec(&data).expect("JSON serializes");
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

/// Only measured numeric quantities may bypass content/token redaction.
pub(super) fn sanitize_metrics(value: &mut Value) {
	if let Some(object) = value.as_object_mut() {
		object.retain(|key, value| match key.as_str() {
			"duration_ms"
			| "input_tokens"
			| "cached_input_tokens"
			| "output_tokens"
			| "retries"
			| "tool_calls" => value.is_u64(),
			"task_id" | "run_id" | "request_id" | "parent_id" | "revision" | "kind" => {
				value.as_str().is_some_and(|s| s.len() <= 256)
			}
			"verification" => value.as_object().is_some_and(|o| {
				o.len() == 2
					&& value["passed"].is_boolean()
					&& value["reference"]
						.as_str()
						.is_some_and(|s| !s.is_empty() && s.len() <= 256)
			}),
			_ => false,
		});
	} else {
		*value = json!({});
	}
}

pub(crate) fn context(request: &Value) -> Value {
	let mut retained = json!({});
	for key in ["request_id", "task_id", "run_id", "parent_id", "revision"] {
		let value = request.get(key).or_else(|| request["context"].get(key));
		if let Some(value) = value.filter(|v| v.as_str().is_some_and(|s| s.len() <= 256)) {
			retained[key] = value.clone();
		}
	}
	retained
}

pub(crate) fn metrics(answer: Option<&Value>, elapsed: std::time::Duration) -> Value {
	let mut metrics = answer
		.and_then(|a| a.get("metrics"))
		.filter(|v| v.is_object())
		.cloned()
		.unwrap_or_else(|| json!({}));
	sanitize_metrics(&mut metrics);
	if let Some(answer) = answer {
		for (key, alternatives) in [
			(
				"input_tokens",
				&["/usage/input_tokens", "/usage/prompt_tokens"][..],
			),
			(
				"output_tokens",
				&["/usage/output_tokens", "/usage/completion_tokens"][..],
			),
			(
				"cached_input_tokens",
				&[
					"/usage/input_tokens_details/cached_tokens",
					"/usage/prompt_tokens_details/cached_tokens",
				][..],
			),
		] {
			if let Some(value) = alternatives
				.iter()
				.find_map(|path| answer.pointer(path).filter(|v| v.is_u64()))
			{
				metrics[key] = value.clone();
			}
		}
	}
	metrics["duration_ms"] = json!(elapsed.as_millis().min(u64::MAX as u128) as u64);
	metrics
}

/// A diagnostic message is the log's meaning, rather than an LLM message body.
pub(crate) fn diagnostic(line: &str) -> Value {
	let mut value: Value = serde_json::from_str(line).unwrap_or_else(|_| json!(line));
	if let Some(object) = value.as_object_mut() {
		if let Some(message) = object.remove("message") {
			object.entry("msg").or_insert(message);
		}
	}
	value
}
