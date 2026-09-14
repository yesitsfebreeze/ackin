//! JSON-RPC 2.0 over one connection.
//!
//! A [`Peer`] owns a connection's reader and writer tasks. Calls made through
//! it run concurrently: each request gets a fresh id and waits on its own
//! reply, so a slow answer never holds up the next call. Requests and
//! notifications the other side sends arrive on the [`Incoming`] receiver
//! [`Peer::spawn`] returns; a [`Request`] is answered through
//! [`Request::reply`], and one dropped unanswered is answered with an internal
//! error, so the caller is never left waiting.
//!
//! Framing is one JSON object per line ([`crate::transport::typed::JsonEnvelopeCodec`]).
//! Every frame written carries `"jsonrpc": "2.0"`; frames read are accepted with
//! or without it.

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
/// The handler ran and failed; the message is its error.
pub const APPLICATION_ERROR: i64 = -32000;
/// No `auth`, a wrong token, or a request outside the token's grant.
pub const UNAUTHORIZED: i64 = -32001;
/// The server does not provide the key or event asked for.
pub const NOT_PROVIDED: i64 = -32002;
/// Local only, never on the wire: the connection is gone.
pub const CLOSED: i64 = -32003;

/// A JSON-RPC error object.
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
	out: mpsc::UnboundedSender<Value>,
	pending: Mutex<Option<Pending>>,
	next: AtomicU64,
	closed: AtomicBool,
	stop: CancellationToken,
	/// Cancelled once the writer has flushed and shut its half.
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

/// One end of a JSON-RPC connection. Clones share the connection; dropping
/// the last clone closes it.
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

/// What the other side sent that is not a reply.
pub enum Incoming {
	Request(Request),
	Notification { method: String, params: Value },
}

/// A request waiting for its answer.
pub struct Request {
	pub method: String,
	pub params: Value,
	id: Value,
	out: Option<mpsc::UnboundedSender<Value>>,
}

impl Request {
	/// Answer the request. Consumes it, so it is answered once.
	pub fn reply(mut self, result: Result<Value, Error>) {
		if let Some(out) = self.out.take() {
			let _ = out.send(response(&self.id, result));
		}
	}
}

impl Drop for Request {
	fn drop(&mut self) {
		if let Some(out) = self.out.take() {
			let _ = out.send(response(
				&self.id,
				Err(Error::new(
					INTERNAL_ERROR,
					"request dropped without a reply",
				)),
			));
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
	/// Take over a connection. Frames longer than `max_frame` bytes, when
	/// given, close it: the limit is what a server that reads a token from an
	/// unauthenticated peer needs.
	pub fn spawn<A: Adapter>(
		adapter: A,
		max_frame: Option<usize>,
	) -> (Peer, mpsc::UnboundedReceiver<Incoming>) {
		let channel = match max_frame {
			Some(max) => Channel::with_max_frame(adapter, max),
			None => Channel::new(adapter),
		};
		let (mut reader, mut writer) = channel.into_split();
		let (out, mut outgoing) = mpsc::unbounded_channel::<Value>();
		let stop = CancellationToken::new();
		let inner = Arc::new(Inner {
			out: out.clone(),
			pending: Mutex::new(Some(HashMap::new())),
			next: AtomicU64::new(1),
			closed: AtomicBool::new(false),
			stop,
			flushed: CancellationToken::new(),
		});
		let (incoming_tx, incoming) = mpsc::unbounded_channel();

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
						// Flush what is already queued, such as the reply to the
						// request that asked for the close.
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
						let _ = out.send(response(
							&Value::Null,
							Err(Error::new(PARSE_ERROR, error.to_string())),
						));
						break;
					}
					None => break,
				};
				dispatch(&reader_inner, &out, &incoming_tx, frame);
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

	/// Call `method` and wait for its result.
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
		if self.inner.out.send(request).is_err() {
			return Err(Error::closed());
		}
		let result = rx.await.unwrap_or_else(|_| Err(Error::closed()));
		drop(waiting);
		result
	}

	/// Send a notification; nothing comes back.
	pub fn notify(&self, method: &str, params: Value) -> Result<(), Error> {
		if self.is_closed() {
			return Err(Error::closed());
		}
		self.inner
			.out
			.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }))
			.map_err(|_| Error::closed())
	}

	pub fn is_closed(&self) -> bool {
		self.inner.closed.load(Ordering::SeqCst)
	}

	/// Resolves once the connection is gone.
	pub async fn closed(&self) {
		self.inner.stop.cancelled().await;
	}

	/// Resolves once the connection is gone and every frame queued before the
	/// close has been written.
	pub async fn flushed(&self) {
		self.inner.flushed.cancelled().await;
	}

	/// Close the connection: waiting calls fail, queued frames are flushed.
	pub fn close(&self) {
		self.inner.close();
	}
}

/// A call's registration belongs to the waiting future: a caller that gives up
/// takes its entry out of the table.
struct Waiting<'a> {
	peer: &'a Peer,
	id: u64,
}

impl Drop for Waiting<'_> {
	fn drop(&mut self) {
		if let Some(pending) = self
			.peer
			.inner
			.pending
			.lock()
			.expect("pending lock")
			.as_mut()
		{
			pending.remove(&self.id);
		}
	}
}

fn dispatch(
	inner: &Inner,
	out: &mpsc::UnboundedSender<Value>,
	incoming: &mpsc::UnboundedSender<Incoming>,
	frame: Value,
) {
	let Value::Object(mut object) = frame else {
		let _ = out.send(response(
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
					out: Some(out.clone()),
				};
				// A receiver nobody reads drops the request, which answers it.
				let _ = incoming.send(Incoming::Request(request));
			}
			None => {
				let _ = incoming.send(Incoming::Notification { method, params });
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
