//! One end of the wire: the lines going out, the replies still owed, and
//! the remote a provided key is on the other end of it.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use serde_json::{json, Value as Json};
use tokio::sync::{mpsc, oneshot};

use crate::error::{Error, Result};

type Pending = HashMap<u64, oneshot::Sender<Result<Json>>>;

/// One end of the wire: outgoing lines and the replies still owed. `None` on
/// the channel stops the writer. Both ends use it — the daemon's here, the
/// cartridge's in [`crate::sdk`] — so `gone` names whichever peer went away.
pub struct Link {
	pub(super) reload: AtomicBool,
	tx: mpsc::UnboundedSender<Option<Json>>,
	pending: Mutex<Option<Pending>>,
	/// Stream subscriptions this end holds, by channel. The daemon's side uses
	/// it to answer `{"unsubscribe"}`; the cartridge's side never subscribes
	/// over its own link, so the map stays empty there.
	subs: Mutex<HashMap<String, u64>>,
	next: AtomicU64,
	gone: &'static str,
}

impl Link {
	pub fn new(tx: mpsc::UnboundedSender<Option<Json>>, gone: &'static str) -> Arc<Self> {
		Arc::new(Self {
			reload: AtomicBool::new(false),
			tx,
			pending: Mutex::new(Some(HashMap::new())),
			subs: Mutex::new(HashMap::new()),
			next: AtomicU64::new(1),
			gone,
		})
	}

	pub(crate) fn send(&self, m: Json) {
		let _ = self.tx.send(Some(m));
	}

	/// Like [`Link::send`], but tells the caller whether the pipe still took the
	/// frame — the stream forwarders exit on the first `false` and announce
	/// their own leaving, so a dead subscriber is never queued into.
	pub(crate) fn try_send(&self, m: Json) -> bool {
		self.tx.send(Some(m)).is_ok()
	}

	pub(crate) fn set_sub(&self, channel: &str, id: Option<u64>) {
		let mut subs = self.subs.lock();
		match id {
			Some(id) => subs.insert(channel.to_owned(), id),
			None => subs.remove(channel),
		};
	}

	pub(crate) fn sub_id(&self, channel: &str) -> Option<u64> {
		self.subs.lock().get(channel).copied()
	}

	/// Announce every stream subscription this end still holds. Called when the
	/// link is going away for good, so a dead watcher is a `leave` event on the
	/// channel, not a silence.
	pub(crate) fn leave_subs(&self, stream: &crate::stream::Stream) {
		for (channel, id) in std::mem::take(&mut *self.subs.lock()) {
			stream.unsubscribe(&channel, id);
		}
	}

	/// Stop the writer once everything already queued is out.
	pub(crate) fn stop(&self) {
		let _ = self.tx.send(None);
	}

	/// The peer went away: the error every waiter on this link gets.
	fn gone(&self) -> Error {
		Error::Gone(self.gone)
	}

	pub(crate) async fn request(&self, mut m: Json) -> Result<Json> {
		let id = self.next.fetch_add(1, Ordering::SeqCst);
		let (tx, rx) = oneshot::channel();
		{
			let mut pending = self.pending.lock();
			let Some(pending) = pending.as_mut() else {
				return Err(self.gone());
			};
			pending.insert(id, tx);
		}
		let _waiting = Waiting { link: self, id };
		m["id"] = json!(id);
		crate::trace::stamp(&mut m);
		if self.tx.send(Some(m)).is_err() {
			self.close();
			return Err(self.gone());
		}
		rx.await.unwrap_or_else(|_| Err(self.gone()))
	}

	pub(crate) fn answer(&self, id: u64, r: Result<Json>) {
		if let Some(tx) = self.pending.lock().as_mut().and_then(|p| p.remove(&id)) {
			let _ = tx.send(r);
		}
	}

	#[cfg(test)]
	pub(crate) fn pending_count(&self) -> usize {
		self.pending.lock().as_ref().map_or(0, HashMap::len)
	}

	/// Answer a request the peer made. An error crosses the wire as its text,
	/// whichever kind it was on this side.
	pub(crate) fn reply<E: std::fmt::Display>(&self, id: u64, r: std::result::Result<Json, E>) {
		self.send(match r {
			Ok(data) => json!({ "reply": id, "data": data }),
			Err(error) => json!({ "reply": id, "error": error.to_string() }),
		});
	}

	/// The inverse of [`Link::reply`]: hand a reply frame back to its waiter.
	/// Returns whether `m` was one, so both ends decode the envelope identically.
	pub fn accept(&self, m: &Json) -> bool {
		let Some(id) = m["reply"].as_u64() else {
			return false;
		};
		self.answer(
			id,
			match m["error"].as_str() {
				Some(e) => Err(Error::remote(e)),
				None => Ok(m["data"].clone()),
			},
		);
		true
	}

	pub(crate) fn close(&self) {
		for (_, tx) in self.pending.lock().take().unwrap_or_default() {
			let _ = tx.send(Err(self.gone()));
		}
	}

	/// Fail every waiter, then stop the writer once the queue drains.
	pub fn shutdown(&self) {
		self.close();
		self.stop();
	}
}

// The registration belongs to the waiting future, not to its remote handler.
struct Waiting<'a> {
	link: &'a Link,
	id: u64,
}

impl Drop for Waiting<'_> {
	fn drop(&mut self) {
		if let Some(pending) = self.link.pending.lock().as_mut() {
			pending.remove(&self.id);
		}
	}
}

/// A key provided by a process cartridge.
#[derive(Clone)]
pub struct Remote {
	pub(super) link: Arc<Link>,
	pub(super) key: String,
}

impl Remote {
	/// A remote over a link this crate did not open itself: a chain node's
	/// client side to its dependency's socket speaks the same frames, so the
	/// need a document declares is a remote value the Lua surface wraps.
	pub fn over(link: Arc<Link>, key: String) -> Self {
		Self { link, key }
	}

	pub(crate) fn process_id(&self) -> usize {
		Arc::as_ptr(&self.link) as usize
	}
	pub(crate) async fn cancel_reload(&self) {
		if self.link.reload.load(Ordering::Relaxed) {
			let _ = self.link.request(json!({"reload":false})).await;
		}
	}
	pub(crate) async fn prepare_reload(&self) -> Result<()> {
		if self.link.reload.load(Ordering::Relaxed) {
			self.link.request(json!({"reload":true})).await?;
		}
		Ok(())
	}
	pub async fn call(&self, args: Json) -> Result<Json> {
		self.link
			.request(json!({ "call": self.key, "args": args }))
			.await
	}
}
