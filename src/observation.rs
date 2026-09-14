//! Native dispatch metadata, written only to the existing opt-in diagnostic sink.
use crate::{lua::Host, runtime::Ctx, service::Service, trace};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

type Sink = Arc<dyn Fn(&str, Value) + Send + Sync>;
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Actor {
	actor: String,
	activity: String,
}
impl Actor {
	fn valid(&self) -> bool {
		matches!(self.actor.as_str(), "agent" | "ui" | "background")
			&& matches!(self.activity.as_str(), "deliberate" | "read" | "poll")
	}
}
struct Descriptor {
	generation: String,
	tool: String,
	revision: String,
}
struct Observer {
	origin: String,
	actors: BTreeMap<String, Actor>,
	cache: Mutex<VecDeque<Descriptor>>,
	sink: Sink,
}
impl Observer {
	fn new(raw: &str, sink: Sink) -> Self {
		// Every bound here is a setting under `host.observation_*`: the actor
		// map an environment may carry, how many actors it may name, and how
		// long a source name may be. They exist to keep a malformed environment
		// from becoming an unbounded one, not to ration a real deployment.
		let limit = crate::settings::host();
		let actors = if raw.len() <= limit.observation_actors_bytes {
			serde_json::from_str::<BTreeMap<String, Actor>>(raw)
				.ok()
				.filter(|map| map.len() <= limit.observation_actors_max)
				.unwrap_or_default()
				.into_iter()
				.filter(|(source, actor)| {
					!source.is_empty()
						&& source.len() <= limit.observation_source_chars
						&& actor.valid()
				})
				.collect()
		} else {
			BTreeMap::new()
		};
		Self {
			origin: trace::mint().to_string(),
			actors,
			cache: Mutex::new(VecDeque::new()),
			sink,
		}
	}
	fn revision(&self, generation: &str, tool: &str) -> Option<String> {
		self.cache
			.try_lock()
			.ok()?
			.iter()
			.find(|entry| entry.generation == generation && entry.tool == tool)
			.map(|entry| entry.revision.clone())
	}
	fn described(&self, generation: &str, tool: &str, revision: String) {
		if let Ok(mut cache) = self.cache.try_lock() {
			cache.retain(|entry| entry.generation != generation || entry.tool != tool);
			cache.push_back(Descriptor {
				generation: generation.into(),
				tool: tool.into(),
				revision,
			});
			while cache.len() > crate::settings::host().observation_cache_entries {
				cache.pop_front();
			}
		}
	}
	fn begin(self: &Arc<Self>, source: &str, tool: &str, op: &str) -> Guard {
		let actor = self.actors.get(source);
		let activity = match op {
			"describe" => Some("discovery"),
			"cancel" => Some("cancellation"),
			_ => actor.map(|actor| actor.activity.as_str()),
		};
		let value = json!({"v":1,"observation_id":trace::mint().to_string(),"source":source,"actor":actor.map(|actor|actor.actor.as_str()),"activity":activity,"tool":tool,"operation":op,"stage":"attempt","dispatched":false,"outcome":null,"completion_known":null,"elapsed_ms":null,"response_bytes":null,"provider_generation":null,"descriptor_revision":null,"descriptor_basis":"last_successful_describe","resumed":false});
		(self.sink)(source, value.clone());
		Guard {
			observer: self.clone(),
			state: Arc::new(Mutex::new(State {
				value,
				target: None,
				captured: false,
			})),
			started: Instant::now(),
			finished: false,
		}
	}
}
fn configured() -> Option<Arc<Observer>> {
	static OBSERVER: OnceLock<Option<Arc<Observer>>> = OnceLock::new();
	OBSERVER
		.get_or_init(|| {
			if std::env::var("CARTRIDGE_TOOL_OBSERVATIONS").as_deref() != Ok("1")
				|| !trace::diagnostics_enabled()
			{
				return None;
			}
			Some(Arc::new(Observer::new(
				&std::env::var("CARTRIDGE_TOOL_ACTORS").unwrap_or_default(),
				Arc::new(|source, fields| {
					let msg = if fields["stage"] == "attempt" {
						"tool_attempt"
					} else {
						"tool_completion"
					};
					trace::diagnostic(source, msg, fields);
				}),
			)))
		})
		.clone()
}
struct State {
	value: Value,
	target: Option<usize>,
	captured: bool,
}
struct Active {
	observer: Arc<Observer>,
	state: Arc<Mutex<State>>,
}
tokio::task_local! { static ACTIVE: Active; }

/// Called under the intended service's reload read gate. Nested service calls do not rebind it.
pub(crate) fn dispatch(service: &Service) {
	let _ = ACTIVE.try_with(|active| {
		let Ok(mut state) = active.state.lock() else {
			return;
		};
		if state.target != Some(service as *const Service as usize) || state.captured {
			return;
		}
		state.captured = true;
		let generation = format!("{}:{}", active.observer.origin, service.version());
		state.value["dispatched"] = json!(true);
		state.value["descriptor_revision"] = json!(active.observer.revision(
			&generation,
			state.value["tool"].as_str().unwrap_or_default()
		));
		state.value["provider_generation"] = json!(generation);
	});
}
/// A logical request spanning explicit reload-resume cannot claim one provider revision.
pub(crate) fn resuming(service: &Service) {
	let _ = ACTIVE.try_with(|active| {
		if let Ok(mut state) = active.state.lock() {
			if state.target == Some(service as *const Service as usize) {
				state.value["resumed"] = json!(true);
				state.value["provider_generation"] = Value::Null;
				state.value["descriptor_revision"] = Value::Null;
			}
		}
	});
}
struct Guard {
	observer: Arc<Observer>,
	state: Arc<Mutex<State>>,
	started: Instant,
	finished: bool,
}
impl Guard {
	fn finish(&mut self, result: Option<&Result<Value, String>>) {
		self.finished = true;
		let Ok(mut state) = self.state.lock() else {
			return;
		};
		let op = state.value["operation"]
			.as_str()
			.unwrap_or("unknown")
			.to_owned();
		let (outcome, known) = match result {
			None => ("interrupted", false),
			Some(Err(_)) if state.value["dispatched"] != true => ("lookup_failed", false),
			Some(Err(_)) => ("transport_error", false),
			Some(Ok(value)) if op == "describe" => {
				if let Some(revision) = descriptor(value) {
					if let Some(generation) = state.value["provider_generation"].as_str() {
						self.observer.described(
							generation,
							state.value["tool"].as_str().unwrap_or_default(),
							revision.clone(),
						);
					}
					state.value["descriptor_revision"] = json!(revision);
					("described", true)
				} else {
					("invalid_descriptor", false)
				}
			}
			Some(Ok(_)) if op == "cancel" => ("cancel_acknowledged", false),
			Some(Ok(value))
				if value["error"] == true && value["content"] == "interrupted-outcome-unknown" =>
			{
				("interrupted", false)
			}
			Some(Ok(value)) if value["content"].is_string() && value["error"].is_boolean() => (
				if value["error"] == true {
					"tool_error"
				} else {
					"success"
				},
				true,
			),
			Some(Ok(_)) => ("invalid_response", false),
		};
		state.value["stage"] = json!("completion");
		state.value["outcome"] = json!(outcome);
		state.value["completion_known"] = json!(known);
		state.value["elapsed_ms"] =
			json!(self.started.elapsed().as_millis().min(u64::MAX as u128) as u64);
		state.value["response_bytes"] =
			json!(result.and_then(|r| r.as_ref().ok()).and_then(json_size));
		let source = state.value["source"].as_str().unwrap_or_default();
		(self.observer.sink)(source, state.value.clone());
	}
}
impl Drop for Guard {
	fn drop(&mut self) {
		if !self.finished {
			self.finish(None);
		}
	}
}
fn descriptor(value: &Value) -> Option<String> {
	// Tool descriptors may be direct or wrapped in the established content envelope.
	let owned;
	let value = if value["content"].is_string() && value["error"] == false {
		let content = value["content"].as_str()?;
		if content.len() > crate::settings::host().observation_content_bytes {
			return None;
		}
		owned = serde_json::from_str::<Value>(content).ok()?;
		&owned
	} else {
		value
	};
	if !value["name"].is_string()
		|| !value["description"].is_string()
		|| !value["input_schema"].is_object()
	{
		return None;
	}
	let mut output = BoundedDigest {
		hash: Sha256::new(),
		bytes: 0,
		cap: crate::settings::host().observation_content_bytes,
	};
	serde_json::to_writer(&mut output, value).ok()?;
	Some(format!("{:x}", output.hash.finalize()))
}

struct Counter(usize);
impl std::io::Write for Counter {
	fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
		self.0 = self
			.0
			.checked_add(bytes.len())
			.ok_or_else(|| std::io::Error::other("size overflow"))?;
		Ok(bytes.len())
	}
	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}
fn json_size(value: &Value) -> Option<usize> {
	let mut count = Counter(0);
	serde_json::to_writer(&mut count, value).ok()?;
	Some(count.0)
}
struct BoundedDigest {
	hash: Sha256,
	bytes: usize,
	cap: usize,
}
impl std::io::Write for BoundedDigest {
	fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
		if bytes.len() > self.cap.saturating_sub(self.bytes) {
			return Err(std::io::Error::other("descriptor too large"));
		}
		self.bytes += bytes.len();
		self.hash.update(bytes);
		Ok(bytes.len())
	}
	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}

pub(crate) async fn invoke(
	host: &Host,
	ctx: &Ctx,
	source: &str,
	key: &str,
	args: Value,
) -> Result<Value, String> {
	let observer = key
		.starts_with("tool.")
		.then(configured)
		.flatten()
		.filter(|_| {
			source.len() <= crate::settings::host().observation_source_chars
				&& key.len() <= crate::settings::host().observation_key_chars
				&& key
					.bytes()
					.all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
		});
	let Some(observer) = observer else {
		return match ctx.get(key) {
			Ok(value) => host.invoke(value, args).await,
			Err(error) => Err(error.to_string()),
		};
	};
	let op = match args["op"].as_str() {
		Some("call") => "call",
		Some("describe") => "describe",
		Some("cancel") => "cancel",
		_ => "unknown",
	};
	let mut guard = observer.begin(source, key, op);
	let result = match ctx.get(key) {
		Err(error) => Err(error.to_string()),
		Ok(value) => {
			if let Ok(mut state) = guard.state.lock() {
				state.target = value
					.downcast_ref::<Service>()
					.map(|service| service as *const Service as usize);
				if state.target.is_none() {
					state.value["dispatched"] = json!(true);
				}
			}
			ACTIVE
				.scope(
					Active {
						observer: observer.clone(),
						state: guard.state.clone(),
					},
					host.invoke(value, args),
				)
				.await
		}
	};
	guard.finish(Some(&result));
	result
}

#[cfg(test)]
#[path = "../.cartridge/tests/unit/observation.rs"]
mod tests;
