//! The cartridge side of the cartridge protocol (docs/transport.txt).

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

use crate::rpc::{self, Incoming, Peer, Request};
use crate::typed::{BindOutcome, Endpoint, LocalAdapter, LocalListener};

pub const SOCKET_ENV: &str = "CARTRIDGE_SOCKET";
pub const HOST_TOKEN_ENV: &str = "CARTRIDGE_HOST_TOKEN";
pub const CONNECT_TIMEOUT_ENV: &str = "CARTRIDGE_CONNECT_TIMEOUT_SECS";

const MAX_FRAME: usize = 64 * 1024 * 1024;
const HISTORY: usize = 1024;
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Directory {
	#[serde(default)]
	pub needs: BTreeMap<String, Address>,
	#[serde(default)]
	pub events: BTreeMap<String, Vec<Address>>,
	#[serde(default)]
	pub accept: BTreeMap<String, Grant>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
	pub cartridge: String,
	pub socket: PathBuf,
	pub token: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
	pub from: String,
	pub scope: Scope,
	#[serde(default)]
	pub names: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
	Call,
	Event,
}

#[derive(Clone)]
enum Access {
	Host,
	Granted(Grant),
}

impl Access {
	fn is_host(&self) -> bool {
		matches!(self, Access::Host)
	}

	fn allows(&self, scope: Scope, name: &str) -> bool {
		match self {
			Access::Host => true,
			Access::Granted(grant) => grant.scope == scope && grant.names.iter().any(|n| n == name),
		}
	}

	fn may_subscribe(&self) -> bool {
		match self {
			Access::Host => true,
			Access::Granted(grant) => grant.scope == Scope::Call,
		}
	}
}

tokio::task_local! {
	static TRACE: String;
}

/// The trace of the request this code is handling.
pub fn trace() -> Option<String> {
	TRACE.try_with(Clone::clone).ok()
}

fn trace_of(params: &Value) -> String {
	params["trace"]
		.as_str()
		.map(str::to_owned)
		.or_else(trace)
		.unwrap_or_else(|| crate::token()[..16].to_owned())
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

/// A cartridge's view of itself and of the composition.
#[derive(Clone)]
pub struct Ctx {
	state: Arc<State>,
}

struct State {
	name: RwLock<String>,
	host_token: String,
	connect_timeout: Duration,
	directory: RwLock<Directory>,
	services: RwLock<HashMap<String, Handler>>,
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
	tx: mpsc::UnboundedSender<Value>,
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
				services: RwLock::default(),
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

	/// The keys this cartridge may call.
	pub fn needs(&self) -> Vec<String> {
		self.state
			.directory
			.read()
			.expect("directory lock")
			.needs
			.keys()
			.cloned()
			.collect()
	}

	pub fn provide<F, Fut>(&self, key: &str, f: F)
	where
		F: Fn(Value) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<Value>> + Send + 'static,
	{
		self.state
			.services
			.write()
			.expect("services lock")
			.insert(key.to_owned(), handler(f));
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

	/// Runs when the cartridge stops, last registered first.
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

	/// Stop serving; [`serve`] returns once the finalizers have run.
	pub fn stop(&self) {
		self.state.stop.cancel();
	}

	pub async fn call(&self, key: &str, args: Value) -> Result<Value> {
		let address = self
			.state
			.directory
			.read()
			.expect("directory lock")
			.needs
			.get(key)
			.cloned()
			.ok_or_else(|| format!("`{key}` is not a need of this cartridge"))?;
		let peer = self.client(&address).await?;
		Ok(peer
			.call(
				"call",
				json!({ "key": key, "args": args, "trace": trace_of(&Value::Null) }),
			)
			.await?)
	}

	fn listeners(&self, name: &str) -> Vec<Address> {
		self.state
			.directory
			.read()
			.expect("directory lock")
			.events
			.get(name)
			.cloned()
			.unwrap_or_default()
	}

	async fn send_event(
		&self,
		address: &Address,
		name: &str,
		data: Value,
		trace: String,
	) -> Result<Value> {
		let peer = self.client(address).await?;
		Ok(peer
			.call(
				"event",
				json!({ "name": name, "data": data, "trace": trace }),
			)
			.await?)
	}

	/// Send to every listener without waiting.
	pub fn emit(&self, name: &str, data: Value) {
		let trace = trace_of(&Value::Null);
		for address in self.listeners(name) {
			let (ctx, name, data, trace) =
				(self.clone(), name.to_owned(), data.clone(), trace.clone());
			tokio::spawn(async move {
				let _ = ctx.send_event(&address, &name, data, trace).await;
			});
		}
	}

	/// Ask listeners in order; the first non-null answer wins.
	pub async fn bail(&self, name: &str, data: Value) -> Result<Option<Value>> {
		let trace = trace_of(&Value::Null);
		for address in self.listeners(name) {
			let answer = self
				.send_event(&address, name, data.clone(), trace.clone())
				.await?;
			if !answer.is_null() {
				return Ok(Some(answer));
			}
		}
		Ok(None)
	}

	/// Ask every listener and wait; fails if any failed.
	pub async fn parallel(&self, name: &str, data: Value) -> Result<()> {
		let trace = trace_of(&Value::Null);
		let listeners = self.listeners(name);
		let answers = futures::future::join_all(
			listeners
				.iter()
				.map(|address| self.send_event(address, name, data.clone(), trace.clone())),
		)
		.await;
		let errors: Vec<String> = answers.into_iter().filter_map(|a| a.err()).collect();
		if errors.is_empty() {
			Ok(())
		} else {
			Err(errors.join("; "))
		}
	}

	/// Every non-null answer, with the cartridge that gave it.
	pub async fn gather(&self, name: &str, data: Value) -> Vec<(String, Value)> {
		let trace = trace_of(&Value::Null);
		let listeners = self.listeners(name);
		let answers = futures::future::join_all(
			listeners
				.iter()
				.map(|address| self.send_event(address, name, data.clone(), trace.clone())),
		)
		.await;
		listeners
			.into_iter()
			.zip(answers)
			.filter_map(|(address, answer)| match answer {
				Ok(value) if !value.is_null() => Some((address.cartridge, value)),
				_ => None,
			})
			.collect()
	}

	/// Emit to listeners and publish on the channel of the same name.
	pub fn notify(&self, name: &str, data: Value) {
		self.emit(name, data.clone());
		self.publish(name, data);
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

	/// Watch `channel` of the needed cartridge `cartridge`. Envelopes reach `f`
	/// in order; the subscription survives the publisher restarting.
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
		let (tx, mut rx) = mpsc::unbounded_channel::<Value>();
		tokio::spawn(async move {
			while let Some(envelope) = rx.recv().await {
				f(envelope).await;
			}
		});
		let last = Arc::new(AtomicU64::new(since.unwrap_or(0)));
		self.state.watchers.lock().expect("watchers lock").insert(
			(address.token.clone(), channel.to_owned()),
			Watcher { tx, last },
		);
		let peer = self.client(&address).await?;
		peer.call("subscribe", json!({ "channel": channel, "since": since }))
			.await?;
		Ok(())
	}

	pub async fn unsubscribe(&self, cartridge: &str, channel: &str) -> Result<()> {
		let address = self.address_of(cartridge)?;
		self.state
			.watchers
			.lock()
			.expect("watchers lock")
			.remove(&(address.token.clone(), channel.to_owned()));
		let peer = self.client(&address).await?;
		peer.call("unsubscribe", json!({ "channel": channel }))
			.await?;
		Ok(())
	}

	fn address_of(&self, cartridge: &str) -> Result<Address> {
		self.state
			.directory
			.read()
			.expect("directory lock")
			.needs
			.values()
			.find(|address| address.cartridge == cartridge)
			.cloned()
			.ok_or_else(|| format!("`{cartridge}` is not needed by this cartridge"))
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
			.entry(address.token.clone())
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
		tokio::spawn(self.clone().pump(address.clone(), incoming));
		Ok(peer)
	}

	async fn pump(self, address: Address, mut incoming: mpsc::UnboundedReceiver<Incoming>) {
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
			if let Some(watcher) = watchers.get(&(address.token.clone(), channel.to_owned())) {
				if let Some(seq) = params["seq"].as_u64() {
					watcher.last.fetch_max(seq, Ordering::SeqCst);
				}
				let _ = watcher.tx.send(params);
			}
		}
		if self.state.stop.is_cancelled() {
			return;
		}
		let channels: Vec<(String, u64)> = self
			.state
			.watchers
			.lock()
			.expect("watchers lock")
			.iter()
			.filter(|((token, _), _)| *token == address.token)
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
						watchers.remove(&(address.token.clone(), channel.clone()))
					{
						let _ = watcher
							.tx
							.send(json!({ "channel": channel, "kind": "error", "data": error }));
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
		self.state
			.directory
			.read()
			.expect("directory lock")
			.accept
			.get(token)
			.cloned()
			.map(Access::Granted)
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
) -> std::result::Result<(Peer, mpsc::UnboundedReceiver<Incoming>), Refused> {
	let adapter = crate::typed::connect(&Endpoint::Unix(address.socket.clone()))
		.await
		.map_err(|e| Refused::Unreachable(e.to_string()))?;
	let (peer, incoming) = Peer::spawn(adapter, None);
	match peer.call("auth", json!({ "token": address.token })).await {
		Ok(_) => Ok((peer, incoming)),
		Err(error) if error.code == rpc::UNAUTHORIZED => Err(Refused::Unauthorized(error)),
		Err(error) => Err(Refused::Unreachable(error.message)),
	}
}

/// Serve `ctx` on `listener` until it is disposed or stopped. `apply` runs when
/// the host calls `apply`; its error is the apply's error.
pub async fn serve(mut listener: LocalListener, ctx: Ctx, apply: Apply) {
	let apply = Arc::new(Mutex::new(Some(apply)));
	let mut connections = tokio::task::JoinSet::new();
	loop {
		tokio::select! {
			accepted = listener.accept() => match accepted {
				Ok(adapter) => {
					connections.spawn(connection(ctx.clone(), adapter, apply.clone()));
				}
				Err(_) => break,
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
}

async fn connection(ctx: Ctx, adapter: LocalAdapter, apply: Arc<Mutex<Option<Apply>>>) {
	let (peer, mut incoming) = Peer::spawn(adapter, Some(MAX_FRAME));
	ctx.state
		.connections
		.lock()
		.expect("connections lock")
		.push(peer.clone());
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
	while let Some(message) = incoming.recv().await {
		let Incoming::Request(request) = message else {
			continue;
		};
		match request.method.as_str() {
			"subscribe" if access.may_subscribe() => {
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
				tokio::spawn(handle(ctx.clone(), access.clone(), request, apply.clone()));
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
				Ok(directory) => *ctx.state.directory.write().expect("directory lock") = directory,
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
					*ctx.state.directory.write().expect("directory lock") = directory;
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
		"call" => {
			let key = params["key"].as_str().unwrap_or_default().to_owned();
			if !access.allows(Scope::Call, &key) {
				return unauthorized(request);
			}
			let service = ctx
				.state
				.services
				.read()
				.expect("services lock")
				.get(&key)
				.cloned();
			let Some(service) = service else {
				request.reply(Err(rpc::Error::new(
					rpc::NOT_PROVIDED,
					format!("`{key}` is not provided"),
				)));
				return;
			};
			let result = TRACE
				.scope(trace_of(&params), service(params["args"].clone()))
				.await;
			request.reply(result.map_err(rpc::Error::application));
		}
		"event" => {
			let name = params["name"].as_str().unwrap_or_default().to_owned();
			if !access.allows(Scope::Event, &name) {
				return unauthorized(request);
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
			match TRACE
				.scope(trace_of(&params), listener(params["data"].clone()))
				.await
			{
				Ok(answer) => request.reply(Ok(answer)),
				Err(error) => {
					ctx.publish_kind("error", "error", json!({ "event": name, "error": error }));
					request.reply(Err(rpc::Error::application(error)));
				}
			}
		}
		"apply" | "directory" | "dispose" | "subscribe" => unauthorized(request),
		other => {
			let message = format!("unknown method `{other}`");
			request.reply(Err(rpc::Error::new(rpc::METHOD_NOT_FOUND, message)));
		}
	}
}

/// Bind `path` for serving.
pub async fn listen(path: impl Into<PathBuf>) -> Result<LocalListener> {
	let path = path.into();
	match crate::typed::bind(&Endpoint::Unix(path.clone())).await {
		Ok(BindOutcome::Bound(listener)) => Ok(listener),
		Ok(BindOutcome::AlreadyRunning) => Err(format!("{} is already served", path.display())),
		Err(error) => Err(format!("{}: {error}", path.display())),
	}
}

/// Run a cartridge process: bind `CARTRIDGE_SOCKET`, serve until disposed or
/// until stdin closes, then exit.
pub async fn run<F, Fut>(apply: F)
where
	F: FnOnce(Ctx, Value) -> Fut + Send + 'static,
	Fut: Future<Output = Result<()>> + Send + 'static,
{
	let (Ok(socket), Ok(host_token)) = (std::env::var(SOCKET_ENV), std::env::var(HOST_TOKEN_ENV))
	else {
		eprintln!("{SOCKET_ENV} and {HOST_TOKEN_ENV} must be set; a cartridge is started by the cartridge host");
		std::process::exit(2);
	};
	let timeout = std::env::var(CONNECT_TIMEOUT_ENV)
		.ok()
		.and_then(|secs| secs.parse().ok())
		.map(Duration::from_secs)
		.unwrap_or(DEFAULT_CONNECT_TIMEOUT);
	let listener = match listen(&socket).await {
		Ok(listener) => listener,
		Err(error) => {
			eprintln!("{error}");
			std::process::exit(1);
		}
	};
	let ctx = Ctx::new(host_token, timeout);
	let lifeline = ctx.clone();
	std::thread::spawn(move || {
		let _ = std::io::copy(&mut std::io::stdin(), &mut std::io::sink());
		lifeline.stop();
	});
	serve(
		listener,
		ctx,
		Box::new(move |ctx, config| apply(ctx, config).boxed()),
	)
	.await;
	std::process::exit(0);
}

#[cfg(test)]
#[path = "tests/cartridge.rs"]
mod tests;
