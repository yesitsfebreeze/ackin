use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;

use crate::runtime::{
	execute, Component, Ctx, Error, Fiber, Meta, Realm, Registry, Runtime, State, Uid, View, ROOT,
};

impl Runtime {
	pub(crate) fn notify(
		self: &Arc<Self>,
		reg: &mut Registry,
		key: &str,
		realm: Realm,
	) -> Vec<Uid> {
		let affected: Vec<Uid> = reg
			.fibers
			.iter()
			.filter(|(_, f)| f.inject.iter().any(|k| k == key))
			.filter(|(_, f)| reg.realm(&f.isolate, key) == Some(realm))
			.map(|(uid, _)| *uid)
			.collect();
		for uid in &affected {
			self.refresh(reg, *uid);
		}
		affected
	}

	fn target(reg: &Registry, uid: Uid) -> Option<View> {
		let f = reg.fibers.get(&uid)?;
		if f.retired || f.error.is_some() {
			return None;
		}
		f.inject
			.iter()
			.map(|key| {
				let realm = reg.realm(&f.isolate, key)?;
				Some((key.clone(), realm, reg.active_provider(realm)?))
			})
			.collect()
	}

	pub(crate) fn refresh(self: &Arc<Self>, reg: &mut Registry, uid: Uid) {
		if uid == ROOT {
			return;
		}
		let target = Self::target(reg, uid);
		let Some(f) = reg.fibers.get_mut(&uid) else {
			return;
		};
		if f.target == target && f.state != State::Inactive {
			return;
		}
		let loading = target.is_some();
		f.target = target;
		f.changed.send_modify(|version| *version += 1);
		if f.inertia {
			return;
		}
		let idle = matches!(f.state, State::Inactive | State::Failed);
		let retired = f.retired;
		if loading {
			self.start(reg, uid, State::Loading);
		} else if !idle {
			self.start(reg, uid, State::Unloading);
		} else if retired {
			let removed = reg.fibers.remove(&uid);
			// Apply closures may own Lua values: never drop them under the registry lock.
			tokio::task::spawn_blocking(move || drop(removed));
		}
	}

	fn start(self: &Arc<Self>, reg: &mut Registry, uid: Uid, state: State) {
		if let Some(f) = reg.fibers.get_mut(&uid) {
			f.inertia = true;
		}
		self.set_state(reg, uid, state);
		match state {
			State::Loading => tokio::spawn(self.clone().reload(uid)),
			_ => tokio::spawn(self.clone().unload(uid)),
		};
	}

	async fn reload(self: Arc<Self>, uid: Uid) {
		let (apply, ctx, target0, mut changed) = {
			let mut reg = self.reg.lock();
			let Some(f) = reg.fibers.get_mut(&uid) else {
				return;
			};
			f.committed = f.target.clone();
			let target0 = f.target.clone();
			let apply = f.apply.clone();
			let changed = f.changed.subscribe();
			(apply, self.ctx_of(&reg, uid), target0, changed)
		};
		let result = execute(
			apply(ctx).boxed(),
			|| {
				self.reg
					.lock()
					.fibers
					.get(&uid)
					.is_some_and(|f| f.target == target0)
			},
			|d| self.reg.lock().push_disposer(uid, d),
			async {
				while changed.changed().await.is_ok() {
					if !self
						.reg
						.lock()
						.fibers
						.get(&uid)
						.is_some_and(|f| f.target == target0)
					{
						break;
					}
				}
			},
		)
		.await;
		let mut reg = self.reg.lock();
		let Some(f) = reg.fibers.get_mut(&uid) else {
			return;
		};
		if let Err(e) = result {
			f.error = Some(e.to_string());
			f.target = None;
		}
		if f.target != target0 {
			self.start(&mut reg, uid, State::Unloading);
			return;
		}
		f.inertia = false;
		self.set_state(&mut reg, uid, State::Active);
		for (key, realm) in reg.provided(uid) {
			self.notify(&mut reg, &key, realm);
		}
	}

	async fn unload(self: Arc<Self>, uid: Uid) {
		let dependents = {
			let mut reg = self.reg.lock();
			let mut dependents = Vec::new();
			for (key, realm) in reg.provided(uid) {
				dependents.extend(self.notify(&mut reg, &key, realm));
			}
			dependents
		};
		futures::future::join_all(dependents.into_iter().map(|d| self.wait_released(d, uid))).await;
		let disposers = {
			let mut reg = self.reg.lock();
			reg.fibers
				.get_mut(&uid)
				.map(|f| std::mem::take(&mut f.disposers))
				.unwrap_or_default()
		};
		for d in disposers.into_iter().rev() {
			d().await;
		}
		let mut reg = self.reg.lock();
		let Some(f) = reg.fibers.get_mut(&uid) else {
			return;
		};
		f.committed = None;
		if f.target.is_some() {
			self.start(&mut reg, uid, State::Loading);
			return;
		}
		f.inertia = false;
		let retired = f.retired;
		let state = if f.error.is_some() {
			State::Failed
		} else {
			State::Inactive
		};
		self.set_state(&mut reg, uid, state);
		if retired {
			let removed = reg.fibers.remove(&uid);
			drop(reg);
			drop(removed);
		}
	}

	async fn wait_until(&self, uid: Uid, holds: impl Fn(&Fiber) -> bool) {
		loop {
			let mut rx = {
				let reg = self.reg.lock();
				let Some(f) = reg.fibers.get(&uid) else {
					return;
				};
				if holds(f) {
					return;
				}
				f.changed.subscribe()
			};
			if rx.changed().await.is_err() {
				return;
			}
		}
	}

	async fn wait_released(&self, dependent: Uid, provider: Uid) {
		self.wait_until(dependent, |f| {
			!f.committed.iter().flatten().any(|(_, _, p)| *p == provider)
		})
		.await;
	}

	async fn wait_settled(&self, uid: Uid) {
		self.wait_until(uid, |f| !f.inertia).await;
	}

	pub(crate) async fn retire(self: &Arc<Self>, uid: Uid) {
		self.revise(uid, |f| f.retired = true);
		self.wait_settled(uid).await;
	}
}

#[derive(Clone)]
pub struct FiberHandle {
	rt: Arc<Runtime>,
	uid: Uid,
}

impl FiberHandle {
	pub fn uid(&self) -> Uid {
		self.uid
	}

	pub fn state(&self) -> Option<State> {
		self.rt.state_of(self.uid)
	}

	pub fn error(&self) -> Option<String> {
		self.rt
			.reg
			.lock()
			.fibers
			.get(&self.uid)
			.and_then(|f| f.error.clone())
	}

	pub async fn settled(&self) {
		self.rt.wait_settled(self.uid).await;
	}

	pub async fn dispose(&self) {
		self.rt.retire(self.uid).await;
	}

	pub async fn retry(&self) {
		self.rt.revise(self.uid, |f| f.error = None);
		self.rt.wait_settled(self.uid).await;
	}
}

impl Runtime {
	/// A handle onto a node of the tree that no profile slot owns.
	pub(crate) fn handle(self: &Arc<Self>, uid: Uid) -> FiberHandle {
		FiberHandle {
			rt: self.clone(),
			uid,
		}
	}

	/// The context a replacement is composed under: the old generation's
	/// parent, with the isolate and intercept the old generation ran with.
	pub(crate) fn ctx_under(
		self: &Arc<Self>,
		parent: Uid,
		isolate: Arc<HashMap<String, Realm>>,
		intercept: Arc<HashMap<String, Meta>>,
	) -> Ctx {
		Ctx {
			rt: self.clone(),
			fiber: parent,
			isolate,
			intercept,
		}
	}
}

impl Ctx {
	pub fn cartridge(&self, component: Component) -> FiberHandle {
		let rt = self.runtime().clone();
		let mut reg = rt.reg.lock();
		let uid = reg.fresh();
		reg.fibers.insert(
			uid,
			Fiber::new(
				component,
				Some(self.fiber()),
				self.isolate.clone(),
				self.intercept.clone(),
			),
		);
		let retire = rt.clone();
		reg.push_disposer(
			self.fiber(),
			Box::new(move || Box::pin(async move { retire.retire(uid).await })),
		);
		rt.refresh(&mut reg, uid);
		drop(reg);
		FiberHandle { rt, uid }
	}

	pub fn error(&self, message: impl Into<String>) -> Error {
		Error::Apply(message.into())
	}
}

impl Runtime {
	/// Publish a prepared generation without withdrawing its stable services.
	pub(crate) fn switch(&self, old: Uid, new: Uid) -> Result<(), Error> {
		let mut retired_values = Vec::new();
		let mut retired_bindings = Vec::new();
		let mut retired_callbacks = Vec::new();
		let mut retired_routes = Vec::new();
		let mut reg = self.reg.lock();
		let previous = reg.fibers.get(&old).ok_or(Error::Gone)?;
		if previous.state != State::Active {
			return Err(Error::Apply(
				"previous generation is no longer active".into(),
			));
		}
		let isolate = previous.isolate.clone();
		let keys = previous.provide.clone();
		let candidate = reg.fibers.get(&new).ok_or(Error::Gone)?;
		if candidate.state != State::Active || candidate.provide != keys {
			return Err(Error::Apply(
				"replacement must be active and preserve provided keys".into(),
			));
		}
		let mut bindings = Vec::new();
		for key in &keys {
			let from = reg
				.owns(new, key)
				.ok_or_else(|| Error::Inactive(key.clone()))?;
			let to = reg
				.owns(old, key)
				.ok_or_else(|| Error::Inactive(key.clone()))?;
			let previous = reg.value(to).ok_or(Error::Gone)?;
			let next = reg.value(from).ok_or(Error::Gone)?;
			if previous.downcast_ref::<crate::service::Service>().is_none()
				|| next.downcast_ref::<crate::service::Service>().is_none()
			{
				return Err(Error::Apply(
					"replacement requires resident services".into(),
				));
			}
			bindings.push((from, to, previous, next));
		}
		for (from, to, previous, next) in bindings {
			retired_values.push(
				previous
					.downcast_ref::<crate::service::Service>()
					.unwrap()
					.switch(
						next.downcast_ref::<crate::service::Service>()
							.unwrap()
							.value(),
					),
			);
			retired_bindings.push(reg.store.remove(&from));
			retired_bindings.push(reg.store.insert(
				to,
				crate::runtime::Binding {
					provider: new,
					value: previous,
				},
			));
		}
		for listeners in reg.listeners.values_mut() {
			let old_routes: Vec<_> = listeners
				.iter()
				.filter(|r| r.uid == old)
				.filter_map(|r| r.route.clone().map(|route| (r.f.clone(), route)))
				.collect();
			let mut replacements = listeners.iter_mut().filter(|r| r.uid == new);
			for (callback, route) in old_routes {
				if let Some(next) = replacements.next() {
					retired_callbacks.push(std::mem::replace(
						&mut *route.lock(),
						next.route.as_ref().and_then(|r| r.lock().clone()),
					));
					retired_callbacks.push(Some(std::mem::replace(&mut next.f, callback)));
					retired_routes.push(next.route.replace(route));
				} else {
					retired_callbacks.push(route.lock().take());
				}
			}
		}
		for fiber in reg.fibers.values_mut() {
			for view in [&mut fiber.target, &mut fiber.committed]
				.into_iter()
				.flatten()
			{
				for (_, _, provider) in view {
					if *provider == old {
						*provider = new;
					}
				}
			}
		}
		let previous = reg.fibers.get_mut(&old).unwrap();
		previous.staged = true;
		previous.provide.clear();
		let candidate = reg.fibers.get_mut(&new).unwrap();
		candidate.isolate = isolate;
		candidate.staged = false;
		// Lua-backed closures can acquire the interpreter lock in Drop.
		drop(reg);
		Ok(())
	}
}
