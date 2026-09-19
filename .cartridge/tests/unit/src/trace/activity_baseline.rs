// Telemetry summaries stay below the trace writer's 64 KiB append contract.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
const ACTIVITY_BYTES: usize = 48 * 1024;
fn summary(value: &Value) -> Value {
	let bytes = serde_json::to_vec(value).expect("JSON serializes");
	if bytes.len() <= 2048 {
		return value.clone();
	}
	json!({"truncated":true,"original_bytes":bytes.len(),"sha256":Sha256::digest(&bytes).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),"preview":String::from_utf8_lossy(&bytes).chars().take(256).collect::<String>()})
}
pub(super) fn bound(value: Value) -> Value {
	let bytes = serde_json::to_vec(&value).expect("JSON serializes");
	if bytes.len() <= ACTIVITY_BYTES {
		return value;
	}
	let mut bounded = json!({"truncation":{"truncated":true,"original_bytes":bytes.len(),"sha256":Sha256::digest(&bytes).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),"scope":"redacted activity; unlisted fields omitted"}});
	for key in [
		"kind",
		"event",
		"origin",
		"ts",
		"operation",
		"correlation",
		"trace",
		"entity",
		"reason",
		"request",
		"envelope",
		"dropped_before",
	] {
		if let Some(field) = value.get(key) {
			bounded[key] = summary(field);
		}
	}
	if let Some(outcome) = value.get("outcome") {
		if outcome.is_object() {
			let mut retained = json!({});
			for key in ["state", "error", "response"] {
				if let Some(field) = outcome.get(key) {
					retained[key] = if key == "error" && field.is_string() {
						json!(field
							.as_str()
							.unwrap()
							.chars()
							.take(512)
							.collect::<String>())
					} else {
						summary(field)
					};
				}
			}
			retained["truncation"] = summary(outcome);
			bounded["outcome"] = retained;
		} else {
			bounded["outcome"] = summary(outcome);
		}
	}
	bounded
}
