use std::collections::{BTreeMap, HashMap, VecDeque};
use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use futures::future::BoxFuture;
use futures::FutureExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::transport::rpc::{self, Incoming, Peer, Request};
use crate::transport::typed::{AdapterError, BindOutcome, Endpoint, LocalAdapter, LocalListener};

pub const SOCKET_ENV: &str = "CARTRIDGE_SOCKET";
pub const NODE_TOKEN_ENV: &str = "CARTRIDGE_NODE_TOKEN";
pub const CONNECT_TIMEOUT_ENV: &str = "CARTRIDGE_CONNECT_TIMEOUT_SECS";

const MAX_FRAME: usize = 64 * 1024 * 1024;
const HISTORY: usize = 1024;

pub type Result<T> = std::result::Result<T, String>;

const IN_FLIGHT: usize = 64;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Directory {
	#[serde(default)]
	pub events: BTreeMap<String, EventEntry>,
	#[serde(default)]
	pub sends: Vec<String>,
	#[serde(default)]
	pub accept: BTreeMap<String, Accept>,
	#[serde(default)]
	pub needs: Vec<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub host: Option<Address>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EventEntry {
	pub owner: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub schema: Option<Value>,
	#[serde(default)]
	pub timeout_ms: u64,
	#[serde(default)]
	pub listeners: Vec<Address>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
	pub cartridge: String,
	pub socket: PathBuf,
	pub token: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Accept {
	pub from: String,
	#[serde(default)]
	pub events: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
	Answered { from: String, data: Value },
	Declined { from: String },
	Failed { from: String, error: String },
	TimedOut { from: String },
	Unavailable { from: String, error: String },
}

impl Outcome {
	pub fn from(&self) -> &str {
		match self {
			Outcome::Answered { from, .. }
			| Outcome::Declined { from }
			| Outcome::Failed { from, .. }
			| Outcome::TimedOut { from }
			| Outcome::Unavailable { from, .. } => from,
		}
	}

	pub fn error(&self) -> Option<String> {
		match self {
			Outcome::Answered { .. } | Outcome::Declined { .. } => None,
			Outcome::Failed { from, error } => Some(format!("{from}: {error}")),
			Outcome::TimedOut { from } => Some(format!("{from} did not answer in time")),
			Outcome::Unavailable { from, error } => Some(format!("{from} is unavailable: {error}")),
		}
	}
}

#[derive(Clone)]
enum Access {
	Host,
	Peer(String),
}

impl Access {
	fn is_host(&self) -> bool {
		matches!(self, Access::Host)
	}
}

tokio::task_local! {
	static TRACE: String;
}

pub fn trace() -> Option<String> {
	TRACE.try_with(Clone::clone).ok()
}

fn trace_of(params: &Value) -> String {
	params["trace"]
		.as_str()
		.map(str::to_owned)
		.or_else(trace)
		.unwrap_or_else(|| crate::transport::token()[..16].to_owned())
}

type Handler = Arc<dyn Fn(Value) -> BoxFuture<'static, Result<Value>> + Send + Sync>;
type Finalizer = Box<dyn FnOnce() -> BoxFuture<'static, ()> + Send>;
pub type Apply = Box<dyn FnOnce(Ctx, Value) -> BoxFuture<'static, Result<()>> + Send>;

fn handler<F, Fut>(f: F) -> Handler
where
	F: Fn(Value) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Result<Value>> + Send + 'static,
{
	Arc::new(move |value| f(value).boxed())
}

struct Prepared {
	listeners: Vec<Address>,
	timeout: Option<Duration>,
	trace: String,
}

#[derive(Clone)]
pub struct Ctx {
	state: Arc<State>,
}

struct State {
	name: RwLock<String>,
	host_token: String,
	connect_timeout: Duration,
	directory: RwLock<Directory>,
	validators: Mutex<HashMap<String, Arc<jsonschema::Validator>>>,
	events: RwLock<HashMap<String, Handler>>,
	finalizers: Mutex<Vec<Finalizer>>,
	channels: Mutex<HashMap<String, Channel>>,
	clients: Mutex<HashMap<String, Arc<tokio::sync::Mutex<Option<Peer>>>>>,
	watchers: Mutex<HashMap<(String, String), Watcher>>,
	connections: Mutex<Vec<Peer>>,
	next: AtomicU64,
	stop: CancellationToken,
}

#[derive(Default)]
struct Channel {
	seq: u64,
	history: VecDeque<Value>,
	subscribers: Vec<(u64, Peer)>,
}

struct Watcher {
	tx: mpsc::Sender<Value>,
	last: Arc<AtomicU64>,
}

impl Ctx {
	pub fn new(host_token: impl Into<String>, connect_timeout: Duration) -> Self {
		Self {
			state: Arc::new(State {
				name: RwLock::new(String::new()),
				host_token: host_token.into(),
				connect_timeout,
				directory: RwLock::default(),
				validators: Mutex::default(),
				events: RwLock::default(),
				finalizers: Mutex::default(),
				channels: Mutex::default(),
				clients: Mutex::default(),
				watchers: Mutex::default(),
				connections: Mutex::default(),
				next: AtomicU64::new(1),
				stop: CancellationToken::new(),
			}),
		}
	}

	pub fn name(&self) -> String {
		self.state.name.read().expect("name lock").clone()
	}

	pub fn directory(&self) -> Directory {
		self.state.directory.read().expect("directory lock").clone()
	}

	pub fn needs(&self) -> Vec<String> {
		self.state
			.directory
			.read()
			.expect("directory lock")
			.needs
			.clone()
	}

	pub fn events(&self) -> BTreeMap<String, EventEntry> {
		self.state
			.directory
			.read()
			.expect("directory lock")
			.events
			.clone()
	}

	pub(crate) fn set_directory(&self, directory: Directory) {
		// Held until validators are cleared: a validator compiled from the old
		// directory must never be cached after this clears it.
		let mut held = self.state.directory.write().expect("directory lock");
		*held = directory;
		self.state
			.validators
			.lock()
			.expect("validators lock")
			.clear();
	}

	pub fn validate(&self, name: &str, data: &Value) -> Result<()> {
		// Read-locked until the validator is cached, so `set_directory` can't
		// clear the cache in between; both paths lock directory then validators.
		let directory = self.state.directory.read().expect("directory lock");
		let schema = directory
			.events
			.get(name)
			.ok_or_else(|| format!("`{name}` is not an event any cartridge declares"))?
			.schema
			.clone();
		let Some(schema) = schema else {
			return Ok(());
		};
		let validator = {
			let mut validators = self.state.validators.lock().expect("validators lock");
			match validators.get(name) {
				Some(validator) => validator.clone(),
				None => {
					let validator = Arc::new(
						jsonschema::validator_for(&schema)
							.map_err(|e| format!("`{name}` has an invalid schema: {e}"))?,
					);
					validators.insert(name.to_owned(), validator.clone());
					validator
				}
			}
		};
		let errors: Vec<String> = validator
			.iter_errors(data)
			.map(|e| format!("{} at {}", e, e.instance_path()))
			.collect();
		if errors.is_empty() {
			Ok(())
		} else {
			Err(format!(
				"`{name}` payload rejected by its schema: {}",
				errors.join("; ")
			))
		}
	}

	fn prepare(&self, name: &str, data: &Value) -> Result<Prepared> {
		self.validate(name, data)?;
		let directory = self.state.directory.read().expect("directory lock");
		let entry = match directory.events.get(name) {
			Some(entry) => entry.clone(),
			None => {
				return Err(format!(
					"`{name}` is not an event this cartridge defines or needs"
				));
			}
		};
		if !directory.sends.iter().any(|n| n == name) {
			return Err(format!(
				"`{name}` is not an event this cartridge defines or needs"
			));
		}
		Ok(Prepared {
			listeners: entry.listeners,
			timeout: (entry.timeout_ms > 0).then(|| Duration::from_millis(entry.timeout_ms)),
			trace: trace_of(&Value::Null),
		})
	}

	fn accepts(&self, token: &str, name: &str) -> bool {
		self.state
			.directory
			.read()
			.expect("directory lock")
			.accept
			.get(token)
			.is_some_and(|accept| accept.events.iter().any(|n| n == name))
	}

	pub fn on<F, Fut>(&self, name: &str, f: F)
	where
		F: Fn(Value) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<Value>> + Send + 'static,
	{
		self.state
			.events
			.write()
			.expect("events lock")
			.insert(name.to_owned(), handler(f));
	}

	pub fn on_dispose<F, Fut>(&self, f: F)
	where
		F: FnOnce() -> Fut + Send + 'static,
		Fut: Future<Output = ()> + Send + 'static,
	{
		self.state
			.finalizers
			.lock()
			.expect("finalizers lock")
			.push(Box::new(move || f().boxed()));
	}

	pub fn stop(&self) {
		self.state.stop.cancel();
	}

	pub async fn host(&self, method: &str, params: Value) -> Result<Value> {
		let address = self
			.state
			.directory
			.read()
			.expect("directory lock")
			.host
			.clone()
			.ok_or_else(|| "no host address in the directory".to_owned())?;
		let peer = self.client(&address).await?;
		Ok(peer.call(method, params).await?)
	}

	async fn send(
		&self,
		address: &Address,
		name: &str,
		data: Value,
		prepared: &Prepared,
	) -> Outcome {
		let from = address.cartridge.clone();
		let call = async {
			let peer = match self.client(address).await {
				Ok(peer) => peer,
				Err(error) => {
					return Outcome::Unavailable {
						from: from.clone(),
						error,
					}
				}
			};
			let params = json!({ "name": name, "data": data, "trace": prepared.trace });
			match peer.call("event", params).await {
				Ok(Value::Null) => Outcome::Declined { from: from.clone() },
				Ok(data) => Outcome::Answered {
					from: from.clone(),
					data,
				},
				Err(error) if error.code == rpc::CLOSED => Outcome::Unavailable {
					from: from.clone(),
					error: error.message,
				},
				Err(error) => Outcome::Failed {
					from: from.clone(),
					error: error.message,
				},
			}
		};
		let started = std::time::Instant::now();
		let outcome = match prepared.timeout {
			Some(timeout) => tokio::time::timeout(timeout, call)
				.await
				.unwrap_or(Outcome::TimedOut { from }),
			None => call.await,
		};
		// Every path out of an event — emit, bail, call — funnels through here,
		// so this is the one place a failure is certain to reach the daemon log.
		// Without it a timeout only ever reaches the caller, and the proxy's
		// advice to read the log names a file that never recorded the cause.
		if let Some(error) = outcome.error() {
			tracing::warn!(
				target: "cartridge",
				trace = %prepared.trace,
				event = name,
				ms = started.elapsed().as_millis() as u64,
				"{error}"
			);
		}
		outcome
	}

	pub fn emit(&self, name: &str, data: Value) -> Result<()> {
		let prepared = Arc::new(self.prepare(name, &data)?);
		for address in prepared.listeners.clone() {
			let (ctx, name, data, prepared) = (
				self.clone(),
				name.to_owned(),
				data.clone(),
				prepared.clone(),
			);
			tokio::spawn(async move {
				let outcome = ctx.send(&address, &name, data, &prepared).await;
				if let Some(error) = outcome.error() {
					ctx.publish_kind("error", "error", json!({ "event": name, "error": error }));
				}
			});
		}
		Ok(())
	}

	pub async fn bail(&self, name: &str, data: Value) -> Result<Option<Value>> {
		let prepared = self.prepare(name, &data)?;
		for address in &prepared.listeners {
			match self.send(address, name, data.clone(), &prepared).await {
				Outcome::Answered { data, .. } => return Ok(Some(data)),
				Outcome::Declined { .. } => {}
				other => return Err(other.error().unwrap_or_default()),
			}
		}
		Ok(None)
	}

	pub async fn gather(&self, name: &str, data: Value) -> Result<Vec<Outcome>> {
		let prepared = self.prepare(name, &data)?;
		Ok(futures::future::join_all(
			prepared
				.listeners
				.iter()
				.map(|address| self.send(address, name, data.clone(), &prepared)),
		)
		.await)
	}

	pub async fn ask(&self, cartridge: &str, name: &str, data: Value) -> Result<Outcome> {
		let prepared = self.prepare(name, &data)?;
		let address = prepared
			.listeners
			.iter()
			.find(|address| address.cartridge == cartridge)
			.ok_or_else(|| format!("`{cartridge}` does not listen to `{name}` here"))?;
		Ok(self.send(address, name, data, &prepared).await)
	}

	pub fn notify(&self, name: &str, data: Value) -> Result<()> {
		self.emit(name, data.clone())?;
		self.publish(name, data);
		Ok(())
	}

	pub fn publish(&self, channel: &str, data: Value) {
		self.publish_kind(channel, "data", data);
	}

	fn publish_kind(&self, channel: &str, kind: &str, data: Value) {
		let mut channels = self.state.channels.lock().expect("channels lock");
		let entry = channels.entry(channel.to_owned()).or_default();
		entry.seq += 1;
		let envelope = json!({ "channel": channel, "seq": entry.seq, "kind": kind, "data": data });
		entry.history.push_back(envelope.clone());
		if entry.history.len() > HISTORY {
			entry.history.pop_front();
		}
		entry
			.subscribers
			.retain(|(_, peer)| peer.notify("channel", envelope.clone()).is_ok());
	}

	fn join(&self, channel: &str, since: Option<u64>, peer: Peer) -> (u64, u64) {
		let id = self.state.next.fetch_add(1, Ordering::SeqCst);
		let seq = {
			let mut channels = self.state.channels.lock().expect("channels lock");
			let entry = channels.entry(channel.to_owned()).or_default();
			if let Some(since) = since {
				for envelope in &entry.history {
					if envelope["seq"].as_u64().is_some_and(|seq| seq > since) {
						let _ = peer.notify("channel", envelope.clone());
					}
				}
			}
			entry.subscribers.push((id, peer));
			entry.seq
		};
		self.publish_kind(channel, "subscribe", json!({ "id": id }));
		(id, seq)
	}

	fn leave(&self, channel: &str, id: u64) {
		let removed = {
			let mut channels = self.state.channels.lock().expect("channels lock");
			channels.get_mut(channel).is_some_and(|entry| {
				let before = entry.subscribers.len();
				entry.subscribers.retain(|(sub, _)| *sub != id);
				entry.subscribers.len() != before
			})
		};
		if removed {
			self.publish_kind(channel, "unsubscribe", json!({ "id": id }));
		}
	}

	pub async fn subscribe<F, Fut>(
		&self,
		cartridge: &str,
		channel: &str,
		since: Option<u64>,
		f: F,
	) -> Result<()>
	where
		F: Fn(Value) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = ()> + Send + 'static,
	{
		let address = self.address_of(cartridge)?;
		let (tx, mut rx) = mpsc::channel::<Value>(rpc::QUEUE);
		tokio::spawn(async move {
			while let Some(envelope) = rx.recv().await {
				f(envelope).await;
			}
		});
		let last = Arc::new(AtomicU64::new(since.unwrap_or(0)));
		let key = (address.cartridge.clone(), channel.to_owned());
		// Inserted before the call: `join` sends the replay before its reply, so
		// an envelope arriving without a watcher would be lost, not deferred.
		self.state.watchers.lock().expect("watchers lock").insert(
			key.clone(),
			Watcher {
				tx,
				last: last.clone(),
			},
		);
		let subscribed: Result<Value> = async {
			let peer = self.client(&address).await?;
			peer.call("subscribe", json!({ "channel": channel, "since": since }))
				.await
				.map_err(|error| error.to_string())
		}
		.await;
		match subscribed {
			Ok(reply) => {
				// Only without a replay: `since` already queues envelopes `pump`
				// has not drained, and seeding from the reply would skip them.
				if since.is_none() {
					if let Some(seq) = reply["seq"].as_u64() {
						last.fetch_max(seq, Ordering::SeqCst);
					}
				}
				Ok(())
			}
			Err(error) => {
				self.state
					.watchers
					.lock()
					.expect("watchers lock")
					.remove(&key);
				Err(error)
			}
		}
	}

	pub async fn unsubscribe(&self, cartridge: &str, channel: &str) -> Result<()> {
		let address = self.address_of(cartridge)?;
		self.state
			.watchers
			.lock()
			.expect("watchers lock")
			.remove(&(address.cartridge.clone(), channel.to_owned()));
		let peer = self.client(&address).await?;
		peer.call("unsubscribe", json!({ "channel": channel }))
			.await?;
		Ok(())
	}

	fn address_of(&self, cartridge: &str) -> Result<Address> {
		let directory = self.state.directory.read().expect("directory lock");
		directory
			.events
			.values()
			.flat_map(|entry| entry.listeners.iter())
			.chain(directory.host.iter())
			.find(|address| address.cartridge == cartridge)
			.cloned()
			.ok_or_else(|| format!("`{cartridge}` listens to nothing this cartridge could send"))
	}

	fn client<'a>(&'a self, address: &'a Address) -> BoxFuture<'a, Result<Peer>> {
		self.connect_client(address).boxed()
	}

	async fn connect_client(&self, address: &Address) -> Result<Peer> {
		let slot = self
			.state
			.clients
			.lock()
			.expect("clients lock")
			.entry(address.cartridge.clone())
			.or_default()
			.clone();
		let mut slot = slot.lock().await;
		if let Some(peer) = slot.as_ref().filter(|peer| !peer.is_closed()) {
			return Ok(peer.clone());
		}
		let deadline = tokio::time::Instant::now() + self.state.connect_timeout;
		let mut delay = Duration::from_millis(20);
		let (peer, incoming) = loop {
			match connect(address).await {
				Ok(connected) => break connected,
				Err(Refused::Unauthorized(error)) => {
					return Err(format!("{}: {error}", address.cartridge))
				}
				Err(Refused::Unreachable(error)) => {
					if tokio::time::Instant::now() >= deadline {
						return Err(format!("{} is unreachable: {error}", address.cartridge));
					}
					tokio::time::sleep(delay).await;
					delay = (delay * 2).min(Duration::from_millis(500));
				}
			}
		};
		*slot = Some(peer.clone());
		tokio::spawn(self.clone().pump(address.clone(), peer.clone(), incoming));
		Ok(peer)
	}

	async fn pump(self, address: Address, peer: Peer, mut incoming: mpsc::Receiver<Incoming>) {
		while let Some(message) = incoming.recv().await {
			let Incoming::Notification { method, params } = message else {
				continue;
			};
			if method != "channel" {
				continue;
			}
			let Some(channel) = params["channel"].as_str() else {
				continue;
			};
			let watchers = self.state.watchers.lock().expect("watchers lock");
			if let Some(watcher) = watchers.get(&(address.cartridge.clone(), channel.to_owned())) {
				let seq = params["seq"].as_u64();
				match watcher.tx.try_send(params) {
					Ok(()) => {
						if let Some(seq) = seq {
							watcher.last.fetch_max(seq, Ordering::SeqCst);
						}
					}
					Err(mpsc::error::TrySendError::Full(_)) => peer.close(),
					Err(mpsc::error::TrySendError::Closed(_)) => {}
				}
			}
		}
		drop(peer);
		if self.state.stop.is_cancelled() {
			return;
		}
		let channels: Vec<(String, u64)> = self
			.state
			.watchers
			.lock()
			.expect("watchers lock")
			.iter()
			.filter(|((cartridge, _), _)| *cartridge == address.cartridge)
			.map(|((_, channel), watcher)| (channel.clone(), watcher.last.load(Ordering::SeqCst)))
			.collect();
		if channels.is_empty() {
			return;
		}
		let peer = match self.client(&address).await {
			Ok(peer) => peer,
			Err(error) => {
				let mut watchers = self.state.watchers.lock().expect("watchers lock");
				for (channel, _) in &channels {
					if let Some(watcher) =
						watchers.remove(&(address.cartridge.clone(), channel.clone()))
					{
						let _ = watcher.tx.try_send(
							json!({ "channel": channel, "kind": "error", "data": error }),
						);
					}
				}
				return;
			}
		};
		for (channel, last) in channels {
			let _ = peer
				.call("subscribe", json!({ "channel": channel, "since": last }))
				.await;
		}
	}

	fn access(&self, token: &str) -> Option<Access> {
		if token == self.state.host_token {
			return Some(Access::Host);
		}
		let directory = self.state.directory.read().expect("directory lock");
		directory
			.accept
			.contains_key(token)
			.then(|| Access::Peer(token.to_owned()))
	}

	async fn finish(&self) {
		let finalizers =
			std::mem::take(&mut *self.state.finalizers.lock().expect("finalizers lock"));
		for finalizer in finalizers.into_iter().rev() {
			finalizer().await;
		}
	}
}

enum Refused {
	Unauthorized(rpc::Error),
	Unreachable(String),
}

async fn connect(
	address: &Address,
) -> std::result::Result<(Peer, mpsc::Receiver<Incoming>), Refused> {
	let adapter = crate::transport::typed::connect(&Endpoint::local(&address.socket))
		.await
		.map_err(|e| match e {
			AdapterError::UntrustedEndpoint(_) | AdapterError::Unauthenticated(_) => {
				Refused::Unauthorized(rpc::Error::new(rpc::UNAUTHORIZED, e.to_string()))
			}
			e => Refused::Unreachable(e.to_string()),
		})?;
	let (peer, incoming) = Peer::spawn(adapter, None);
	match peer.call("auth", json!({ "token": address.token })).await {
		Ok(_) => Ok((peer, incoming)),
		Err(error) if error.code == rpc::UNAUTHORIZED => Err(Refused::Unauthorized(error)),
		Err(error) => Err(Refused::Unreachable(error.message)),
	}
}

pub async fn serve(mut listener: LocalListener, ctx: Ctx, apply: Apply) -> Result<()> {
	let apply = Arc::new(Mutex::new(Some(apply)));
	let mut connections = tokio::task::JoinSet::new();
	let mut result = Ok(());
	loop {
		tokio::select! {
			accepted = listener.accept() => match accepted {
				Ok(adapter) => {
					connections.spawn(connection(ctx.clone(), adapter, apply.clone()));
				}
				Err(error) => {
					result = Err(error.to_string());
					break;
				}
			},
			_ = ctx.state.stop.cancelled() => break,
		}
	}
	drop(listener);
	ctx.finish().await;
	for peer in std::mem::take(&mut *ctx.state.connections.lock().expect("connections lock")) {
		peer.close();
	}
	let _ = tokio::time::timeout(Duration::from_secs(2), async {
		while connections.join_next().await.is_some() {}
	})
	.await;
	result
}

async fn connection(ctx: Ctx, adapter: LocalAdapter, apply: Arc<Mutex<Option<Apply>>>) {
	let (peer, mut incoming) = Peer::spawn(adapter, Some(MAX_FRAME));
	{
		let mut connections = ctx.state.connections.lock().expect("connections lock");
		connections.retain(|peer| !peer.is_closed());
		connections.push(peer.clone());
	}
	let access = loop {
		match incoming.recv().await {
			None => return,
			Some(Incoming::Notification { .. }) => continue,
			Some(Incoming::Request(request)) => {
				let access = (request.method == "auth")
					.then(|| {
						request.params["token"]
							.as_str()
							.and_then(|token| ctx.access(token))
					})
					.flatten();
				match access {
					Some(access) => {
						request.reply(Ok(json!({ "cartridge": ctx.name() })));
						break access;
					}
					None => {
						request.reply(Err(rpc::Error::new(
							rpc::UNAUTHORIZED,
							"authenticate first with a valid token",
						)));
						peer.close();
						peer.flushed().await;
						return;
					}
				}
			}
		}
	};
	let mut subscriptions: Vec<(String, u64)> = Vec::new();
	let in_flight = Arc::new(tokio::sync::Semaphore::new(IN_FLIGHT));
	while let Some(message) = incoming.recv().await {
		let Incoming::Request(request) = message else {
			continue;
		};
		match request.method.as_str() {
			"subscribe" => {
				let Some(channel) = request.params["channel"].as_str().map(str::to_owned) else {
					request.reply(Err(rpc::Error::new(
						rpc::INVALID_PARAMS,
						"channel is required",
					)));
					continue;
				};
				if let Some(at) = subscriptions.iter().position(|(c, _)| *c == channel) {
					let (_, id) = subscriptions.remove(at);
					ctx.leave(&channel, id);
				}
				let since = request.params["since"].as_u64();
				let (id, seq) = ctx.join(&channel, since, peer.clone());
				subscriptions.push((channel, id));
				request.reply(Ok(json!({ "seq": seq })));
			}
			"unsubscribe" => {
				let channel = request.params["channel"].as_str().unwrap_or_default();
				if let Some(at) = subscriptions.iter().position(|(c, _)| c == channel) {
					let (channel, id) = subscriptions.remove(at);
					ctx.leave(&channel, id);
				}
				request.reply(Ok(json!({})));
			}
			_ => {
				let Ok(permit) = in_flight.clone().acquire_owned().await else {
					break;
				};
				let (ctx, access, apply) = (ctx.clone(), access.clone(), apply.clone());
				tokio::spawn(async move {
					handle(ctx, access, request, apply).await;
					drop(permit);
				});
			}
		}
	}
	for (channel, id) in subscriptions {
		ctx.leave(&channel, id);
	}
	peer.flushed().await;
}

fn unauthorized(request: Request) {
	let message = format!("`{}` is not granted to this token", request.method);
	request.reply(Err(rpc::Error::new(rpc::UNAUTHORIZED, message)));
}

async fn handle(ctx: Ctx, access: Access, request: Request, apply: Arc<Mutex<Option<Apply>>>) {
	let params = request.params.clone();
	match request.method.as_str() {
		"apply" if access.is_host() => {
			let Some(apply) = apply.lock().expect("apply lock").take() else {
				request.reply(Err(rpc::Error::application("already applied")));
				return;
			};
			*ctx.state.name.write().expect("name lock") =
				params["name"].as_str().unwrap_or_default().to_owned();
			match serde_json::from_value::<Directory>(params["directory"].clone()) {
				Ok(directory) => ctx.set_directory(directory),
				Err(error) => {
					request.reply(Err(rpc::Error::new(
						rpc::INVALID_PARAMS,
						format!("directory: {error}"),
					)));
					return;
				}
			}
			match apply(ctx.clone(), params["config"].clone()).await {
				Ok(()) => request.reply(Ok(json!({}))),
				Err(error) => {
					request.reply(Err(rpc::Error::application(error)));
					ctx.stop();
				}
			}
		}
		"directory" if access.is_host() => {
			match serde_json::from_value::<Directory>(params["directory"].clone()) {
				Ok(directory) => {
					ctx.set_directory(directory);
					request.reply(Ok(json!({})));
				}
				Err(error) => request.reply(Err(rpc::Error::new(
					rpc::INVALID_PARAMS,
					format!("directory: {error}"),
				))),
			}
		}
		"dispose" if access.is_host() => {
			request.reply(Ok(json!({})));
			ctx.stop();
		}
		"event" => {
			let name = params["name"].as_str().unwrap_or_default().to_owned();
			if let Access::Peer(token) = &access {
				if !ctx.accepts(token, &name) {
					let message = format!("`{name}` may not be sent here with this token");
					return request.reply(Err(rpc::Error::new(rpc::UNAUTHORIZED, message)));
				}
			}
			let listener = ctx
				.state
				.events
				.read()
				.expect("events lock")
				.get(&name)
				.cloned();
			let Some(listener) = listener else {
				request.reply(Ok(Value::Null));
				return;
			};
			if let Err(error) = ctx.validate(&name, &params["data"]) {
				return request.reply(Err(rpc::Error::new(rpc::INVALID_PARAMS, error)));
			}
			// Off the workers: a listener takes its node's Lua lock synchronously,
			// and a lock held by a handler waiting on another cartridge would park
			// the worker whose queue carries that very reply.
			let (trace, data) = (trace_of(&params), params["data"].clone());
			let runtime = tokio::runtime::Handle::current();
			let outcome = tokio::task::spawn_blocking(move || {
				runtime.block_on(TRACE.scope(trace, listener(data)))
			})
			.await
			.unwrap_or_else(|error| Err(format!("listener panicked: {error}")));
			match outcome {
				Ok(answer) => request.reply(Ok(answer)),
				Err(error) => {
					ctx.publish_kind("error", "error", json!({ "event": name, "error": error }));
					request.reply(Err(rpc::Error::application(error)));
				}
			}
		}
		"apply" | "directory" | "dispose" => unauthorized(request),
		other => {
			let message = format!("unknown method `{other}`");
			request.reply(Err(rpc::Error::new(rpc::METHOD_NOT_FOUND, message)));
		}
	}
}

pub async fn listen(path: impl Into<PathBuf>) -> Result<LocalListener> {
	let path = path.into();
	match crate::transport::typed::bind(&Endpoint::local(&path)).await {
		Ok(BindOutcome::Bound(listener)) => Ok(listener),
		Ok(BindOutcome::AlreadyRunning) => Err(format!("{} is already served", path.display())),
		Err(error) => Err(format!("{}: {error}", path.display())),
	}
}

#[cfg(test)]
#[path = "tests/cartridge.rs"]
mod tests;
