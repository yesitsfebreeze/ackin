//! The long-lived host: the profile reconciled against what is loaded, and
//! the generation switch that replaces an entry or a composed node in place
//! while its services stay stable.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::lua::Host;
use crate::runtime::{Component, FiberHandle};

use super::{normalize, Entry, Loaded, Source};

impl Host {
	/// The loaded slot for `id`, under the lock.
	pub(super) fn slot<R>(&self, id: &str, f: impl FnOnce(&mut Loaded) -> R) -> Option<R> {
		self.loaded
			.lock()
			.iter_mut()
			.find(|l| l.entry.id == id)
			.map(f)
	}

	pub(super) fn spawn(&self, entry: &Entry, component: Component) -> FiberHandle {
		entry
			.isolate
			.iter()
			.fold(self.rt.ctx(), |ctx, key| ctx.isolate(key))
			.cartridge(component)
	}

	pub(super) async fn instantiate(self: &Arc<Self>, entry: Entry) -> Loaded {
		let sources = vec![Source::new(entry.file(&self.dir))];
		let mut loaded = Loaded {
			entry,
			fiber: None,
			error: None,
			sources,
			reload: crate::reload::Reload::default(),
		};
		if loaded.entry.disabled {
			return loaded;
		}
		match self.load_entry_async(loaded.entry.clone()).await {
			Ok((mut component, files)) => {
				component.reload = loaded.reload.clone();
				loaded.sources = files.into_iter().map(Source::new).collect();
				loaded.fiber = Some(self.spawn(&loaded.entry, component));
			}
			Err(e) => {
				loaded.error = Some(e.to_string());
				self.report(&loaded.entry.id, e);
			}
		}
		loaded
	}

	pub async fn reconcile(self: &Arc<Self>) -> Result<()> {
		self.start_reconcile()
			.await
			.map_err(|e| Error::Profile(format!("reconcile: {e}")))?
	}

	pub(crate) fn start_reconcile(self: &Arc<Self>) -> tokio::task::JoinHandle<Result<()>> {
		// Start at acceptance, not when a reply waiter is first polled.
		// Disconnection never cancels a transaction between begin and finish.
		let host = self.clone();
		tokio::spawn(async move { host.reconcile_inner().await })
	}

	pub(super) async fn reconcile_inner(self: &Arc<Self>) -> Result<()> {
		let _reload = self.reload_lock.lock().await;
		let wanted = self.entries()?;
		let removed: Vec<_> = {
			let mut loaded = self.loaded.lock();
			let mut removed = Vec::new();
			let mut index = 0;
			while index < loaded.len() {
				if wanted.iter().any(|e| {
					e.id == loaded[index].entry.id && !e.disabled && !loaded[index].entry.disabled
				}) {
					index += 1;
				} else {
					removed.push(loaded.remove(index));
				}
			}
			removed
		};
		for old in removed {
			if let Some(fiber) = old.fiber {
				fiber.dispose().await;
			}
			old.reload.finish();
		}
		for entry in wanted {
			let previous = self.slot(&entry.id, |l| l.entry.clone());
			match previous {
				Some(old) if old != entry => {
					let _ = self.replace_entry(entry).await;
				}
				Some(_) => {}
				None => {
					let loaded = self.instantiate(entry).await;
					self.loaded.lock().push(loaded);
				}
			}
		}
		Ok(())
	}

	pub async fn replace(self: &Arc<Self>, path: &Path) {
		self.replace_changed(&[normalize(path)]).await;
	}

	pub(super) async fn replace_changed(self: &Arc<Self>, paths: &[PathBuf]) {
		let _reload = self.reload_lock.lock().await;
		let entries: Vec<Entry> = self
			.loaded
			.lock()
			.iter()
			.filter(|l| {
				!l.entry.disabled
					&& l.sources.iter().any(|source| {
						(paths.contains(&source.path) && source.changed())
							|| (source.path.extension().is_some_and(|e| {
								e == "tsx" || e == "ts" || e == "js" || e == "jsx"
							}) && source.path.parent().is_some_and(|parent| {
								paths.iter().any(|path| path.starts_with(parent))
							}))
					})
			})
			.map(|l| l.entry.clone())
			.collect();
		for entry in entries {
			let _ = self.replace_entry(entry).await;
		}
	}

	pub(super) fn report_entry(&self, id: &str, error: impl std::fmt::Display) {
		let error = error.to_string();
		self.slot(id, |slot| slot.error = Some(error.clone()));
		self.report(id, error);
	}

	pub(super) async fn replace_entry(
		self: &Arc<Self>,
		entry: Entry,
	) -> Result<crate::runtime::Uid> {
		let id = entry.id.clone();
		let result = self.replace_entry_inner(entry).await;
		if let Err(error) = &result {
			self.report_entry(&id, error);
		}
		result
	}

	pub(super) async fn replace_entry_inner(
		self: &Arc<Self>,
		entry: Entry,
	) -> Result<crate::runtime::Uid> {
		if self
			.slot(&entry.id, |l| l.entry.isolate != entry.isolate)
			.unwrap_or(false)
		{
			return Err(Error::Reload(
				"changing isolation requires removing and recomposing the entry".into(),
			));
		}
		let (mut component, files) = self.load_entry_async(entry.clone()).await?;
		let sources = files.into_iter().map(Source::new).collect();
		let (old, reload) = self
			.slot(&entry.id, |l| (l.fiber.clone(), l.reload.clone()))
			.ok_or_else(|| {
				Error::Reload(format!("profile entry `{}` is no longer loaded", entry.id))
			})?;
		if let Some(old) = &old {
			old.settled().await;
		}
		if old
			.as_ref()
			.is_none_or(|old| old.state() != Some(crate::runtime::State::Active))
		{
			// Initial failures have no active publication to switch. Keep the
			// existing recovery path, recording its actual newly spawned handle.
			if let Some(old) = old {
				old.dispose().await;
			}
			component.reload = reload;
			let fiber = self.spawn(&entry, component);
			let uid = fiber.uid();
			self.slot(&entry.id, |l| {
				l.entry = entry.clone();
				l.fiber = Some(fiber);
				l.sources = sources;
				l.error = None;
			});
			return Ok(uid);
		}
		let composing = entry
			.isolate
			.iter()
			.fold(self.rt.ctx(), |ctx, key| ctx.isolate(key));
		self.replace_active(old.unwrap(), component, composing, reload, |candidate| {
			// The caller holds reload_lock. This synchronous bookkeeping is
			// published before the shared transaction unlocks or disposes old.
			self.slot(&entry.id, |l| {
				l.entry = entry.clone();
				l.fiber = Some(candidate);
				l.sources = sources;
				l.error = None;
			});
		})
		.await
	}

	pub(crate) fn request_reload(self: &Arc<Self>, uid: crate::runtime::Uid) {
		let entry = self
			.loaded
			.lock()
			.iter()
			.find(|l| l.fiber.as_ref().is_some_and(|f| f.uid() == uid) && !l.reload.pending())
			.map(|l| l.entry.id.clone());
		if let Some(entry) = entry {
			let host = self.clone();
			tokio::spawn(async move {
				let _reload = host.reload_lock.lock().await;
				if let Some(entry) = host.slot(&entry, |l| l.entry.clone()) {
					let _ = host.replace_entry(entry).await;
				}
			});
			return;
		}
		// A node composed inside another cartridge has no profile slot; the
		// same subtree transaction answers its ask.
		let host = self.clone();
		tokio::spawn(async move {
			if let Err(e) = host.replace_node(uid).await {
				host.report(&uid.to_string(), e);
			}
		});
	}

	/// The Lua surface addresses the node, not the generation: a handle
	/// captured before a swap names a uid the swap retired, so the ask re-finds
	/// the live generation by what persists across one — the node's shared
	/// [`Reload`] transaction — and fires the same ask on it. An ask whose
	/// transaction no live fiber carries is refused on the report channel, as
	/// the raw uid ask's refusals are.
	pub(crate) fn request_reload_for(
		self: &Arc<Self>,
		uid: crate::runtime::Uid,
		reload: crate::reload::Reload,
	) {
		let live = {
			let reg = self.rt.reg.lock();
			if reg.fibers.contains_key(&uid) {
				Some(uid)
			} else {
				// A candidate is staged until the switch publishes it and the
				// disposed generation is staged until it goes, so the live
				// generation is the unstaged one.
				reg.fibers
					.iter()
					.filter(|(_, f)| !f.retired && !f.staged && f.reload.same(&reload))
					.map(|(uid, _)| *uid)
					.max()
			}
		};
		match live {
			Some(live) => self.request_reload(live),
			None => self.report(
				&uid.to_string(),
				"no live generation carries this node's reload transaction",
			),
		}
	}

	/// Replace one node of the running tree, addressed by its uid. A profile
	/// entry goes through [`Host::replace_entry`]; a node composed inside
	/// another cartridge resolves its rebuild before the shared transaction.
	/// Success names the current generation; candidate refusal is an error.
	/// Recovery of an initially failed profile entry returns its newly spawned
	/// handle, which retains the ordinary asynchronous readiness lifecycle.
	pub async fn replace_node(
		self: &Arc<Self>,
		uid: crate::runtime::Uid,
	) -> Result<crate::runtime::Uid> {
		let _reload = self.reload_lock.lock().await;
		let entry = self
			.loaded
			.lock()
			.iter()
			.find(|l| l.fiber.as_ref().is_some_and(|f| f.uid() == uid))
			.map(|l| l.entry.clone());
		if let Some(entry) = entry {
			return self.replace_entry(entry).await;
		}
		self.swap_node(uid).await
	}

	/// The subtree transaction: rebuild the node's component from its source,
	/// drain its service calls, publish only an active candidate presenting the
	/// same provided keys, and dispose the old generation. A rejected candidate
	/// unfreezes the old bank and the live generation stays.
	pub(super) async fn swap_node(
		self: &Arc<Self>,
		uid: crate::runtime::Uid,
	) -> Result<crate::runtime::Uid> {
		let (rebuild, reload, isolate, intercept, parent) = {
			let reg = self.rt.reg.lock();
			let Some(f) = reg.fibers.get(&uid) else {
				return Err(Error::Reload(format!("no running node carries uid {uid}")));
			};
			let Some(rebuild) = f.rebuild.clone() else {
				return Err(Error::Reload(
					"this node was not composed to be replaceable".into(),
				));
			};
			(
				rebuild,
				f.reload.clone(),
				f.isolate.clone(),
				f.intercept.clone(),
				f.parent.unwrap_or(crate::runtime::ROOT),
			)
		};
		let old = self.rt.handle(uid);
		old.settled().await;
		if old.state() != Some(crate::runtime::State::Active) {
			return Err(Error::Reload(
				"the live generation is no longer active".into(),
			));
		}
		let composing = self.rt.ctx_under(parent, isolate, intercept);
		let mut component = rebuild(composing.clone())?;
		component.resident = true;
		component.rebuild = Some(rebuild);
		self.replace_active(old, component, composing, reload, |_| {})
			.await
	}

	/// One publication transaction for an active generation at any depth.
	/// The caller supplies its resolved context and synchronous bookkeeping;
	/// process preparation, drain, switch and rejection cleanup stay here.
	pub(super) async fn replace_active(
		self: &Arc<Self>,
		old: FiberHandle,
		mut component: Component,
		composing: crate::runtime::Ctx,
		reload: crate::reload::Reload,
		publish: impl FnOnce(FiberHandle) + Send,
	) -> Result<crate::runtime::Uid> {
		old.settled().await;
		if old.state() != Some(crate::runtime::State::Active) {
			return Err(Error::Reload(
				"the live generation is no longer active".into(),
			));
		}
		if !reload.begin() {
			return Err(Error::Reload(
				"a reload is already in flight for this node".into(),
			));
		}

		let mut values = {
			let reg = self.rt.reg.lock();
			reg.provided(old.uid())
				.into_iter()
				.filter_map(|(_, realm)| reg.value(realm))
				.collect::<Vec<_>>()
		};
		// A process may provide several keys, but its lifecycle hook runs once.
		let mut prepared = std::collections::HashSet::new();
		values.retain(|value| {
			value
				.downcast_ref::<crate::service::Service>()
				.and_then(|service| service.process_id())
				.is_some_and(|id| prepared.insert(id))
		});
		for value in &values {
			if let Some(service) = value.downcast_ref::<crate::service::Service>() {
				if let Err(e) = service.prepare().await {
					for value in &values {
						if let Some(service) = value.downcast_ref::<crate::service::Service>() {
							service.cancel().await;
						}
					}
					reload.finish();
					return Err(e);
				}
			}
		}
		let _calls = reload.gate().write_owned().await;
		component.reload = reload.clone();
		component.staged = true;
		// The candidate provides into private realms, as a profile entry's
		// candidate does; the switch publishes it into the live realms.
		let keys = component.provide.clone();
		let candidate = keys
			.iter()
			.fold(composing.clone(), |ctx, key| ctx.isolate(key))
			.cartridge(component);
		candidate.settled().await;
		let result = if let Some(error) = candidate.error() {
			Err(Error::Reload(error))
		} else {
			self.rt
				.switch(old.uid(), candidate.uid())
				.map_err(Error::Runtime)
		};
		match result {
			Ok(()) => {
				publish(candidate.clone());
				reload.finish();
				drop(_calls);
				old.dispose().await;
				Ok(candidate.uid())
			}
			Err(error) => {
				candidate.dispose().await;
				for value in &values {
					if let Some(service) = value.downcast_ref::<crate::service::Service>() {
						service.cancel().await;
					}
				}
				reload.finish();
				Err(error)
			}
		}
	}

	pub fn fiber_of(&self, id: &str) -> Option<FiberHandle> {
		self.slot(id, |l| l.fiber.clone()).flatten()
	}
}
