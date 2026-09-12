//! The stream: named channels a cartridge publishes to and any other node
//! subscribes to. One JSON envelope per event, appended under one lock, so the
//! log is ordered and replaying it is deterministic. Publishing with nobody
//! listening is a sequence bump, one append and no fan-out; subscribing and
//! unsubscribing are themselves envelopes on the channel, so membership is part
//! of the log any late subscriber replays.

use parking_lot::Mutex;
use serde_json::{json, Value as Json};
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
	Data,
	Subscribe,
	Unsubscribe,
	Error,
}

impl Kind {
	pub fn name(self) -> &'static str {
		match self {
			Kind::Data => "data",
			Kind::Subscribe => "subscribe",
			Kind::Unsubscribe => "unsubscribe",
			Kind::Error => "error",
		}
	}
}

/// What a subscriber holds: its id on the channel and the queue its events
/// arrive on. Dropping the receiver stops the subscription on the next publish;
/// [`Stream::unsubscribe`] is the explicit, announced way off.
pub struct Subscription {
	pub id: u64,
	pub rx: mpsc::UnboundedReceiver<Json>,
}

#[derive(Default)]
struct State {
	next: u64,
	channels: HashMap<String, Channel>,
}

#[derive(Default)]
struct Channel {
	seq: u64,
	/// Every envelope ever published here, in sequence order. Retention is not
	/// this module's policy; the log grows until a caller says otherwise.
	log: Vec<Json>,
	subs: Vec<Sub>,
}

struct Sub {
	id: u64,
	owner: String,
	tx: mpsc::UnboundedSender<Json>,
}

pub struct Stream(Mutex<State>);

impl Default for Stream {
	fn default() -> Self {
		Self::new()
	}
}

impl Stream {
	pub fn new() -> Self {
		Self(Mutex::new(State::default()))
	}

	/// Append one envelope and hand it to every listener. The sequence is the
	/// channel's next number whether or not anyone is listening, so a subscriber
	/// can always name how far it has read.
	pub fn publish(&self, channel: &str, from: &str, kind: Kind, data: Json) -> u64 {
		let mut st = self.0.lock();
		Self::append(&mut st, channel, from, kind, data)
	}

	/// Join a channel and receive everything on it from here on, oldest first.
	/// `after` resumes a subscription the subscriber lost: the envelopes after
	/// that sequence are handed over first, in order, so it ends up where it was.
	pub fn subscribe(&self, channel: &str, owner: &str, after: Option<u64>) -> Subscription {
		let mut st = self.0.lock();
		st.next += 1;
		let id = st.next;
		let (tx, rx): (mpsc::UnboundedSender<Json>, _) = mpsc::unbounded_channel();
		{
			let ch = st.channels.entry(channel.to_owned()).or_default();
			// The log is in sequence order, so the resume point is one binary
			// search; `None` starts from now and replays nothing.
			let past = match after {
				Some(seen) => {
					&ch.log[ch
						.log
						.partition_point(|e| e["seq"].as_u64().is_none_or(|s| s <= seen))..]
				}
				None => &ch.log[ch.log.len()..],
			};
			for envelope in past {
				let _ = tx.send(envelope.clone());
			}
			ch.subs.push(Sub {
				id,
				owner: owner.to_owned(),
				tx,
			});
		}
		// Same lock as the registration, so no envelope can land between the
		// replay and the announcement of the join.
		Self::append(
			&mut st,
			channel,
			owner,
			Kind::Subscribe,
			json!({ "id": id }),
		);
		Subscription { id, rx }
	}

	/// Leave a channel, announced: the envelope lands on the channel like any
	/// other, so everyone still listening knows the watch ended.
	pub fn unsubscribe(&self, channel: &str, id: u64) {
		let mut st = self.0.lock();
		let Some(ch) = st.channels.get_mut(channel) else {
			return;
		};
		let Some(index) = ch.subs.iter().position(|s| s.id == id) else {
			return;
		};
		let owner = ch.subs.remove(index).owner;
		Self::append(
			&mut st,
			channel,
			&owner,
			Kind::Unsubscribe,
			json!({ "id": id }),
		);
	}

	/// The envelopes after `after`, for a reader that wants the log without
	/// holding a subscription.
	pub fn replay(&self, channel: &str, after: u64) -> Vec<Json> {
		self.0
			.lock()
			.channels
			.get(channel)
			.map(|ch| {
				ch.log
					.iter()
					.filter(|e| e["seq"].as_u64().is_some_and(|s| s > after))
					.cloned()
					.collect()
			})
			.unwrap_or_default()
	}

	/// The one write path: sequence, log append, fan-out, in that order, under
	/// the caller's lock hold. A queue whose reader is gone is dropped here and
	/// then — the subscription is over, so the next announced leave is not owed.
	fn append(st: &mut State, channel: &str, from: &str, kind: Kind, data: Json) -> u64 {
		let ch = st.channels.entry(channel.to_owned()).or_default();
		ch.seq += 1;
		let envelope = json!({
			"ch": channel,
			"seq": ch.seq,
			"from": from,
			"kind": kind.name(),
			"data": data,
		});
		ch.log.push(envelope.clone());
		let mut gone = Vec::new();
		for (index, sub) in ch.subs.iter().enumerate() {
			if sub.tx.send(envelope.clone()).is_err() {
				gone.push(index);
			}
		}
		for index in gone.into_iter().rev() {
			ch.subs.remove(index);
		}
		ch.seq
	}
}
