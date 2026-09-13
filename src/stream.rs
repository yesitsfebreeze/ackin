//! The stream: named channels a cartridge publishes to and any other node
//! subscribes to. One JSON envelope per event, appended under a per-channel lock, so the
//! retained history is ordered and replaying it is deterministic. Publishing with nobody
//! listening is a sequence bump, one append and no fan-out; subscribing and
//! unsubscribing are themselves envelopes on the channel, so membership is part
//! of the log any late subscriber replays.

use parking_lot::{Mutex, RwLock};
use serde_json::{json, Value as Json};
use std::collections::{HashMap, VecDeque};
use std::sync::{
	atomic::{AtomicU64, Ordering},
	Arc,
};
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
	pub rx: mpsc::Receiver<Json>,
}

const HISTORY_EVENTS: usize = 256;
const HISTORY_BYTES: usize = 1024 * 1024;
// A full replay, an optional gap, the join, and one reserved error slot.
const SUBSCRIBER_EVENTS: usize = HISTORY_EVENTS + 3;

#[derive(Default)]
struct Channel {
	seq: u64,
	log: VecDeque<(Json, usize)>,
	bytes: usize,
	subs: Vec<Sub>,
}

struct Sub {
	id: u64,
	owner: String,
	tx: mpsc::Sender<Json>,
}

pub struct Stream {
	next: AtomicU64,
	channels: RwLock<HashMap<String, Arc<Mutex<Channel>>>>,
}

impl Default for Stream {
	fn default() -> Self {
		Self::new()
	}
}

impl Stream {
	pub fn new() -> Self {
		Self {
			next: AtomicU64::new(0),
			channels: RwLock::new(HashMap::new()),
		}
	}

	fn channel(&self, name: &str) -> Arc<Mutex<Channel>> {
		if let Some(channel) = self.channels.read().get(name) {
			return channel.clone();
		}
		self.channels
			.write()
			.entry(name.into())
			.or_default()
			.clone()
	}

	pub fn publish(&self, channel: &str, from: &str, kind: Kind, data: Json) -> u64 {
		Self::append(&mut self.channel(channel).lock(), channel, from, kind, data)
	}

	/// Replay is bounded. A cursor older than retained history receives an
	/// explicit gap before the retained events, so consumers can resynchronize.
	pub fn subscribe(&self, channel: &str, owner: &str, after: Option<u64>) -> Subscription {
		let ch = self.channel(channel);
		let mut ch = ch.lock();
		let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
		let (tx, rx) = mpsc::channel(SUBSCRIBER_EVENTS);
		if let Some(after) = after {
			for envelope in Self::history(&ch, channel, after) {
				let _ = tx.try_send(envelope);
			}
		}
		ch.subs.push(Sub {
			id,
			owner: owner.into(),
			tx,
		});
		Self::append(&mut ch, channel, owner, Kind::Subscribe, json!({"id":id}));
		Subscription { id, rx }
	}

	pub fn unsubscribe(&self, channel: &str, id: u64) {
		let Some(ch) = self.channels.read().get(channel).cloned() else {
			return;
		};
		let mut ch = ch.lock();
		let Some(index) = ch.subs.iter().position(|s| s.id == id) else {
			return;
		};
		let owner = ch.subs.remove(index).owner;
		Self::append(
			&mut ch,
			channel,
			&owner,
			Kind::Unsubscribe,
			json!({"id":id}),
		);
	}

	pub fn replay(&self, channel: &str, after: u64) -> Vec<Json> {
		let Some(ch) = self.channels.read().get(channel).cloned() else {
			return Vec::new();
		};
		let channel_state = ch.lock();
		Self::history(&channel_state, channel, after)
	}

	fn history(ch: &Channel, channel: &str, after: u64) -> Vec<Json> {
		let oldest = ch
			.log
			.front()
			.map_or(ch.seq + 1, |(e, _)| e["seq"].as_u64().unwrap());
		let mut events = Vec::new();
		if after.saturating_add(1) < oldest {
			events.push(json!({"ch":channel,"seq":oldest - 1,"kind":"error","from":"stream",
                "data":{"error":"stream history truncated; resynchronize","after":after,"oldest":oldest}}));
		}
		events.extend(
			ch.log
				.iter()
				.filter(|(e, _)| e["seq"].as_u64().unwrap() > after)
				.map(|(e, _)| e.clone()),
		);
		events
	}

	fn append(ch: &mut Channel, channel: &str, from: &str, kind: Kind, data: Json) -> u64 {
		ch.seq += 1;
		let envelope =
			json!({"ch":channel,"seq":ch.seq,"from":from,"kind":kind.name(),"data":data});
		let bytes = envelope.to_string().len();
		ch.bytes += bytes;
		ch.log.push_back((envelope.clone(), bytes));
		while ch.log.len() > HISTORY_EVENTS || ch.bytes > HISTORY_BYTES {
			ch.bytes -= ch.log.pop_front().unwrap().1;
		}
		ch.subs.retain(|sub| {
			// Reserve the last queue slot for a visible failure. Dropping the
			// sender then closes this subscription after it drains the notice.
			if sub.tx.capacity() <= 1 {
				let _ = sub.tx.try_send(
					json!({"ch":channel,"seq":ch.seq,"from":"stream","kind":"error",
                    "data":{"error":"slow stream subscriber; reconnect and resynchronize"}}),
				);
				return false;
			}
			sub.tx.try_send(envelope.clone()).is_ok()
		});
		ch.seq
	}
}
