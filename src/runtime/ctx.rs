//! A component's handle on the runtime: what it may provide, get, listen to
//! and dispatch, and the effects it holds until it is unloaded.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use parking_lot::Mutex;
use tokio::sync::watch;

use super::{Binding, Disposer, Error, Listener, Meta, Realm, Registered, Runtime, Uid, Value};

#[derive(Clone)]
pub struct Effect {
	armed: Arc<AtomicBool>,
	inverses: Arc<Mutex<Vec<Disposer>>>,
	done: watch::Receiver<bool>,
	cancel: watch::Sender<bool>,
}

impl Effect {
	pub async fn dispose(&self) {
		if self.armed.swap(false, Ordering::SeqCst) {
			let _ = self.cancel.send(true);
			let _ = self.done.clone().wait_for(|done| *done).await;
			let inverses = std::mem::take(&mut *self.inverses.lock());
			for inverse in inverses.into_iter().rev() {
				inverse().await;
			}
		}
	}
}

pub(crate) async fn execute<S, G, K, C>(
	mut stream: S,
	guard: G,
	mut sink: K,
	cancelled: C,
) -> Result<(), Error>
where
	S: futures::Stream<Item = Result<Disposer, Error>> + Unpin,
	G: Fn() -> bool,
	K: FnMut(Disposer),
	C: std::future::Future<Output = ()>,
{
	tokio::pin!(cancelled);
	while guard() {
		let next = stream.next();
		tokio::pin!(next);
		let (item, stopping) = tokio::select! {
			biased;
			_ = &mut cancelled => {
				// Give an in-flight effect a brief chance to yield its inverse.
				// A stream that never yields cannot hold retirement indefinitely.
				(tokio::time::timeout(crate::settings::host().lifecycle_drain(), &mut next).await.ok().flatten(), true)
			},
			item = &mut next => (item, false),
		};
		match item {
			Some(Ok(d)) => sink(d),
			Some(Err(e)) => return Err(e),
			None => break,
		}
		if stopping {
			break;
		}
	}
	Ok(())
}

#[derive(Clone)]
pub struct Ctx {
	pub(crate) rt: Arc<Runtime>,
	pub(crate) fiber: Uid,
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
		self.rt
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
		let (cancel, mut cancelled) = watch::channel(false);
		let effect = Effect {
			armed: armed.clone(),
			inverses: inverses.clone(),
			done: done_rx,
			cancel,
		};
		let rt = self.rt.clone();
		let fiber = self.fiber;
		tokio::spawn(async move {
			let result = execute(
				stream.boxed(),
				|| armed.load(Ordering::SeqCst),
				|d| inverses.lock().push(d),
				async move {
					let _ = cancelled.wait_for(|stopped| *stopped).await;
				},
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
			cancel: watch::channel(false).0,
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
		let (f, route) =
			if let Some(fiber) = reg.fibers.get(&self.fiber).filter(|fiber| fiber.resident) {
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
		reg.listeners
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
		reg.listeners
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

	/// Every listener's answer, kept with the fiber that gave it. This is the
	/// dispatch an announce needs: ask the composition a question and keep all
	/// of the replies, attributed, instead of the first one ([`Ctx::bail`]) or
	/// none of them ([`Ctx::parallel`]).
	///
	/// A listener that answers nothing contributes nothing. A listener that
	/// fails fails its own fiber, as with [`Ctx::emit`] — the error lands on
	/// the failing cartridge's channel and the caller is not the party that
	/// broke — so one cartridge that cannot answer never costs the caller the
	/// answers the rest of the composition gave.
	pub async fn gather(&self, name: &str, payload: Value) -> Vec<(Uid, Value)> {
		let listeners = self.listeners(name);
		let answers =
			futures::future::join_all(listeners.iter().map(|(_, f)| f(payload.clone()))).await;
		let mut gathered = Vec::new();
		for ((uid, _), answer) in listeners.into_iter().zip(answers) {
			match answer {
				Ok(Some(value)) => gathered.push((uid, value)),
				Ok(None) => {}
				Err(e) => self.rt.fail(uid, e),
			}
		}
		gathered
	}
}
