//! The host: reads the profile, starts every cartridge on its own socket, hands
//! each its directory, and stops them. Calls and events between cartridges do
//! not pass through it.

mod lua;
mod plan;
mod process;
mod run;
pub mod socket;
mod watch;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::future::BoxFuture;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::{broadcast, mpsc};
use transport::cartridge::{Directory, Scope};
use transport::rpc::{Incoming, Peer};

use crate::error::{Error, Result};
use crate::loader::{Entry, Source};

pub use plan::{executable, Kind, Plan};

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
	pub provide: Vec<String>,
	pub needs: Vec<String>,
	pub on: Vec<String>,
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

pub struct Host {
	pub(crate) dir: PathBuf,
	pub(crate) profile: PathBuf,
	pub(crate) yolo: bool,
	pub(crate) lua: mlua::Lua,
	pub(crate) solo: Mutex<Option<Vec<Entry>>>,
	sockets: PathBuf,
	host_token: String,
	edges: Mutex<HashMap<(String, String, Scope), String>>,
	slots: Mutex<Vec<Slot>>,
	providers: Mutex<HashMap<String, String>>,
	op: tokio::sync::Mutex<()>,
	lifecycle: broadcast::Sender<Value>,
	inner: std::sync::OnceLock<tokio::task::JoinHandle<()>>,
}

impl Drop for Host {
	fn drop(&mut self) {
		if let Some(task) = self.inner.get() {
			task.abort();
		}
		let _ = std::fs::remove_dir_all(&self.sockets);
	}
}

impl Host {
	/// `dir` holds the cartridges, `profile` the `init.lua` and `config.lua`.
	pub fn new(
		dir: impl Into<PathBuf>,
		profile: impl Into<PathBuf>,
		yolo: bool,
	) -> Result<Arc<Self>> {
		let dir: PathBuf = dir.into();
		let profile: PathBuf = profile.into();
		let lua = crate::lua::interpreter()?;
		let limit = crate::settings::host().lua_memory_bytes;
		if limit > 0 {
			lua.set_memory_limit(limit)?;
		}
		let profile = profile.canonicalize().unwrap_or(profile);
		Ok(Arc::new(Self {
			sockets: socket::run_dir(&profile)?,
			dir: dir.canonicalize().unwrap_or(dir),
			profile,
			yolo,
			lua,
			solo: Mutex::new(None),
			host_token: transport::token(),
			edges: Mutex::default(),
			slots: Mutex::default(),
			providers: Mutex::default(),
			op: tokio::sync::Mutex::new(()),
			lifecycle: broadcast::channel(crate::settings::host().lifecycle_queue).0,
			inner: std::sync::OnceLock::new(),
		}))
	}

	pub fn dir(&self) -> &Path {
		&self.dir
	}

	pub fn profile(&self) -> &Path {
		&self.profile
	}

	pub(crate) fn host_token(&self) -> &str {
		&self.host_token
	}

	/// The socket cartridges reach the host on.
	pub(crate) fn inner_socket(&self) -> PathBuf {
		self.sockets.join("host.sock")
	}

	/// The socket a cartridge serves on, stable for the life of this host.
	pub fn socket(&self, id: &str) -> PathBuf {
		self.sockets.join(socket::file_name(id))
	}

	/// State changes of every cartridge, as `{id, state, error}`.
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
		let providers = self.providers.lock();
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
							providers
								.get(*key)
								.is_none_or(|p| !active.contains(p.as_str()))
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
					provide: plan.map(|p| p.provide.clone()).unwrap_or_default(),
					needs,
					on: plan.map(|p| p.on.clone()).unwrap_or_default(),
					socket: self.socket(&slot.entry.id),
				}
			})
			.collect()
	}

	/// Load the profile and bring the running cartridges in line with it.
	pub async fn reconcile(self: &Arc<Self>) -> Result<()> {
		let _op = self.op.lock().await;
		if self.inner.get().is_none() {
			let listener = socket::listen(&self.inner_socket()).await?;
			let _ = self
				.inner
				.set(tokio::spawn(socket::accept(Arc::downgrade(self), listener)));
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

	/// Recompute providers and directories, and tell running cartridges what changed.
	async fn rewire(self: &Arc<Self>) {
		let updates: Vec<(Peer, Directory)> = {
			let mut slots = self.slots.lock();
			let all: Vec<Arc<Plan>> = slots
				.iter()
				.filter(|s| s.state != State::Failed || s.running.is_some())
				.filter_map(|s| s.plan.clone())
				.collect();
			let (_, clashes) = plan::providers(&all);
			for slot in slots.iter_mut() {
				if let Some(clash) = clashes.get(&slot.entry.id) {
					if slot.running.is_none() {
						slot.state = State::Failed;
						slot.error = Some(clash.clone());
					}
				}
			}
			let plans: Vec<Arc<Plan>> = slots
				.iter()
				.filter(|s| s.state != State::Failed || s.running.is_some())
				.filter_map(|s| s.plan.clone())
				.collect();
			let (providers, _) = plan::providers(&plans);
			let mut updates = Vec::new();
			for slot in slots.iter_mut() {
				let Some(plan) = slot.plan.clone() else {
					continue;
				};
				let directory = plan::directory(self, &plan, &plans, &providers);
				if directory != slot.directory {
					slot.directory = directory.clone();
					if let Some(running) = &slot.running {
						updates.push((running.peer.clone(), directory));
					}
				}
			}
			*self.providers.lock() = providers;
			updates
		};
		for (peer, directory) in updates {
			let _ = peer
				.call("directory", json!({ "directory": directory }))
				.await;
		}
	}

	/// Start every waiting cartridge whose needs are all served, wave by wave.
	async fn start_ready(self: &Arc<Self>) {
		loop {
			let ready: Vec<(String, Arc<Plan>, Directory, u64)> = {
				let mut slots = self.slots.lock();
				let providers = self.providers.lock().clone();
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
					let served = plan
						.needs
						.iter()
						.all(|key| providers.get(key).is_some_and(|p| active.contains(p)));
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
		match &plan.kind {
			Kind::Process(command) => {
				process::start(self, plan, command, directory, generation).await
			}
			Kind::Lua(apply) => lua::start(self, plan, apply.clone(), directory).await,
		}
	}

	/// A cartridge process ended on its own.
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

	/// Stop and start one cartridge again from its current sources.
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
		self.stop_slot(id).await;
		let rewire = {
			let mut slots = self.slots.lock();
			let slot = slots
				.iter_mut()
				.find(|s| s.entry.id == id)
				.expect("slot of a known entry");
			match &plan {
				Ok(plan) => {
					let rewire = slot
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

	/// Replace every cartridge whose sources are among `paths` and changed.
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

	/// Stop every cartridge.
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
	}

	fn peer_of(&self, id: &str) -> Option<Peer> {
		self.slots
			.lock()
			.iter()
			.find(|s| s.entry.id == id && s.state == State::Active)
			.and_then(|s| s.running.as_ref().map(|r| r.peer.clone()))
	}

	/// The cartridge providing `key`, when one is planned.
	pub fn provider(&self, key: &str) -> Option<String> {
		self.providers.lock().get(key).cloned()
	}

	/// Call `key` on the cartridge providing it.
	pub async fn call(&self, key: &str, args: Value) -> Result<Value> {
		let provider = self
			.provider(key)
			.ok_or_else(|| Error::NotProvided(key.to_owned()))?;
		let peer = self.peer_of(&provider).ok_or_else(|| Error::Unavailable {
			key: key.to_owned(),
			why: format!("`{provider}` is not active"),
		})?;
		let trace =
			transport::cartridge::trace().unwrap_or_else(|| crate::trace::mint().to_string());
		peer.call("call", json!({ "key": key, "args": args, "trace": trace }))
			.await
			.map_err(|e| Error::Remote(e.message))
	}

	/// Send an event to every active listener; one answer or error per listener.
	pub async fn emit(
		&self,
		name: &str,
		data: Value,
	) -> Vec<(String, std::result::Result<Value, String>)> {
		let listeners: Vec<(String, Peer)> = self
			.slots
			.lock()
			.iter()
			.filter(|s| {
				s.state == State::Active
					&& s.plan
						.as_ref()
						.is_some_and(|p| p.on.iter().any(|n| n == name))
			})
			.filter_map(|s| Some((s.entry.id.clone(), s.running.as_ref()?.peer.clone())))
			.collect();
		futures::future::join_all(listeners.into_iter().map(|(id, peer)| {
			let data = data.clone();
			async move {
				let answer = peer
					.call("event", json!({ "name": name, "data": data }))
					.await
					.map_err(|e| e.message);
				(id, answer)
			}
		}))
		.await
	}

	/// The cartridge a host token was issued to.
	pub(crate) fn caller(&self, token: &str) -> Option<String> {
		self.edges
			.lock()
			.iter()
			.find(|((_, to, _), issued)| to == "host" && *issued == token)
			.map(|((from, _, _), _)| from.clone())
	}

	/// Enabled cartridges that are running, with their folders.
	pub fn cartridges(&self) -> Value {
		let slots = self.slots.lock();
		Value::Array(
			slots
				.iter()
				.filter(|s| s.running.is_some())
				.map(|s| json!({ "id": s.entry.id, "dir": s.entry.file(&self.dir).parent() }))
				.collect(),
		)
	}

	/// The composition as data: every entry, its state, wiring, sources and context paths.
	pub fn snapshot(&self) -> Value {
		let providers = self.providers.lock().clone();
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
					.map(|key| json!({ "key": key, "provider": providers.get(key) }))
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
					"inject": needs,
					"provide": plan.map(|p| p.provide.clone()).unwrap_or_default(),
					"on": plan.map(|p| p.on.clone()).unwrap_or_default(),
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
			"profile": self.profile,
			"cartridge_root": self.dir,
			"entries": entries,
		})
	}

	fn module(slot: &Slot) -> Option<&Path> {
		slot.sources
			.iter()
			.map(|source| source.path.as_path())
			.find(|path| {
				path.extension()
					.is_some_and(|e| e == "tsx" || e == "ts" || e == "js" || e == "jsx")
			})
	}

	/// Whether the profile grants `id` the client-module bridge.
	pub(crate) fn bridged(&self, id: &str) -> bool {
		self.slots
			.lock()
			.iter()
			.any(|s| s.entry.id == id && s.entry.config["bridge"] == true)
	}

	/// Active cartridges that ship a client module, and the services they provide.
	pub fn bridge_status(&self) -> Value {
		let slots = self.slots.lock();
		Value::Array(
			slots
				.iter()
				.filter(|s| s.state == State::Active)
				.filter_map(|s| {
					let module = Self::module(s)?;
					Some(json!({
						"id": s.entry.id,
						"generation": s.generation,
						"module": module,
						"services": s.plan.as_ref().map(|p| p.provide.clone()).unwrap_or_default(),
					}))
				})
				.collect(),
		)
	}

	/// A client module's call into a service its own cartridge provides.
	pub async fn bridge_call(
		&self,
		owner: &str,
		generation: u64,
		key: &str,
		args: Value,
	) -> Result<Value> {
		{
			let slots = self.slots.lock();
			let slot = slots
				.iter()
				.find(|s| s.entry.id == owner)
				.ok_or_else(|| Error::Reload("bridge owner unloaded".into()))?;
			if slot.state != State::Active || slot.generation != generation {
				return Err(Error::Reload(
					"bridge generation is no longer active".into(),
				));
			}
			if Self::module(slot).is_none() {
				return Err(Error::Reload("cartridge has no client module".into()));
			}
			if !slot
				.plan
				.as_ref()
				.is_some_and(|p| p.provide.iter().any(|k| k == key))
			{
				return Err(Error::Reload(
					"a bridge client may only call services provided by its own cartridge".into(),
				));
			}
		}
		self.call(key, args).await
	}

	/// A new connection to an active cartridge, authenticated as the host.
	pub async fn open(&self, id: &str) -> Result<(Peer, mpsc::UnboundedReceiver<Incoming>)> {
		if self.peer_of(id).is_none() {
			return Err(Error::Unavailable {
				key: id.to_owned(),
				why: "not active".into(),
			});
		}
		connect(&self.socket(id), &self.host_token).await
	}
}

pub(crate) async fn connect(
	socket: &Path,
	token: &str,
) -> Result<(Peer, mpsc::UnboundedReceiver<Incoming>)> {
	let adapter =
		transport::typed::connect(&transport::typed::Endpoint::Unix(socket.to_path_buf()))
			.await
			.map_err(|e| Error::Remote(format!("{}: {e}", socket.display())))?;
	let (peer, incoming) = Peer::spawn(adapter, None);
	peer.call("auth", json!({ "token": token }))
		.await
		.map_err(|e| Error::Remote(e.message))?;
	Ok((peer, incoming))
}
