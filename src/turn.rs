//! The identifier one terminal turn carries across the runtimes cartridge spans.
//!
//! A turn is ambient, not a parameter: it is set once where work enters the
//! host (a socket `call`, a cartridge frame that already carries one) and read
//! wherever a diagnostic is written. [`stamp`] puts it on every outgoing wire
//! frame and [`of`] takes it off an incoming one, so the Rust host, a Lua
//! service and a cartridge process all name the same turn.
//!
//! `tokio::spawn` does not inherit a task-local, so every spawn that continues
//! a turn re-enters it with [`scope`].
//!
//! [`diagnostic`] is the channel the turn exists for: one JSON line per event on
//! a stream the protocol never uses, redacted by field name and bounded by a
//! byte cap with one rotated generation. `CARTRIDGE_DIAGNOSTICS` names the file (the
//! default is stderr) and `CARTRIDGE_DIAGNOSTICS_MAX_BYTES` the cap.

use serde_json::Value as Json;
use std::future::Future;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

tokio::task_local! {
	static TURN: Arc<str>;
}

/// The turn this task runs in, when it runs in one.
pub fn current() -> Option<Arc<str>> {
	TURN.try_with(Arc::clone).ok()
}

/// A fresh turn id, unique for the life of this process and unlikely to
/// collide with another cartridge's.
pub fn mint() -> Arc<str> {
	static NEXT: AtomicU64 = AtomicU64::new(1);
	static ORIGIN: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
	let origin = *ORIGIN.get_or_init(|| {
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map(|d| d.as_nanos() as u64)
			.unwrap_or_default()
	});
	Arc::from(format!(
		"{:x}-{}",
		origin,
		NEXT.fetch_add(1, Ordering::Relaxed)
	))
}

/// The turn a wire frame carries, or a fresh one when it carries none.
pub fn of(m: &Json) -> Arc<str> {
	m["turn"].as_str().map(Arc::from).unwrap_or_else(mint)
}

/// Put the ambient turn on an outgoing frame. A frame sent outside any turn is
/// left alone, so the wire never grows a field that names nothing.
pub fn stamp(m: &mut Json) {
	if let Some(id) = current() {
		m["turn"] = Json::String(id.to_string());
	}
}

/// Run `f` inside `id`.
pub async fn scope<F: Future>(id: Arc<str>, f: F) -> F::Output {
	TURN.scope(id, f).await
}

/// Continue the current turn inside a future that a `tokio::spawn` will run in
/// a task of its own, where the task-local would otherwise be lost.
pub fn carry<F: Future>(f: F) -> impl Future<Output = F::Output> {
	let id = current();
	async move {
		match id {
			Some(id) => scope(id, f).await,
			None => f.await,
		}
	}
}

/// Field names whose values a diagnostic never carries: credentials, prompt
/// bodies and tool bodies. Matched as a substring of the lowercased name, so
/// `api_key`, `Authorization` and `messages` are all caught.
const OMIT: &[&str] = &[
	"key",
	"token",
	"secret",
	"password",
	"credential",
	"auth",
	"cookie",
	"prompt",
	"message",
	"completion",
	"content",
	"body",
	"args",
	"input",
	"output",
	"result",
];

fn sensitive(name: &str) -> bool {
	let name = name.to_lowercase();
	OMIT.iter().any(|n| name.contains(n))
}

/// Replace every sensitive value in `v` with `"<omitted>"`, naming what went.
fn redact(v: &mut Json, omitted: &mut Vec<String>) {
	if let Json::Array(values) = v {
		for value in values {
			redact(value, omitted);
		}
		return;
	}
	let Json::Object(o) = v else { return };
	for (k, value) in o.iter_mut() {
		if sensitive(k) {
			*value = Json::String("<omitted>".into());
			omitted.push(k.clone());
		} else {
			redact(value, omitted);
		}
	}
}

/// The diagnostic stream: stderr, or a file bounded at `cap` bytes with one
/// rotated generation beside it. Nothing is truncated at startup — a file is
/// opened for append and counts what is already there.
struct Sink {
	out: Out,
	cap: u64,
	written: u64,
}

enum Out {
	Disabled,
	Stderr,
	File(PathBuf, std::fs::File),
}

impl Sink {
	fn file(path: PathBuf, cap: u64) -> std::io::Result<Self> {
		let file = std::fs::OpenOptions::new()
			.create(true)
			.append(true)
			.open(&path)?;
		let written = file.metadata().map(|m| m.len()).unwrap_or(0);
		Ok(Sink {
			out: Out::File(path, file),
			cap,
			written,
		})
	}

	fn from_env() -> Self {
		let cap = std::env::var("CARTRIDGE_DIAGNOSTICS_MAX_BYTES")
			.ok()
			.and_then(|v| v.parse().ok())
			.unwrap_or(8 << 20);
		match std::env::var("CARTRIDGE_DIAGNOSTICS") {
			Ok(p) if !p.is_empty() => Sink::file(PathBuf::from(p), cap).unwrap_or(Sink {
				out: Out::Stderr,
				cap,
				written: 0,
			}),
			_ => Sink {
				out: Out::Stderr,
				cap,
				written: 0,
			},
		}
	}

	fn write(&mut self, line: &str) {
		// Never split encoded JSON or a multibyte character. An oversized
		// record is replaced by a small valid record naming the omission.
		let replacement;
		let line = if line.len() as u64 > self.cap {
			replacement = format!(
				"{}\n",
				serde_json::json!({"omitted":"oversized diagnostic", "bytes":line.len()})
			);
			if replacement.len() as u64 <= self.cap {
				replacement.as_str()
			} else if self.cap >= 17 {
				"{\"omitted\":true}\n"
			} else {
				// A cap smaller than the minimal marker admits no record.
				return;
			}
		} else {
			line
		};
		let bytes = line.as_bytes();
		match &mut self.out {
			Out::Disabled => {}
			Out::Stderr => {
				let _ = std::io::stderr().write_all(bytes);
			}
			Out::File(path, file) => {
				if self.written > 0 && self.written + bytes.len() as u64 > self.cap {
					let mut rotated = path.clone().into_os_string();
					rotated.push(".1");
					let _ = std::fs::rename(&*path, rotated);
					match std::fs::File::create(&*path) {
						Ok(fresh) => *file = fresh,
						Err(_) => return,
					}
					self.written = 0;
				}
				if file.write_all(bytes).is_ok() {
					self.written += bytes.len() as u64;
				} else if file
					.set_len(self.written)
					.and_then(|()| file.seek(SeekFrom::End(0)).map(|_| ()))
					.is_err()
				{
					// Roll back a partial JSON record. If storage cannot be
					// repaired, stop this sink rather than append corrupt JSON.
					self.out = Out::Disabled;
				}
			}
		}
	}
}

fn sink() -> &'static Mutex<Sink> {
	static SINK: OnceLock<Mutex<Sink>> = OnceLock::new();
	SINK.get_or_init(|| Mutex::new(Sink::from_env()))
}

/// One diagnostic line: `{"t","turn","src","msg",…}`, sensitive fields omitted.
/// `fields` may carry its own `turn`, which wins over the ambient one — that is
/// how a line a cartridge wrote in its own turn keeps it.
pub fn diagnostic(src: &str, msg: impl std::fmt::Display, fields: Json) {
	let mut extra = match fields {
		Json::Object(o) => o,
		Json::Null => serde_json::Map::new(),
		other => serde_json::Map::from_iter([("data".to_owned(), other)]),
	};
	let id = extra
		.remove("turn")
		.and_then(|t| t.as_str().map(str::to_owned))
		.or_else(|| current().map(|i| i.to_string()));
	let mut omitted = Vec::new();
	let mut extra = Json::Object(extra);
	redact(&mut extra, &mut omitted);
	let Json::Object(extra) = extra else {
		unreachable!("redact keeps the shape")
	};
	let mut line = serde_json::Map::new();
	line.insert(
		"t".into(),
		Json::from(
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.map(|d| d.as_millis() as u64)
				.unwrap_or_default(),
		),
	);
	line.insert("turn".into(), id.map(Json::String).unwrap_or(Json::Null));
	line.insert("src".into(), Json::String(src.to_owned()));
	line.insert("msg".into(), Json::String(msg.to_string()));
	if !omitted.is_empty() {
		line.insert("omitted".into(), Json::from(omitted));
	}
	for (k, v) in extra {
		line.entry(k).or_insert(v);
	}
	sink()
		.lock()
		.unwrap_or_else(|e| e.into_inner())
		.write(&format!("{}\n", Json::Object(line)));
}

/// A line a cartridge wrote on its stderr. One it already shaped as a JSON
/// object keeps its fields; anything else is the message.
pub fn diagnostic_line(src: &str, line: &str) {
	match serde_json::from_str::<Json>(line) {
		Ok(Json::Object(mut o)) => {
			let msg = o
				.remove("msg")
				.or_else(|| o.remove("message"))
				.and_then(|m| m.as_str().map(str::to_owned))
				.unwrap_or_default();
			diagnostic(src, msg, Json::Object(o));
		}
		_ => diagnostic(src, line, Json::Null),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_credential_or_a_prompt_body_is_omitted_and_named() {
		let mut fields = serde_json::json!({
			"model": "m",
			"api_key": "sk-live-1",
			"request": { "messages": [{ "role": "user" }] },
		});
		let mut omitted = Vec::new();
		redact(&mut fields, &mut omitted);
		assert_eq!(fields["api_key"], "<omitted>");
		assert_eq!(fields["request"]["messages"], "<omitted>");
		assert_eq!(fields["model"], "m");
		omitted.sort();
		assert_eq!(omitted, ["api_key", "messages"]);
	}

	#[test]
	fn the_sink_stays_bounded_by_rotating_one_generation() {
		let dir = std::env::temp_dir().join(format!("cartridge-diag-{}", mint()));
		std::fs::create_dir_all(&dir).unwrap();
		let path = dir.join("cartridge.jsonl");
		let mut sink = Sink::file(path.clone(), 64).unwrap();
		for i in 0..50 {
			sink.write(&format!("{{\"n\":{i},\"pad\":\"xxxxxxxxxxxxxxxx\"}}\n"));
		}
		let live = std::fs::metadata(&path).unwrap().len();
		assert!(live <= 64, "live generation is {live} bytes, cap is 64");
		assert!(
			dir.join("cartridge.jsonl.1").exists(),
			"no rotated generation"
		);
		// Reopening keeps what is there; nothing truncates at startup.
		let reopened = Sink::file(path.clone(), 64).unwrap();
		assert_eq!(reopened.written, live);
		std::fs::remove_dir_all(&dir).unwrap();
	}
	#[test]
	fn redaction_reaches_objects_inside_nested_arrays() {
		let mut fields = serde_json::json!({"events":[[{"api_key":"synthetic-token", "safe":"kept"}]], "count":1});
		let mut omitted = Vec::new();
		redact(&mut fields, &mut omitted);
		assert_eq!(fields["events"][0][0]["api_key"], "<omitted>");
		assert_eq!(fields["events"][0][0]["safe"], "kept");
		assert_eq!(omitted, ["api_key"]);
		assert!(!fields.to_string().contains("synthetic-token"));
	}

	#[test]
	fn oversized_records_are_valid_json_and_bounded_before_and_after_rotation() {
		let dir = tempfile::tempdir().unwrap();
		let path = dir.path().join("diagnostic.jsonl");
		let mut sink = Sink::file(path.clone(), 64).unwrap();
		let huge = format!("{}\n", serde_json::json!({"text":"é🙂".repeat(100)}));
		sink.write(&huge);
		sink.write("{\"ordinary\":true}\n");
		sink.write(&huge);
		for file in [path, dir.path().join("diagnostic.jsonl.1")] {
			let data = std::fs::read_to_string(file).unwrap();
			assert!(data.len() <= 64);
			for line in data.lines() {
				let value: Json = serde_json::from_str(line).unwrap();
				assert!(value.get("omitted").is_some() || value["ordinary"] == true);
			}
		}
	}

	#[test]
	fn a_sink_that_cannot_repair_a_failed_write_stops_accepting_records() {
		let dir = tempfile::tempdir().unwrap();
		let path = dir.path().join("diagnostic.jsonl");
		std::fs::write(&path, "{\"ordinary\":true}\n").unwrap();
		let mut sink = Sink::file(path.clone(), 64).unwrap();
		sink.out = Out::File(path.clone(), std::fs::File::open(&path).unwrap());
		sink.write("{\"next\":1}\n");
		assert!(matches!(sink.out, Out::Disabled));
		sink.write("{\"next\":2}\n");
		assert_eq!(
			std::fs::read_to_string(path).unwrap(),
			"{\"ordinary\":true}\n"
		);
	}
}
