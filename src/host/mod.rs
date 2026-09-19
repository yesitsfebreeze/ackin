mod plan;
mod process;
mod run;
pub mod socket;
mod watch;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::transport::cartridge::{Ctx, Directory, Outcome};
use crate::transport::rpc::{Incoming, Peer};
use futures::future::BoxFuture;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::{broadcast, mpsc};

use crate::error::{Error, Result};
use crate::loader::{Entry, Source};

pub use plan::Plan;
pub use process::{LISTENER_FD_ENV, NODE_BIN_ENV};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
	Disabled,
	Waiting,
	Starting,
	Active,
	Stopping,
	Failed,
}

#[derive(Clone, Debug, Serialize)]
pub struct Status {
	pub id: String,
	pub state: State,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub waiting: Vec<String>,
	pub events: Vec<String>,
	pub needs: Vec<String>,
	pub listen: Vec<String>,
	pub socket: PathBuf,
}

pub(crate) struct Running {
	peer: Peer,
	stop: Box<dyn FnOnce(Peer) -> BoxFuture<'static, ()> + Send>,
}

impl Running {
	async fn stop(self) {
		(self.stop)(self.peer).await;
	}
}

struct Slot {
	entry: Entry,
	plan: Option<Arc<Plan>>,
	state: State,
	error: Option<String>,
	sources: Vec<Source>,
	directory: Directory,
	generation: u64,
	running: Option<Running>,
}

impl Slot {
	fn new(entry: Entry, dir: &Path) -> Self {
		Self {
			sources: vec![Source::new(entry.file(dir))],
			entry,
			plan: None,
			state: State::Disabled,
			error: None,
			directory: Directory::default(),
			generation: 0,
			running: None,
		}
	}
}

fn unix_time() -> u64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_default()
		.as_secs()
}

pub struct Host {
	pub(crate) dir: PathBuf,
	pub(crate) descriptor: PathBuf,
	pub(crate) solo: Mutex<Option<Vec<Entry>>>,
	sockets: PathBuf,
	nodes: PathBuf,
	host_token: String,
	tokens: Mutex<HashMap<(String, Option<String>), String>>,
	node_tokens: Mutex<HashMap<String, String>>,
	ctx: Ctx,
	slots: Mutex<Vec<Slot>>,
	listeners: Mutex<HashMap<String, Vec<String>>>,
	op: tokio::sync::Mutex<()>,
	lifecycle: broadcast::Sender<Value>,
	inner: std::sync::OnceLock<tokio::task::JoinHandle<()>>,
	stop: tokio_util::sync::CancellationToken,
	/// Set while a takeover holds its listener back until its composition is up.
	deferred: std::sync::atomic::AtomicBool,
	/// A private host (one command's own `run` or `verify`) serves this path in
	/// its own run directory instead of the project's name, so it cannot take
	/// the name from the daemon, and its nodes' host calls still reach it.
	private: std::sync::atomic::AtomicBool,
	/// The socket file this host published, so a stop unlinks its own and
	/// never a successor's.
	published: Mutex<Option<(u64, u64)>>,
	/// Clients attached right now, and when the last one left. Nodes are not
	/// clients: they are this host's own children and are connected for as
	/// long as it serves, so counting them would mean no host is ever idle.
	clients: std::sync::atomic::AtomicI64,
	idle_since: std::sync::atomic::AtomicU64,
	/// Listeners bound on behalf of nodes (a manifest's `listener`), kept
	/// across a node's restarts and handed down as an inherited fd.
	#[cfg(unix)]
	tcp: Mutex<HashMap<String, std::net::TcpListener>>,
}

impl Drop for Host {
	fn drop(&mut self) {
		if let Some(task) = self.inner.get() {
			task.abort();
		}
		self.unpublish();
	}
}

impl Host {
	pub fn new(dir: impl Into<PathBuf>, descriptor: impl Into<PathBuf>) -> Result<Arc<Self>> {
		let dir: PathBuf = dir.into();
		let descriptor: PathBuf = descriptor.into();
		let descriptor = descriptor.canonicalize().unwrap_or(descriptor);
		let host_token = crate::transport::token();
		Ok(Arc::new(Self {
			sockets: socket::run_dir(&descriptor)?,
			nodes: socket::host_dir(&descriptor)?,
			dir: dir.canonicalize().unwrap_or(dir),
			descriptor,
			solo: Mutex::new(None),
			ctx: Ctx::new(
				host_token.clone(),
				crate::settings::host().startup_timeout(),
			),
			host_token,
			tokens: Mutex::default(),
			node_tokens: Mutex::default(),
			slots: Mutex::default(),
			listeners: Mutex::default(),
			op: tokio::sync::Mutex::new(()),
			lifecycle: broadcast::channel(crate::settings::host().lifecycle_queue).0,
			inner: std::sync::OnceLock::new(),
			stop: tokio_util::sync::CancellationToken::new(),
			deferred: std::sync::atomic::AtomicBool::new(false),
			private: std::sync::atomic::AtomicBool::new(false),
			published: Mutex::new(None),
			clients: std::sync::atomic::AtomicI64::new(0),
			idle_since: std::sync::atomic::AtomicU64::new(unix_time()),
			#[cfg(unix)]
			tcp: Mutex::default(),
		}))
	}

	/// One command's own host: it never binds the project's socket.
	pub fn private(self: &Arc<Self>) -> Arc<Self> {
		self.private
			.store(true, std::sync::atomic::Ordering::SeqCst);
		self.clone()
	}

	pub fn dir(&self) -> &Path {
		&self.dir
	}

	pub fn descriptor(&self) -> &Path {
		&self.descriptor
	}

	pub(crate) fn host_token(&self) -> &str {
		&self.host_token
	}

	pub(crate) fn node_token(&self, id: &str) -> String {
		self.node_tokens.lock().get(id).cloned().unwrap_or_default()
	}

	pub(crate) fn client_joined(&self) {
		self.clients
			.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
	}

	pub(crate) fn client_left(&self) {
		self.clients
			.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
		self.idle_since
			.store(unix_time(), std::sync::atomic::Ordering::SeqCst);
	}

	/// How long this host has served no client, and zero while one is attached.
	pub fn idle_for(&self) -> std::time::Duration {
		if self.clients.load(std::sync::atomic::Ordering::SeqCst) > 0 {
			return std::time::Duration::ZERO;
		}
		let since = self.idle_since.load(std::sync::atomic::Ordering::SeqCst);
		std::time::Duration::from_secs(unix_time().saturating_sub(since))
	}

	pub fn stop_signal(&self) -> tokio_util::sync::CancellationToken {
		self.stop.clone()
	}

	pub async fn stopped(&self) {
		self.stop.cancelled().await;
	}

	pub fn socket_path(&self) -> PathBuf {
		if self.private.load(std::sync::atomic::Ordering::SeqCst) {
			return self.nodes.join("host.sock");
		}
		self.sockets.join("host.sock")
	}

	pub fn socket(&self, id: &str) -> PathBuf {
		self.nodes.join(socket::file_name(id))
	}

	pub fn lifecycle(&self) -> broadcast::Receiver<Value> {
		self.lifecycle.subscribe()
	}

	fn publish(&self, slot: &Slot) {
		let _ = self.lifecycle.send(json!({
			"id": slot.entry.id,
			"state": slot.state,
			"error": slot.error,
		}));
	}

	pub fn status(&self) -> Vec<Status> {
		let slots = self.slots.lock();
		let listeners = self.listeners.lock();
		let active: HashSet<&str> = slots
			.iter()
			.filter(|s| s.state == State::Active)
			.map(|s| s.entry.id.as_str())
			.collect();
		slots
			.iter()
			.map(|slot| {
				let plan = slot.plan.as_deref();
				let needs = plan.map(|p| p.needs.clone()).unwrap_or_default();
				let waiting = match slot.state {
					State::Waiting => needs
						.iter()
						.filter(|key| {
							plan.is_some_and(|p| p.waits_for(key))
								&& !listeners.get(*key).is_some_and(|ids| {
									ids.iter().any(|id| active.contains(id.as_str()))
								})
						})
						.cloned()
						.collect(),
					_ => Vec::new(),
				};
				Status {
					id: slot.entry.id.clone(),
					state: slot.state,
					error: slot.error.clone(),
					waiting,
					events: plan
						.map(|p| p.events.keys().cloned().collect())
						.unwrap_or_default(),
					needs,
					listen: plan.map(|p| p.listen.clone()).unwrap_or_default(),
					socket: self.socket(&slot.entry.id),
				}
			})
			.collect()
	}

	/// Bind the project's socket before composing, so a host that loses the
	/// name to another fails here instead of running nodeless.
	pub async fn listen(self: &Arc<Self>) -> Result<()> {
		let _op = self.op.lock().await;
		if self.inner.get().is_none() {
			let listener = socket::listen(&self.socket_path()).await?;
			self.publish_listener(listener);
		}
		Ok(())
	}

	pub async fn reconcile(self: &Arc<Self>) -> Result<()> {
		let _op = self.op.lock().await;
		if self.inner.get().is_none() && !self.deferred.load(std::sync::atomic::Ordering::SeqCst) {
			let listener = socket::listen(&self.socket_path()).await?;
			self.publish_listener(listener);
		}
		let entries = self.entries()?;
		let planned: Vec<(Entry, Option<Result<Plan>>)> = entries
			.into_iter()
			.map(|entry| {
				let plan = (!entry.disabled).then(|| self.plan(&entry));
				(entry, plan)
			})
			.collect();
		let stale: Vec<String> = {
			let slots = self.slots.lock();
			slots
				.iter()
				.filter(|slot| slot.running.is_some())
				.filter(|slot| {
					!planned
						.iter()
						.any(|(entry, plan)| *entry == slot.entry && matches!(plan, Some(Ok(_))))
				})
				.map(|slot| slot.entry.id.clone())
				.collect()
		};
		for id in stale {
			self.stop_slot(&id).await;
			#[cfg(unix)]
			self.tcp.lock().remove(&id);
		}
		{
			let mut slots = self.slots.lock();
			let mut old = std::mem::take(&mut *slots);
			for (entry, plan) in planned {
				let previous = old
					.iter()
					.position(|slot| slot.entry.id == entry.id)
					.map(|at| old.remove(at));
				let slot = match (previous, plan) {
					(Some(mut slot), Some(Ok(plan))) if slot.running.is_some() => {
						slot.plan = Some(Arc::new(plan));
						slot
					}
					(_, None) => Slot::new(entry, &self.dir),
					(_, Some(Err(error))) => {
						let mut slot = Slot::new(entry, &self.dir);
						slot.state = State::Failed;
						slot.error = Some(error.to_string());
						tracing::error!(target: "cartridge", cartridge = %slot.entry.id, "{error}");
						slot
					}
					(_, Some(Ok(plan))) => {
						let mut slot = Slot::new(entry, &self.dir);
						slot.sources = plan.sources.iter().cloned().map(Source::new).collect();
						slot.plan = Some(Arc::new(plan));
						slot.state = State::Waiting;
						slot
					}
				};
				self.publish(&slot);
				slots.push(slot);
			}
		}
		self.rewire().await;
		self.start_ready().await;
		Ok(())
	}

	async fn rewire(self: &Arc<Self>) {
		let updates: Vec<(Peer, Directory)> = {
			let mut slots = self.slots.lock();
			let all: Vec<Arc<Plan>> = slots
				.iter()
				.filter(|s| s.state != State::Failed || s.running.is_some())
				.filter_map(|s| s.plan.clone())
				.collect();
			let (catalogue, clashes) = plan::catalogue(&all);
			for slot in slots.iter_mut().filter(|s| s.running.is_none()) {
				let Some(plan) = slot.plan.clone() else {
					continue;
				};
				let refusal = clashes
					.get(&slot.entry.id)
					.cloned()
					.or_else(|| plan::unmatched(&plan, &catalogue));
				if let Some(why) = refusal {
					if slot.state != State::Failed {
						tracing::error!(target: "cartridge", cartridge = %slot.entry.id, "{why}");
					}
					slot.state = State::Failed;
					slot.error = Some(why);
				}
			}
			let plans: Vec<Arc<Plan>> = slots
				.iter()
				.filter(|s| s.state != State::Failed || s.running.is_some())
				.filter_map(|s| s.plan.clone())
				.collect();
			let (catalogue, _) = plan::catalogue(&plans);
			let mut updates = Vec::new();
			for slot in slots.iter_mut() {
				let Some(plan) = slot.plan.clone() else {
					continue;
				};
				let directory = plan::directory(self, &plan, &plans, &catalogue);
				if directory != slot.directory {
					slot.directory = directory.clone();
					if let Some(running) = &slot.running {
						updates.push((running.peer.clone(), directory));
					}
				}
			}
			*self.listeners.lock() = plan::listeners(&plans);
			updates
		};
		for (peer, directory) in updates {
			let _ = tokio::time::timeout(
				Duration::from_millis(500),
				peer.call("directory", json!({ "directory": directory })),
			)
			.await;
		}
	}

	async fn start_ready(self: &Arc<Self>) {
		loop {
			let ready: Vec<(String, Arc<Plan>, Directory, u64)> = {
				let mut slots = self.slots.lock();
				let listeners = self.listeners.lock().clone();
				let active: HashSet<String> = slots
					.iter()
					.filter(|s| s.state == State::Active)
					.map(|s| s.entry.id.clone())
					.collect();
				let mut ready = Vec::new();
				for slot in slots.iter_mut().filter(|s| s.state == State::Waiting) {
					let Some(plan) = slot.plan.clone() else {
						continue;
					};
					let served = plan.needs.iter().all(|key| {
						!plan.waits_for(key)
							|| listeners
								.get(key)
								.is_some_and(|ids| ids.iter().any(|id| active.contains(id)))
					});
					if served {
						slot.state = State::Starting;
						slot.generation += 1;
						slot.error = None;
						ready.push((
							slot.entry.id.clone(),
							plan,
							slot.directory.clone(),
							slot.generation,
						));
					}
				}
				for slot in slots.iter().filter(|s| s.state == State::Starting) {
					self.publish(slot);
				}
				ready
			};
			if ready.is_empty() {
				return;
			}
			let started = futures::future::join_all(ready.into_iter().map(
				|(id, plan, directory, generation)| {
					let host = self.clone();
					async move {
						let result = host.launch(&plan, &directory, generation).await;
						(id, generation, result)
					}
				},
			))
			.await;
			let mut slots = self.slots.lock();
			for (id, generation, result) in started {
				let Some(slot) = slots
					.iter_mut()
					.find(|s| s.entry.id == id && s.generation == generation)
				else {
					continue;
				};
				match result {
					Ok(running) => {
						slot.running = Some(running);
						slot.state = State::Active;
					}
					Err(error) => {
						tracing::error!(target: "cartridge", cartridge = %id, "{error}");
						slot.state = State::Failed;
						slot.error = Some(error.to_string());
					}
				}
				self.publish(slot);
			}
		}
	}

	async fn launch(
		self: &Arc<Self>,
		plan: &Plan,
		directory: &Directory,
		generation: u64,
	) -> Result<Running> {
		tracing::info!(target: "cartridge", cartridge = %plan.id, "starting");
		process::start(self, plan, directory, generation).await
	}

	pub(crate) fn exited(&self, id: &str, generation: u64, why: String) {
		let mut slots = self.slots.lock();
		if let Some(slot) = slots
			.iter_mut()
			.find(|s| s.entry.id == id && s.generation == generation && s.state == State::Active)
		{
			tracing::error!(target: "cartridge", cartridge = %id, "{why}");
			slot.running = None;
			slot.state = State::Failed;
			slot.error = Some(why);
			self.publish(slot);
		}
	}

	async fn stop_slot(&self, id: &str) {
		let running = {
			let mut slots = self.slots.lock();
			let Some(slot) = slots.iter_mut().find(|s| s.entry.id == id) else {
				return;
			};
			slot.generation += 1;
			let running = slot.running.take();
			if running.is_some() {
				slot.state = State::Stopping;
				self.publish(slot);
			}
			running
		};
		if let Some(running) = running {
			running.stop().await;
		}
	}

	pub async fn replace(self: &Arc<Self>, id: &str) -> Result<()> {
		let _op = self.op.lock().await;
		self.replace_locked(id).await
	}

	async fn replace_locked(self: &Arc<Self>, id: &str) -> Result<()> {
		let entry = self
			.slots
			.lock()
			.iter()
			.find(|s| s.entry.id == id && !s.entry.disabled)
			.map(|s| s.entry.clone())
			.ok_or_else(|| Error::Reload(format!("`{id}` is not an enabled entry")))?;
		let plan = self.plan(&entry);
		let was_active = {
			let slots = self.slots.lock();
			slots
				.iter()
				.find(|s| s.entry.id == id)
				.is_some_and(|s| s.state == State::Active)
		};
		self.stop_slot(id).await;
		let rewire = {
			let mut slots = self.slots.lock();
			let slot = slots
				.iter_mut()
				.find(|s| s.entry.id == id)
				.expect("slot of a known entry");
			match &plan {
				Ok(plan) => {
					let rewire = !was_active
						|| slot
							.plan
							.as_ref()
							.is_none_or(|old| old.wiring() != plan.wiring());
					slot.sources = plan.sources.iter().cloned().map(Source::new).collect();
					slot.plan = Some(Arc::new(plan.clone()));
					slot.state = State::Waiting;
					slot.error = None;
					rewire
				}
				Err(error) => {
					slot.state = State::Failed;
					slot.error = Some(error.to_string());
					false
				}
			}
		};
		if rewire {
			self.rewire().await;
		}
		self.start_ready().await;
		plan.map(|_| ())
	}

	pub(crate) async fn replace_changed(self: &Arc<Self>, paths: &[PathBuf]) {
		let _op = self.op.lock().await;
		let ids: Vec<String> = self
			.slots
			.lock()
			.iter()
			.filter(|s| {
				!s.entry.disabled
					&& s.sources
						.iter()
						.any(|source| paths.contains(&source.path) && source.changed())
			})
			.map(|s| s.entry.id.clone())
			.collect();
		for id in ids {
			if let Err(error) = self.replace_locked(&id).await {
				tracing::error!(target: "cartridge", cartridge = %id, "{error}");
			}
		}
	}

	pub async fn stop(&self) {
		let _op = self.op.lock().await;
		let ids: Vec<String> = self
			.slots
			.lock()
			.iter()
			.filter(|s| s.running.is_some())
			.map(|s| s.entry.id.clone())
			.collect();
		futures::future::join_all(ids.iter().map(|id| self.stop_slot(id))).await;
		for slot in self.slots.lock().iter_mut() {
			if slot.state == State::Stopping {
				slot.state = State::Waiting;
			}
		}
		// A task may still hold this base past the runtime's end, so the files
		// go now, not on drop.
		self.unpublish();
	}

	fn unpublish(&self) {
		// A successor may already hold this name (`daemon --replace`): unlink
		// only the socket this host itself published.
		let ours = self.published.lock().take();
		if ours.is_some() && socket::identity(&self.socket_path()).ok() == ours {
			let _ = std::fs::remove_file(self.socket_path());
		}
		#[cfg(unix)]
		self.tcp.lock().clear();
		let _ = std::fs::remove_dir_all(&self.nodes);
	}

	fn publish_listener(self: &Arc<Self>, listener: crate::transport::typed::LocalListener) {
		*self.published.lock() = socket::identity(&self.socket_path()).ok();
		let task = tokio::spawn(socket::accept(Arc::downgrade(self), listener));
		if let Err(task) = self.inner.set(task) {
			// A second acceptor on a rebound name; it ends with the stop signal.
			drop(task);
		}
	}

	/// Become the host of a project another host serves. The addresses this
	/// composition's nodes are reached on are bound first, beside the old
	/// host's, so nothing refuses a connection while the swap happens; then
	/// the old host is stopped, which releases the stores and sockets its
	/// nodes hold; only then does this one compose. Composing beside it
	/// instead would mean two hosts holding one project's single-writer
	/// stores, which is what makes a plain restart fail today.
	pub async fn takeover(self: &Arc<Self>, old: &Peer) -> Result<()> {
		use std::sync::atomic::Ordering;
		let settings = crate::settings::host();
		self.deferred.store(true, Ordering::SeqCst);
		let pid = old
			.call("snapshot", Value::Null)
			.await
			.ok()
			.and_then(|snapshot| snapshot["host_pid"].as_u64())
			.map(|pid| pid as u32);
		#[cfg(unix)]
		let staged = {
			// Bound before the old host goes: a caller reaching one of these
			// addresses in the meantime waits in a backlog this host owns.
			for (id, address) in self.fronts()? {
				if let Err(error) = self.listener_for(&id, &address) {
					tracing::warn!(target: "cartridge", cartridge = %id, "{error}");
				}
			}
			let staged = self
				.sockets
				.join(format!("host.sock.{}", std::process::id()));
			let listener = socket::listen(&staged).await?;
			(staged, listener)
		};
		let _ = old.call("stop", Value::Null).await;
		if let Some(pid) = pid {
			let deadline = tokio::time::Instant::now() + settings.shutdown_timeout() * 2;
			while socket::alive(pid) && tokio::time::Instant::now() < deadline {
				tokio::time::sleep(Duration::from_millis(50)).await;
			}
		}
		#[cfg(unix)]
		{
			let (staged, mut listener) = staged;
			// The name is taken back at once, so a command arriving during the
			// composition below finds this host starting rather than none.
			std::fs::rename(&staged, self.socket_path()).map_err(|e| Error::file(&staged, e))?;
			listener.moved_to(self.socket_path());
			self.publish_listener(listener);
		}
		self.deferred.store(false, Ordering::SeqCst);
		self.reconcile().await?;
		Ok(())
	}

	/// Every enabled cartridge that names an address for the host to bind,
	/// with that address.
	#[cfg(unix)]
	fn fronts(self: &Arc<Self>) -> Result<Vec<(String, String)>> {
		Ok(self
			.entries()?
			.iter()
			.filter(|entry| !entry.disabled)
			.filter_map(|entry| {
				let plan = self.plan(entry).ok()?;
				Some((plan.id.clone(), plan.listener.clone()?))
			})
			.collect())
	}

	/// The listener a node inherits: bound once per cartridge id and kept
	/// across its restarts. Port 0 in the configured address takes the port
	/// this project last bound (`<run>/<id>.port`), so a replacing host and
	/// a restarted node keep the address a launched agent was given.
	#[cfg(unix)]
	pub(crate) fn listener_for(&self, id: &str, address: &str) -> Result<std::os::fd::RawFd> {
		use std::os::fd::AsRawFd;
		let want: std::net::SocketAddr = address
			.parse()
			.map_err(|e| Error::Descriptor(format!("{id}: listener `{address}`: {e}")))?;
		let mut held = self.tcp.lock();
		if let Some(listener) = held.get(id) {
			let same = listener.local_addr().is_ok_and(|bound| {
				want.port() == 0
					|| bound == want
					|| self.private.load(std::sync::atomic::Ordering::SeqCst)
			});
			if same {
				return Ok(listener.as_raw_fd());
			}
			held.remove(id);
		}
		// A private host serves nobody: a fresh port of its own, never the
		// project's, which SO_REUSEPORT would otherwise let it share with the daemon.
		if self.private.load(std::sync::atomic::Ordering::SeqCst) {
			let listener = std::net::TcpListener::bind((want.ip(), 0))
				.map_err(|e| Error::Descriptor(format!("{id}: listen on {}:0: {e}", want.ip())))?;
			let fd = listener.as_raw_fd();
			held.insert(id.to_owned(), listener);
			return Ok(fd);
		}
		let memo = self.sockets.join(format!("{}.port", socket::file_name(id)));
		let remembered = std::fs::read_to_string(&memo)
			.ok()
			.and_then(|text| text.trim().parse::<std::net::SocketAddr>().ok())
			.filter(|last| want.port() == 0 && last.ip() == want.ip());
		let listener = remembered
			.and_then(|last| bind_reuse(last).ok())
			.map(Ok)
			.unwrap_or_else(|| bind_reuse(want))
			.map_err(|e| Error::Descriptor(format!("{id}: listen on {want}: {e}")))?;
		if let Ok(bound) = listener.local_addr() {
			let _ = socket::write_private(&memo, &bound.to_string());
		}
		let fd = listener.as_raw_fd();
		held.insert(id.to_owned(), listener);
		Ok(fd)
	}

	fn peer_of(&self, id: &str) -> Option<Peer> {
		self.slots
			.lock()
			.iter()
			.find(|s| s.entry.id == id && s.state == State::Active)
			.and_then(|s| s.running.as_ref().map(|r| r.peer.clone()))
	}

	fn sender(&self, name: &str, data: &Value) -> Result<Ctx> {
		let directory = {
			let slots = self.slots.lock();
			let planned = |s: &&Slot| s.state != State::Failed || s.running.is_some();
			let plans: Vec<Arc<Plan>> = slots
				.iter()
				.filter(planned)
				.filter_map(|s| s.plan.clone())
				.collect();
			let active: Vec<Arc<Plan>> = slots
				.iter()
				.filter(|s| s.state == State::Active)
				.filter_map(|s| s.plan.clone())
				.collect();
			plan::host_directory(self, &active, &plan::catalogue(&plans).0)
		};
		if !directory.events.contains_key(name) {
			return Err(Error::NotProvided(name.to_owned()));
		}
		if self.ctx.directory() != directory {
			self.ctx.set_directory(directory);
		}
		self.ctx.validate(name, data).map_err(Error::Argument)?;
		Ok(self.ctx.clone())
	}

	pub async fn send_to(&self, id: &str, name: &str, data: Value) -> Result<Value> {
		let ctx = self.sender(name, &data)?;
		let outcome = ctx
			.ask(id, name, data)
			.await
			.map_err(|_| Error::Unavailable {
				key: name.to_owned(),
				why: format!("`{id}` is not active"),
			})?;
		match outcome {
			Outcome::Answered { data, .. } => Ok(data),
			Outcome::Declined { .. } => Ok(Value::Null),
			Outcome::Unavailable { error, .. } => Err(Error::Unavailable {
				key: name.to_owned(),
				why: error,
			}),
			other => Err(Error::Remote(other.error().unwrap_or_default())),
		}
	}

	/// The plans of the cartridges serving right now.
	pub(crate) fn active_plans(&self) -> Vec<Arc<Plan>> {
		self.slots
			.lock()
			.iter()
			.filter(|s| s.state == State::Active)
			.filter_map(|s| s.plan.clone())
			.collect()
	}

	pub async fn bail(&self, name: &str, data: Value) -> Result<Option<Value>> {
		// ASP is the host's own service, so it has no listener to find.
		if name == crate::asp::SERVICE {
			return self.asp(data).await.map(Some);
		}
		let ctx = self.sender(name, &data)?;
		// Shared with other requests that may swap it between `sender`'s set
		// and this read, so never index it: unknown or empty both mean nobody
		// can answer.
		if ctx
			.events()
			.get(name)
			.is_none_or(|e| e.listeners.is_empty())
		{
			return Err(Error::Unavailable {
				key: name.to_owned(),
				why: "no active listener".into(),
			});
		}
		ctx.bail(name, data).await.map_err(Error::Remote)
	}

	pub async fn gather(&self, name: &str, data: Value) -> Result<Vec<Outcome>> {
		let ctx = self.sender(name, &data)?;
		ctx.gather(name, data).await.map_err(Error::Remote)
	}

	pub(crate) fn caller(&self, token: &str) -> Option<String> {
		self.tokens
			.lock()
			.iter()
			.find(|((_, to), issued)| to.is_none() && *issued == token)
			.map(|((id, _), _)| id.clone())
	}

	pub fn cartridges(&self) -> Value {
		let slots = self.slots.lock();
		Value::Array(
			slots
				.iter()
				.filter(|s| s.running.is_some())
				.map(|s| {
					json!({
						"id": s.entry.id,
						"dir": s.entry.file(&self.dir).parent(),
						"generation": s.generation,
						"listen": s.plan.as_ref().map(|p| p.listen.clone()).unwrap_or_default(),
					})
				})
				.collect(),
		)
	}

	pub fn snapshot(&self) -> Value {
		let listeners = self.listeners.lock().clone();
		let slots = self.slots.lock();
		let existing = |root: &Path, name: &str| {
			let path = root.join(name);
			path.exists().then_some(path)
		};
		let entries: Vec<Value> = slots
			.iter()
			.map(|slot| {
				let file = slot.entry.file(&self.dir);
				let root = file.parent().unwrap_or(&self.dir).to_path_buf();
				let plan = slot.plan.as_deref();
				let needs = plan.map(|p| p.needs.clone()).unwrap_or_default();
				let dependencies: Vec<Value> = needs
					.iter()
					.map(|key| json!({ "key": key, "listeners": listeners.get(key) }))
					.collect();
				let sources: Vec<Value> = slot
					.sources
					.iter()
					.map(|source| json!({ "path": source.path, "changed": source.changed() }))
					.collect();
				let source_reference = std::fs::read_to_string(root.join("cartridge.json"))
					.ok()
					.and_then(|text| serde_json::from_str::<Value>(&text).ok())
					.and_then(|manifest| manifest["source"].as_str().map(str::to_owned));
				let implementation = ["Cargo.toml", "package.json"]
					.iter()
					.any(|name| root.join(name).is_file());
				json!({
					"id": slot.entry.id,
					"state": slot.state,
					"error": slot.error,
					"path": file,
					"dir": root,
					"needs": needs,
					"events": plan.map(|p| p.events.keys().cloned().collect::<Vec<_>>()).unwrap_or_default(),
					"listen": plan.map(|p| p.listen.clone()).unwrap_or_default(),
					"dependencies": dependencies,
					"sources": sources,
					"context": {
						"readme": existing(&root, "README.md"),
						"memos": existing(&root, ".cartridge/memos"),
						"tests": existing(&root, "tests"),
						"source": root,
						"implementation_source": implementation.then_some(&root),
						"source_reference": source_reference,
					},
				})
			})
			.collect();
		json!({
			"host_pid": std::process::id(),
			"descriptor": self.descriptor,
			"cartridge_root": self.dir,
			"entries": entries,
		})
	}

	pub async fn open(&self, id: &str) -> Result<(Peer, mpsc::Receiver<Incoming>)> {
		if self.peer_of(id).is_none() {
			return Err(Error::Unavailable {
				key: id.to_owned(),
				why: "not active".into(),
			});
		}
		connect(&self.socket(id), &self.node_token(id)).await
	}
}

/// Reuse-address and reuse-port: a replacing host binds beside the host it
/// replaces, and the kernel hands new connections to whichever still accepts.
#[cfg(unix)]
fn bind_reuse(address: std::net::SocketAddr) -> std::io::Result<std::net::TcpListener> {
	use socket2::{Domain, Protocol, Socket, Type};
	let socket = Socket::new(
		Domain::for_address(address),
		Type::STREAM,
		Some(Protocol::TCP),
	)?;
	socket.set_reuse_address(true)?;
	socket.set_reuse_port(true)?;
	socket.bind(&address.into())?;
	socket.listen(128)?;
	Ok(socket.into())
}

pub(crate) async fn connect(
	socket: &Path,
	token: &str,
) -> Result<(Peer, mpsc::Receiver<Incoming>)> {
	let adapter =
		crate::transport::typed::connect(&crate::transport::typed::Endpoint::local(socket))
			.await
			.map_err(|e| Error::Remote(format!("{}: {e}", socket.display())))?;
	let (peer, incoming) = Peer::spawn(adapter, None);
	let auth = async { peer.call("auth", json!({ "token": token })).await };
	tokio::time::timeout(crate::settings::host().startup_timeout(), auth)
		.await
		.map_err(|_| Error::Remote(format!("{}: auth timed out", socket.display())))?
		.map_err(|e| Error::Remote(e.message))?;
	Ok((peer, incoming))
}
