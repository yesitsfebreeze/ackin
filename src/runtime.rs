use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures::StreamExt;
use parking_lot::Mutex;
use serde_json::json;
use tokio::sync::{broadcast, watch};

pub use crate::fiber::FiberHandle;

pub type Uid = u64;
pub type Value = Arc<dyn Any + Send + Sync>;
pub type Meta = serde_json::Value;
pub type Disposer = Box<dyn FnOnce() -> BoxFuture<'static, ()> + Send>;
pub type Apply = Arc<dyn Fn(Ctx) -> BoxStream<'static, Result<Disposer, Error>> + Send + Sync>;
pub type Listener =
	Arc<dyn Fn(Value) -> BoxFuture<'static, Result<Option<Value>, Error>> + Send + Sync>;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Realm(pub(crate) u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum State {
	Inactive,
	Loading,
	Active,
	Unloading,
	Failed,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum Error {
	#[error("undeclared access to `{0}`")]
	Undeclared(String),
	#[error("inactive access to `{0}`")]
	Inactive(String),
	#[error("`{0}` is already provided in this realm")]
	Provided(String),
	#[error("`{0}` is not in this component's provision")]
	NotProvidable(String),
	#[error("{0}")]
	Apply(String),
	#[error("listeners failed: {0:?}")]
	Listeners(Vec<String>),
	#[error("fiber is gone")]
	Gone,
}

pub struct Component {
	pub(crate) resident: bool,
	pub(crate) staged: bool,
	pub name: String,
	pub inject: Vec<String>,
	pub provide: Vec<String>,
	pub apply: Apply,
	pub reload: crate::reload::Reload,
}

impl Component {
	pub fn new(name: impl Into<String>, apply: Apply) -> Self {
		Self {
			resident: false,
			staged: false,
			name: name.into(),
			inject: Vec::new(),
			provide: Vec::new(),
			apply,
			reload: crate::reload::Reload::default(),
		}
	}

	pub fn inject<I: IntoIterator<Item = S>, S: Into<String>>(mut self, keys: I) -> Self {
		self.inject = keys.into_iter().map(Into::into).collect();
		self
	}

	pub fn provide<I: IntoIterator<Item = S>, S: Into<String>>(mut self, keys: I) -> Self {
		self.provide = keys.into_iter().map(Into::into).collect();
		self
	}
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Transition {
	pub uid: Uid,
	pub name: String,
	pub state: State,
	pub error: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct FiberInfo {
	pub uid: Uid,
	pub name: String,
	pub parent: Option<Uid>,
	pub state: State,
	pub inject: Vec<String>,
	pub provide: Vec<String>,
	pub error: Option<String>,
}

pub(crate) struct Binding {
	pub(crate) provider: Uid,
	pub(crate) value: Value,
}

pub(crate) struct Registered {
	pub(crate) uid: Uid,
	id: u64,
	pub(crate) f: Listener,
	pub(crate) route: Option<Arc<Mutex<Option<Listener>>>>,
}

pub(crate) type View = Vec<(String, Realm, Uid)>;

pub(crate) struct Fiber {
	pub(crate) resident: bool,
	pub(crate) staged: bool,
	pub(crate) reload: crate::reload::Reload,
	pub(crate) name: String,
	pub(crate) parent: Option<Uid>,
	pub(crate) inject: Vec<String>,
	pub(crate) provide: Vec<String>,
	pub(crate) isolate: Arc<HashMap<String, Realm>>,
	pub(crate) intercept: Arc<HashMap<String, Meta>>,
	pub(crate) apply: Apply,
	pub(crate) state: State,
	pub(crate) target: Option<View>,
	pub(crate) committed: Option<View>,
	pub(crate) disposers: Vec<Disposer>,
	pub(crate) inertia: bool,
	pub(crate) retired: bool,
	pub(crate) error: Option<String>,
	pub(crate) changed: watch::Sender<u64>,
}

impl Fiber {
	pub(crate) fn new(
		component: Component,
		parent: Option<Uid>,
		isolate: Arc<HashMap<String, Realm>>,
		intercept: Arc<HashMap<String, Meta>>,
	) -> Self {
		Self {
			resident: component.resident,
			staged: component.staged,
			reload: component.reload,
			name: component.name,
			parent,
			inject: component.inject,
			provide: component.provide,
			isolate,
			intercept,
			apply: component.apply,
			state: State::Inactive,
			target: None,
			committed: None,
			disposers: Vec::new(),
			inertia: false,
			retired: false,
			error: None,
			changed: watch::channel(0).0,
		}
	}
}

pub(crate) struct Registry {
	pub(crate) next: u64,
	pub(crate) fibers: HashMap<Uid, Fiber>,
	pub(crate) realms: HashMap<String, Realm>,
	pub(crate) store: HashMap<Realm, Binding>,
	pub(crate) listeners: HashMap<String, Vec<Registered>>,
}

impl Registry {
	pub(crate) fn fresh(&mut self) -> u64 {
		self.next += 1;
		self.next
	}

	pub(crate) fn realm(&self, isolate: &HashMap<String, Realm>, key: &str) -> Option<Realm> {
		isolate.get(key).or_else(|| self.realms.get(key)).copied()
	}

	pub(crate) fn realm_of(&mut self, isolate: &HashMap<String, Realm>, key: &str) -> Realm {
		if let Some(r) = self.realm(isolate, key) {
			return r;
		}
		let r = Realm(self.fresh());
		self.realms.insert(key.to_string(), r);
		r
	}

	pub(crate) fn active_provider(&self, realm: Realm) -> Option<Uid> {
		let b = self.store.get(&realm)?;
		let f = self.fibers.get(&b.provider)?;
		(f.state == State::Active).then_some(b.provider)
	}

	pub(crate) fn value(&self, realm: Realm) -> Option<Value> {
		self.store.get(&realm).map(|b| b.value.clone())
	}

	pub(crate) fn owns(&self, uid: Uid, key: &str) -> Option<Realm> {
		let f = self.fibers.get(&uid)?;
		if !f.provide.iter().any(|k| k == key) {
			return None;
		}
		let realm = self.realm(&f.isolate, key)?;
		self
			.store
			.get(&realm)
			.filter(|b| b.provider == uid)
			.map(|_| realm)
	}

	pub(crate) fn provided(&self, uid: Uid) -> Vec<(String, Realm)> {
		let Some(f) = self.fibers.get(&uid) else {
			return Vec::new();
		};
		f.provide
			.iter()
			.filter_map(|k| Some((k.clone(), self.owns(uid, k)?)))
			.collect()
	}

	pub(crate) fn push_disposer(&mut self, uid: Uid, d: Disposer) {
		if let Some(f) = self.fibers.get_mut(&uid) {
			f.disposers.push(d);
		}
	}
}

pub struct Runtime {
	pub(crate) reg: Mutex<Registry>,
	lifecycle: broadcast::Sender<Transition>,
	/// The stream every fiber of this runtime publishes to. It lives here so a
	/// failure needs no second path: [`Runtime::fail`] is the one place an error
	/// lands, and it is where the error becomes an event.
	pub(crate) stream: Arc<crate::stream::Stream>,
}

pub const ROOT: Uid = 0;

impl Runtime {
	pub fn new() -> Arc<Self> {
		let root = Component::new("root", Arc::new(|_| futures::stream::empty().boxed()));
		let mut fibers = HashMap::new();
		fibers.insert(
			ROOT,
			Fiber {
				state: State::Active,
				target: Some(Vec::new()),
				committed: Some(Vec::new()),
				..Fiber::new(
					root,
					None,
					Arc::new(HashMap::new()),
					Arc::new(HashMap::new()),
				)
			},
		);
		let (lifecycle, _) = broadcast::channel(256);
		Arc::new(Self {
			reg: Mutex::new(Registry {
				next: ROOT,
				fibers,
				realms: HashMap::new(),
				store: HashMap::new(),
				listeners: HashMap::new(),
			}),
			lifecycle,
			stream: Arc::new(crate::stream::Stream::new()),
		})
	}

	pub fn ctx(self: &Arc<Self>) -> Ctx {
		self.ctx_of(&self.reg.lock(), ROOT)
	}

	pub fn lifecycle(&self) -> broadcast::Receiver<Transition> {
		self.lifecycle.subscribe()
	}

	pub fn stream(&self) -> &Arc<crate::stream::Stream> {
		&self.stream
	}

	pub fn fibers(&self) -> Vec<FiberInfo> {
		let reg = self.reg.lock();
		let mut out: Vec<_> = reg
			.fibers
			.iter()
			.map(|(uid, f)| FiberInfo {
				uid: *uid,
				name: f.name.clone(),
				parent: f.parent,
				state: f.state,
				inject: f.inject.clone(),
				provide: f.provide.clone(),
				error: f.error.clone(),
			})
			.collect();
		out.sort_by_key(|f| f.uid);
		out
	}

	pub(crate) fn state_of(&self, uid: Uid) -> Option<State> {
		self.reg.lock().fibers.get(&uid).map(|f| f.state)
	}

	pub(crate) fn set_state(&self, reg: &mut Registry, uid: Uid, state: State) {
		let Some(f) = reg.fibers.get_mut(&uid) else {
			return;
		};
		f.state = state;
		let _ = self.lifecycle.send(Transition {
			uid,
			name: f.name.clone(),
			state,
			error: f.error.clone(),
		});
		f.changed.send_modify(|v| *v += 1);
	}

	pub(crate) fn ctx_of(self: &Arc<Self>, reg: &Registry, uid: Uid) -> Ctx {
		let f = &reg.fibers[&uid];
		Ctx {
			rt: self.clone(),
			fiber: uid,
			isolate: f.isolate.clone(),
			intercept: f.intercept.clone(),
		}
	}

	pub(crate) fn revise(self: &Arc<Self>, uid: Uid, change: impl FnOnce(&mut Fiber)) {
		let mut reg = self.reg.lock();
		if let Some(f) = reg.fibers.get_mut(&uid) {
			change(f);
		}
		self.refresh(&mut reg, uid);
	}

	pub(crate) fn fail(self: &Arc<Self>, uid: Uid, error: Error) {
		let (name, stream) = {
			let reg = self.reg.lock();
			match reg.fibers.get(&uid) {
				Some(f) => (f.name.clone(), self.stream.clone()),
				None => return,
			}
		};
		// The failure is published on the cartridge's own channel, so whoever
		// watches that channel learns of it even if nobody called into it.
		self.revise(uid, |f| f.error = Some(error.to_string()));
		stream.publish(&name, &name, crate::stream::Kind::Error, json!(error.to_string()));
	}
}

#[derive(Clone)]
pub struct Effect {
	armed: Arc<AtomicBool>,
	inverses: Arc<Mutex<Vec<Disposer>>>,
	done: watch::Receiver<bool>,
}

impl Effect {
	pub async fn dispose(&self) {
		if self.armed.swap(false, Ordering::SeqCst) {
			let _ = self.done.clone().wait_for(|done| *done).await;
			let inverses = std::mem::take(&mut *self.inverses.lock());
			for inverse in inverses.into_iter().rev() {
				inverse().await;
			}
		}
	}
}

pub(crate) async fn execute<S, G, K>(mut stream: S, guard: G, mut sink: K) -> Result<(), Error>
where
	S: futures::Stream<Item = Result<Disposer, Error>> + Unpin,
	G: Fn() -> bool,
	K: FnMut(Disposer),
{
	while guard() {
		match stream.next().await {
			Some(Ok(d)) => sink(d),
			Some(Err(e)) => return Err(e),
			None => break,
		}
	}
	Ok(())
}

#[derive(Clone)]
pub struct Ctx {
	rt: Arc<Runtime>,
	fiber: Uid,
	pub(crate) isolate: Arc<HashMap<String, Realm>>,
	pub(crate) intercept: Arc<HashMap<String, Meta>>,
}

impl Ctx {
	pub fn runtime(&self) -> &Arc<Runtime> {
		&self.rt
	}

	pub fn fiber(&self) -> Uid {
		self.fiber
	}

	/// The keys this fiber declared it injects, in declaration order. A
	/// cartridge composed with a glob (`inject = {"tool.*"}`) learns from this
	/// what the profile actually handed it, instead of restating the list in
	/// its config.
	pub fn injections(&self) -> Vec<String> {
		self
			.rt
			.reg
			.lock()
			.fibers
			.get(&self.fiber)
			.map(|f| f.inject.clone())
			.unwrap_or_default()
	}

	/// Hold `effect`'s inverse on this fiber, so unloading it disposes.
	fn track(&self, effect: Effect) -> Effect {
		let handle = effect.clone();
		self.rt.reg.lock().push_disposer(
			self.fiber,
			Box::new(move || Box::pin(async move { handle.dispose().await })),
		);
		effect
	}

	pub fn effect<S>(&self, stream: S) -> Effect
	where
		S: futures::Stream<Item = Result<Disposer, Error>> + Send + 'static,
	{
		let armed = Arc::new(AtomicBool::new(true));
		let inverses = Arc::new(Mutex::new(Vec::new()));
		let (done_tx, done_rx) = watch::channel(false);
		let effect = Effect {
			armed: armed.clone(),
			inverses: inverses.clone(),
			done: done_rx,
		};
		let rt = self.rt.clone();
		let fiber = self.fiber;
		tokio::spawn(async move {
			let result = execute(
				stream.boxed(),
				|| armed.load(Ordering::SeqCst),
				|d| inverses.lock().push(d),
			)
			.await;
			let _ = done_tx.send(true);
			if let Err(e) = result {
				rt.fail(fiber, e);
			}
		});
		self.track(effect)
	}

	pub fn effect_sync<F>(&self, f: F) -> Effect
	where
		F: FnOnce() -> Disposer,
	{
		self.track(Effect {
			armed: Arc::new(AtomicBool::new(true)),
			inverses: Arc::new(Mutex::new(vec![f()])),
			done: watch::channel(true).1,
		})
	}

	pub fn provide(&self, key: &str, value: Value) -> Result<(), Error> {
		let mut reg = self.rt.reg.lock();
		let f = reg.fibers.get(&self.fiber).ok_or(Error::Gone)?;
		if !f.provide.iter().any(|k| k == key) {
			return Err(Error::NotProvidable(key.into()));
		}
		let value = if f.resident {
			crate::service::shared(value, f.reload.clone())
		} else {
			value
		};
		let realm = reg.realm_of(&self.isolate, key);
		if reg.store.contains_key(&realm) {
			drop(reg);
			return Err(Error::Provided(key.into()));
		}
		reg.store.insert(
			realm,
			Binding {
				provider: self.fiber,
				value,
			},
		);
		self.rt.notify(&mut reg, key, realm);
		let rt = self.rt.clone();
		let uid = self.fiber;
		let key = key.to_string();
		reg.push_disposer(
			uid,
			Box::new(move || {
				Box::pin(async move {
					let mut reg = rt.reg.lock();
					let removed = if let Some(realm) = reg.owns(uid, &key) {
						let removed = reg.store.remove(&realm);
						rt.notify(&mut reg, &key, realm);
						removed
					} else {
						None
					};
					drop(reg);
					drop(removed);
				})
			}),
		);
		Ok(())
	}

	pub fn get(&self, key: &str) -> Result<Value, Error> {
		let reg = self.rt.reg.lock();
		let mut uid = self.fiber;
		loop {
			let f = reg.fibers.get(&uid).ok_or(Error::Gone)?;
			let committed = f
				.committed
				.iter()
				.flatten()
				.find_map(|(k, realm, _)| (k == key).then_some(*realm));
			if let Some(realm) = committed.or_else(|| reg.owns(uid, key)) {
				return reg.value(realm).ok_or_else(|| Error::Inactive(key.into()));
			}
			if f.inject.iter().any(|k| k == key) {
				return Err(Error::Inactive(key.into()));
			}
			match f.parent {
				Some(p) => uid = p,
				None => return Err(Error::Undeclared(key.into())),
			}
		}
	}

	pub fn peek(&self, key: &str) -> Option<Value> {
		let reg = self.rt.reg.lock();
		reg.value(reg.realm(&self.isolate, key)?)
	}

	pub fn isolate(&self, key: &str) -> Ctx {
		let realm = Realm(self.rt.reg.lock().fresh());
		let mut table = (*self.isolate).clone();
		table.insert(key.into(), realm);
		Ctx {
			isolate: Arc::new(table),
			..self.clone()
		}
	}

	pub fn intercept(&self, key: &str, meta: Meta) -> Ctx {
		let mut table = (*self.intercept).clone();
		let merged = match (table.remove(key), meta) {
			(Some(Meta::Object(mut base)), Meta::Object(over)) => {
				base.extend(over);
				Meta::Object(base)
			}
			(_, over) => over,
		};
		table.insert(key.into(), merged);
		Ctx {
			intercept: Arc::new(table),
			..self.clone()
		}
	}

	pub fn meta(&self, key: &str) -> Meta {
		self.intercept.get(key).cloned().unwrap_or(Meta::Null)
	}

	pub fn on(&self, name: &str, f: Listener) {
		let mut reg = self.rt.reg.lock();
		let (f, route) = if let Some(fiber) = reg.fibers.get(&self.fiber).filter(|fiber| fiber.resident)
		{
			let reload = fiber.reload.clone();
			let route = Arc::new(Mutex::new(Some(f)));
			let current = route.clone();
			let wrapped: Listener = Arc::new(move |value| {
				let gate = reload.gate();
				let current = current.clone();
				Box::pin(async move {
					let _guard = gate.read_owned().await;
					let listener = current.lock().clone();
					match listener {
						Some(listener) => listener(value).await,
						None => Ok(None),
					}
				})
			});
			(wrapped, Some(route))
		} else {
			(f, None)
		};
		let id = reg.fresh();
		reg
			.listeners
			.entry(name.into())
			.or_default()
			.push(Registered {
				id,
				f,
				uid: self.fiber,
				route,
			});
		let rt = self.rt.clone();
		let name = name.to_string();
		reg.push_disposer(
			self.fiber,
			Box::new(move || {
				Box::pin(async move {
					let mut reg = rt.reg.lock();
					let mut removed = Vec::new();
					if let Some(list) = reg.listeners.get_mut(&name) {
						if let Some(index) = list.iter().position(|l| l.id == id) {
							removed.push(list.remove(index));
						}
					}
					drop(reg);
					drop(removed);
				})
			}),
		);
	}

	fn listeners(&self, name: &str) -> Vec<(Uid, Listener)> {
		let reg = self.rt.reg.lock();
		reg
			.listeners
			.get(name)
			.map(|l| {
				l.iter()
					.filter(|r| reg.fibers.get(&r.uid).is_some_and(|f| !f.staged))
					.map(|r| (r.uid, r.f.clone()))
					.collect()
			})
			.unwrap_or_default()
	}

	pub fn emit(&self, name: &str, payload: Value) {
		for (owner, f) in self.listeners(name) {
			let rt = self.rt.clone();
			let payload = payload.clone();
			tokio::spawn(async move {
				// A listener that fails fails its own fiber, so the error event
				// lands on the failing cartridge's channel — the caller is not
				// the party that broke.
				if let Err(e) = f(payload).await {
					rt.fail(owner, e);
				}
			});
		}
	}

	pub async fn bail(&self, name: &str, payload: Value) -> Result<Option<Value>, Error> {
		for (_, f) in self.listeners(name) {
			if let Some(v) = f(payload.clone()).await? {
				return Ok(Some(v));
			}
		}
		Ok(None)
	}

	pub async fn parallel(&self, name: &str, payload: Value) -> Result<(), Error> {
		let results = futures::future::join_all(
			self.listeners(name)
				.into_iter()
				.map(|(_, f)| f(payload.clone())),
		)
		.await;
		let errors: Vec<String> = results
			.into_iter()
			.filter_map(|r| r.err())
			.map(|e| e.to_string())
			.collect();
		if errors.is_empty() {
			Ok(())
		} else {
			Err(Error::Listeners(errors))
		}
	}
}
