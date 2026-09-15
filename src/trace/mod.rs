use serde_json::Value as Json;
use std::future::Future;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

tokio::task_local! {
	static TURN: Arc<str>;
}

pub fn current() -> Option<Arc<str>> {
	TURN.try_with(Arc::clone).ok()
}

/// Unique for the life of this process and unlikely to collide with another
/// cartridge's.
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

pub fn of(m: &Json) -> Arc<str> {
	m["trace"].as_str().map(Arc::from).unwrap_or_else(mint)
}

/// A frame sent outside any trace is left alone, so the wire never grows a
/// field that names nothing.
pub fn stamp(m: &mut Json) {
	if let Some(id) = current() {
		m["trace"] = Json::String(id.to_string());
	}
}

pub async fn scope<F: Future>(id: Arc<str>, f: F) -> F::Output {
	TURN.scope(id, f).await
}

/// Continue the current trace inside a future that a `tokio::spawn` will run in
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

/// Matched as a substring of the lowercased name, so `api_key`, `Authorization`
/// and `messages` are all caught.
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

fn redact(v: &mut Json, omitted: &mut Vec<String>) {
	if let Json::Array(values) = v {
		for value in values {
			redact(value, omitted);
		}
		return;
	}
	let Json::Object(o) = v else { return };
	for (k, value) in o.iter_mut() {
		// This typed observation fact contains no completion body. Strings under
		// the same spelling still receive the ordinary redaction.
		if k == "completion_known" && (value.is_boolean() || value.is_null()) {
			continue;
		}
		if sensitive(k) {
			*value = Json::String("<omitted>".into());
			omitted.push(k.clone());
		} else {
			redact(value, omitted);
		}
	}
}

/// Nothing is truncated at startup — a file is opened for append and counts
/// what is already there.
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
		// The cap is a setting (`host.diagnostics_max_bytes`); the environment
		// still wins, because turning diagnostics up for one invocation is what
		// the variable is for and editing a file to do it is not.
		let cap = std::env::var("CARTRIDGE_DIAGNOSTICS_MAX_BYTES")
			.ok()
			.and_then(|v| v.parse().ok())
			.unwrap_or_else(|| crate::settings::host().diagnostics_max_bytes);
		match std::env::var("CARTRIDGE_DIAGNOSTICS") {
			Ok(p) if p == "stderr" => Sink {
				out: Out::Stderr,
				cap,
				written: 0,
			},
			Ok(p) if !p.is_empty() && p != "off" => Sink::file(PathBuf::from(&p), cap)
				.unwrap_or_else(|err| {
					eprintln!("cartridge: {p}: {err}; diagnostics go to stderr");
					Sink {
						out: Out::Stderr,
						cap,
						written: 0,
					}
				}),
			_ => Sink {
				out: Out::Disabled,
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

pub(crate) fn diagnostics_enabled() -> bool {
	static ENABLED: OnceLock<bool> = OnceLock::new();
	*ENABLED.get_or_init(|| {
		std::env::var("CARTRIDGE_DIAGNOSTICS").is_ok_and(|v| !v.is_empty() && v != "off")
	})
}

enum Note {
	Line(String),
	Flush(std::sync::mpsc::SyncSender<()>),
}

static DROPPED: AtomicU64 = AtomicU64::new(0);

static OUT: OnceLock<std::sync::mpsc::SyncSender<Note>> = OnceLock::new();

fn drain(sink: &mut Sink, notes: &std::sync::mpsc::Receiver<Note>, dropped: &AtomicU64) {
	while let Ok(note) = notes.recv() {
		let missed = dropped.swap(0, Ordering::Relaxed);
		if missed > 0 {
			sink.write(&format!(
				"{}\n",
				serde_json::json!({ "t": now_ms(), "trace": Json::Null, "src": "cartridge",
					"msg": "diagnostics dropped", "dropped": missed })
			));
		}
		match note {
			Note::Line(line) => sink.write(&line),
			Note::Flush(ack) => {
				let _ = ack.send(());
			}
		}
	}
}

/// The sink is built on the first caller's thread, so it reads the environment
/// and working directory it reads today; the blocking write happens on the
/// writer thread.
fn out() -> &'static std::sync::mpsc::SyncSender<Note> {
	// Settle before entering the cell, not inside it: the initializer reads
	// `settings::host()`, and settling warns through `tracing::warn!` for each
	// refused value, which comes back here — re-entering a `OnceLock` from its
	// own initializer, which deadlocks. Only on the way to the first `out()`;
	// afterwards this is the cell's own check.
	if OUT.get().is_none() {
		let _ = crate::settings::host();
	}
	OUT.get_or_init(|| {
		let mut sink = Sink::from_env();
		let (lines, notes) =
			std::sync::mpsc::sync_channel::<Note>(crate::settings::host().diagnostics_queue);
		if std::thread::Builder::new()
			.name("cartridge-diagnostics".into())
			.spawn(move || drain(&mut sink, &notes, &DROPPED))
			.is_err()
		{
			// Visibly off, not quietly lossy.
			eprintln!("cartridge: no thread for the diagnostic stream; diagnostics are off");
		}
		lines
	})
}

fn now_ms() -> u64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|d| d.as_millis() as u64)
		.unwrap_or_default()
}

/// The writer dies with the process, and the next command may read this one's lines.
pub fn flush() {
	let Some(lines) = OUT.get() else { return };
	let (ack, done) = std::sync::mpsc::sync_channel::<()>(1);
	// Enqueue without blocking: the queue is full exactly when the writer
	// thread is stalled inside its own write, and a plain `send` would wait on
	// it past the cap below.
	let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
	let mut note = Note::Flush(ack);
	loop {
		match lines.try_send(note) {
			Ok(()) => break,
			Err(std::sync::mpsc::TrySendError::Disconnected(_)) => return,
			Err(std::sync::mpsc::TrySendError::Full(n)) => {
				if std::time::Instant::now() >= deadline {
					return;
				}
				note = n;
				std::thread::sleep(std::time::Duration::from_millis(10));
			}
		}
	}
	let _ = done.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()));
}

/// `fields` may carry its own `trace`, which wins over the ambient one — that
/// is how a line a cartridge wrote in its own trace keeps it.
pub fn diagnostic(src: &str, msg: impl std::fmt::Display, fields: Json) {
	if !diagnostics_enabled() {
		return;
	}
	let mut extra = match fields {
		Json::Object(o) => o,
		Json::Null => serde_json::Map::new(),
		other => serde_json::Map::from_iter([("data".to_owned(), other)]),
	};
	let id = extra
		.remove("trace")
		.and_then(|t| t.as_str().map(str::to_owned))
		.or_else(|| current().map(|i| i.to_string()));
	let mut omitted = Vec::new();
	let mut extra = Json::Object(extra);
	redact(&mut extra, &mut omitted);
	let Json::Object(extra) = extra else {
		unreachable!("redact keeps the shape")
	};
	let mut line = serde_json::Map::new();
	line.insert("t".into(), Json::from(now_ms()));
	line.insert("trace".into(), id.map(Json::String).unwrap_or(Json::Null));
	line.insert("src".into(), Json::String(src.to_owned()));
	line.insert("msg".into(), Json::String(msg.to_string()));
	if !omitted.is_empty() {
		line.insert("omitted".into(), Json::from(omitted));
	}
	for (k, v) in extra {
		line.entry(k).or_insert(v);
	}
	if out()
		.try_send(Note::Line(format!("{}\n", Json::Object(line))))
		.is_err()
	{
		DROPPED.fetch_add(1, Ordering::Relaxed);
	}
}

pub fn diagnostic_line(src: &str, line: &str) {
	if !diagnostics_enabled() {
		return;
	}
	match serde_json::from_str::<Json>(line) {
		Ok(Json::Object(mut o)) => {
			// A cartridge wrote this msg, so a non-string one clears the same
			// redaction as the rest of the line: the OMIT list above must not
			// be bypassed by nesting a field under it.
			let msg = o
				.remove("msg")
				.or_else(|| o.remove("message"))
				.map(|m| match m {
					Json::String(s) => s,
					Json::Null => String::new(),
					mut other => {
						redact(&mut other, &mut Vec::new());
						other.to_string()
					}
				})
				.unwrap_or_default();
			diagnostic(src, msg, Json::Object(o));
		}
		_ => diagnostic(src, line, Json::Null),
	}
}

// ---------------------------------------------------------------------------
// tracing
// ---------------------------------------------------------------------------

type Deliver = Arc<dyn Fn(&str, &str, Json) + Send + Sync>;

/// Only events from this crate are taken (targets under `cartridge`); a
/// dependency's chatter stays on stderr where the level filter governs it.
pub struct Layer {
	deliver: Deliver,
}

impl Default for Layer {
	fn default() -> Self {
		Self {
			deliver: Arc::new(|src, msg, fields| diagnostic(src, msg, fields)),
		}
	}
}

impl Layer {
	pub fn delivering(deliver: impl Fn(&str, &str, Json) + Send + Sync + 'static) -> Self {
		Self {
			deliver: Arc::new(deliver),
		}
	}
}

#[derive(Default)]
struct Fields {
	message: String,
	fields: serde_json::Map<String, Json>,
}

impl tracing::field::Visit for Fields {
	fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
		self.put(field, Json::String(format!("{value:?}")));
	}
	fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
		self.put(field, Json::String(value.to_owned()));
	}
	fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
		self.put(field, Json::from(value));
	}
	fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
		self.put(field, Json::from(value));
	}
	fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
		self.put(field, Json::from(value));
	}
	fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
		self.put(field, Json::Bool(value));
	}
	fn record_error(
		&mut self,
		field: &tracing::field::Field,
		value: &(dyn std::error::Error + 'static),
	) {
		self.put(field, Json::String(value.to_string()));
	}
}

impl Fields {
	fn put(&mut self, field: &tracing::field::Field, value: Json) {
		if field.name() == "message" {
			self.message = match value {
				Json::String(s) => s,
				other => other.to_string(),
			};
		} else {
			self.fields.insert(field.name().to_owned(), value);
		}
	}
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Layer {
	fn on_event(
		&self,
		event: &tracing::Event<'_>,
		_ctx: tracing_subscriber::layer::Context<'_, S>,
	) {
		let target = event.metadata().target();
		if !target.starts_with("cartridge") {
			return;
		}
		let mut fields = Fields::default();
		event.record(&mut fields);
		fields.fields.insert(
			"level".into(),
			Json::String(event.metadata().level().as_str().to_lowercase()),
		);
		let src = match fields.fields.remove("cartridge") {
			Some(Json::String(cartridge)) => cartridge,
			_ => target.to_owned(),
		};
		(self.deliver)(&src, &fields.message, Json::Object(fields.fields));
	}
}

/// Idempotent — a second call, or a call after the embedding program installed
/// a subscriber of its own, changes nothing.
pub fn subscribe() {
	use tracing_subscriber::layer::SubscriberExt;
	use tracing_subscriber::util::SubscriberInitExt;
	use tracing_subscriber::{EnvFilter, Layer as _};
	let filter =
		EnvFilter::try_from_env("CARTRIDGE_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
	let stderr = tracing_subscriber::fmt::layer()
		.with_writer(std::io::stderr)
		.with_target(false)
		.without_time()
		.with_filter(filter);
	let _ = tracing_subscriber::registry()
		.with(stderr)
		.with(Layer::default())
		.try_init();
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/trace/tests.rs"]
mod tests;
