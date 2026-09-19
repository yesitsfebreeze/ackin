use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::transport::typed::{Adapter, Channel};

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;
pub const APPLICATION_ERROR: i64 = -32000;
pub const UNAUTHORIZED: i64 = -32001;
pub const NOT_PROVIDED: i64 = -32002;
pub const CLOSED: i64 = -32003;

pub const QUEUE: usize = 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
pub struct Error {
	pub code: i64,
	pub message: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub data: Option<Value>,
}

impl Error {
	pub fn new(code: i64, message: impl Into<String>) -> Self {
		Self {
			code,
			message: message.into(),
			data: None,
		}
	}

	pub fn application(message: impl Into<String>) -> Self {
		Self::new(APPLICATION_ERROR, message)
	}

	pub fn closed() -> Self {
		Self::new(CLOSED, "connection closed")
	}
}

impl From<Error> for String {
	fn from(e: Error) -> Self {
		e.message
	}
}

type Pending = HashMap<u64, oneshot::Sender<Result<Value, Error>>>;

struct Inner {
	out: mpsc::Sender<Value>,
	pending: Mutex<Option<Pending>>,
	next: AtomicU64,
	closed: AtomicBool,
	stop: CancellationToken,
	flushed: CancellationToken,
}

impl Inner {
	fn close(&self) {
		if self.closed.swap(true, Ordering::SeqCst) {
			return;
		}
		self.stop.cancel();
		let pending = self.pending.lock().expect("pending lock").take();
		for (_, waiter) in pending.unwrap_or_default() {
			let _ = waiter.send(Err(Error::closed()));
		}
	}
}

#[derive(Clone)]
pub struct Peer {
	inner: Arc<Inner>,
	_guard: Arc<Guard>,
}

/// Held only by [`Peer`] clones, never by the connection's own tasks, so the
/// last handle going away is what closes the connection.
struct Guard(Arc<Inner>);

impl Drop for Guard {
	fn drop(&mut self) {
		self.0.close();
	}
}

pub enum Incoming {
	Request(Request),
	Notification { method: String, params: Value },
}

pub struct Request {
	pub method: String,
	pub params: Value,
	id: Value,
	connection: Option<Arc<Inner>>,
}

impl Request {
	pub fn id(&self) -> Option<u64> {
		self.id.as_u64()
	}

	pub fn reply(mut self, result: Result<Value, Error>) {
		if let Some(connection) = self.connection.take() {
			connection.push(response(&self.id, result));
		}
	}
}

impl Drop for Request {
	fn drop(&mut self) {
		if let Some(connection) = self.connection.take() {
			connection.push(response(
				&self.id,
				Err(Error::new(
					INTERNAL_ERROR,
					"request dropped without a reply",
				)),
			));
		}
	}
}

impl Inner {
	fn push(&self, frame: Value) -> bool {
		match self.out.try_send(frame) {
			Ok(()) => true,
			Err(mpsc::error::TrySendError::Full(_)) => {
				self.close();
				false
			}
			Err(mpsc::error::TrySendError::Closed(_)) => false,
		}
	}
}

fn response(id: &Value, result: Result<Value, Error>) -> Value {
	match result {
		Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
		Err(error) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
	}
}

impl Peer {
	pub fn spawn<A: Adapter>(
		adapter: A,
		max_frame: Option<usize>,
	) -> (Peer, mpsc::Receiver<Incoming>) {
		let channel = match max_frame {
			Some(max) => Channel::with_max_frame(adapter, max),
			None => Channel::new(adapter),
		};
		let (mut reader, mut writer) = channel.into_split();
		let (out, mut outgoing) = mpsc::channel::<Value>(QUEUE);
		let stop = CancellationToken::new();
		let inner = Arc::new(Inner {
			out,
			pending: Mutex::new(Some(HashMap::new())),
			next: AtomicU64::new(1),
			closed: AtomicBool::new(false),
			stop,
			flushed: CancellationToken::new(),
		});
		let (incoming_tx, incoming) = mpsc::channel(QUEUE);

		let stopped = inner.stop.clone();
		let writer_inner = inner.clone();
		tokio::spawn(async move {
			loop {
				tokio::select! {
					frame = outgoing.recv() => match frame {
						Some(frame) => {
							if writer.send(frame).await.is_err() {
								break;
							}
						}
						None => break,
					},
					_ = stopped.cancelled() => {
						while let Ok(frame) = outgoing.try_recv() {
							if writer.send(frame).await.is_err() {
								break;
							}
						}
						break;
					}
				}
			}
			let _ = writer.close().await;
			writer_inner.close();
			writer_inner.flushed.cancel();
		});

		let stopped = inner.stop.clone();
		let reader_inner = inner.clone();
		tokio::spawn(async move {
			loop {
				let frame = tokio::select! {
					frame = reader.next() => frame,
					_ = stopped.cancelled() => break,
				};
				let frame = match frame {
					Some(Ok(frame)) => frame,
					Some(Err(error)) => {
						reader_inner.push(response(
							&Value::Null,
							Err(Error::new(PARSE_ERROR, error.to_string())),
						));
						break;
					}
					None => break,
				};
				tokio::select! {
					() = dispatch(&reader_inner, &incoming_tx, frame) => {}
					_ = stopped.cancelled() => break,
				}
			}
			reader_inner.close();
		});

		let guard = Arc::new(Guard(inner.clone()));
		(
			Peer {
				inner,
				_guard: guard,
			},
			incoming,
		)
	}

	pub async fn call(&self, method: &str, params: Value) -> Result<Value, Error> {
		let id = self.inner.next.fetch_add(1, Ordering::SeqCst);
		let (tx, rx) = oneshot::channel();
		{
			let mut pending = self.inner.pending.lock().expect("pending lock");
			let Some(pending) = pending.as_mut() else {
				return Err(Error::closed());
			};
			pending.insert(id, tx);
		}
		let waiting = Waiting { peer: self, id };
		let request = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
		if self.inner.out.send(request).await.is_err() {
			return Err(Error::closed());
		}
		let result = rx.await.unwrap_or_else(|_| Err(Error::closed()));
		drop(waiting);
		result
	}

	pub fn notify(&self, method: &str, params: Value) -> Result<(), Error> {
		if self.is_closed()
			|| !self
				.inner
				.push(json!({ "jsonrpc": "2.0", "method": method, "params": params }))
		{
			return Err(Error::closed());
		}
		Ok(())
	}

	pub fn is_closed(&self) -> bool {
		self.inner.closed.load(Ordering::SeqCst)
	}

	pub async fn closed(&self) {
		self.inner.stop.cancelled().await;
	}

	pub async fn flushed(&self) {
		self.inner.flushed.cancelled().await;
	}

	pub fn close(&self) {
		self.inner.close();
	}
}

struct Waiting<'a> {
	peer: &'a Peer,
	id: u64,
}

impl Drop for Waiting<'_> {
	fn drop(&mut self) {
		// A surviving entry is an abandoned call: `dispatch` removes it when a
		// response arrives, so removal succeeding here means a timeout, a
		// dropped future or a turn that ended — never a call that answered.
		let removed = self
			.peer
			.inner
			.pending
			.lock()
			.expect("pending lock")
			.as_mut()
			.is_some_and(|pending| pending.remove(&self.id).is_some());
		if removed {
			let _ = self.peer.notify("cancel", json!({ "id": self.id }));
		}
	}
}

async fn dispatch(inner: &Arc<Inner>, incoming: &mpsc::Sender<Incoming>, frame: Value) {
	let Value::Object(mut object) = frame else {
		inner.push(response(
			&Value::Null,
			Err(Error::new(INVALID_REQUEST, "a frame must be an object")),
		));
		return;
	};
	if let Some(method) = object
		.get("method")
		.and_then(Value::as_str)
		.map(str::to_owned)
	{
		let params = object.remove("params").unwrap_or(Value::Null);
		match object.remove("id").filter(|id| !id.is_null()) {
			Some(id) => {
				let request = Request {
					method,
					params,
					id,
					connection: Some(inner.clone()),
				};
				let _ = incoming.send(Incoming::Request(request)).await;
			}
			None => {
				let _ = incoming
					.send(Incoming::Notification { method, params })
					.await;
			}
		}
		return;
	}
	let Some(id) = object.get("id").and_then(Value::as_u64) else {
		return;
	};
	let result = match object.remove("error") {
		Some(error) => Err(serde_json::from_value(error)
			.unwrap_or_else(|_| Error::new(INTERNAL_ERROR, "malformed error object"))),
		None => Ok(object.remove("result").unwrap_or(Value::Null)),
	};
	let waiter = inner
		.pending
		.lock()
		.expect("pending lock")
		.as_mut()
		.and_then(|pending| pending.remove(&id));
	if let Some(waiter) = waiter {
		let _ = waiter.send(result);
	}
}

#[cfg(test)]
#[path = "tests/rpc.rs"]
mod tests;
