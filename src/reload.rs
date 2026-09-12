//! The generation switch. A call in flight holds the gate; a replacement takes
//! it exclusively and bumps the epoch, so a resumable call wakes on the
//! generation that replaced the one it started on. Nothing here stores data.
use std::sync::{
	atomic::{AtomicBool, Ordering},
	Arc,
};

/// One entry's reload transaction, shared by every generation of that entry.
#[derive(Clone)]
pub struct Reload {
	gate: Arc<tokio::sync::RwLock<()>>,
	epoch: tokio::sync::watch::Sender<u64>,
	pending: Arc<AtomicBool>,
}

impl Default for Reload {
	fn default() -> Self {
		Self {
			gate: Arc::default(),
			epoch: tokio::sync::watch::channel(0).0,
			pending: Arc::default(),
		}
	}
}

impl Reload {
	/// Read-held by every call; write-held for the span of a switch.
	pub(crate) fn gate(&self) -> Arc<tokio::sync::RwLock<()>> {
		self.gate.clone()
	}
	/// Bumped once per completed switch, so a waiter can tell generations apart.
	pub(crate) fn epoch(&self) -> tokio::sync::watch::Receiver<u64> {
		self.epoch.subscribe()
	}
	/// Claim the transaction. `false` means one is already in flight.
	pub(crate) fn begin(&self) -> bool {
		!self.pending.swap(true, Ordering::SeqCst)
	}
	pub(crate) fn pending(&self) -> bool {
		self.pending.load(Ordering::SeqCst)
	}
	pub(crate) fn finish(&self) {
		self.pending.store(false, Ordering::SeqCst);
		self.epoch.send_modify(|n| *n += 1);
	}
}
