//! One run of the profile in this process: load it, call one service or
//! every declared contract, dispose everything on the way out.

use std::sync::Arc;

use crate::error::{Error, Result};
use crate::ledger::{Bound, Installed, Ledger};
use crate::lua::Host;
use crate::runtime::FiberHandle;

use super::document::{classify, Cartridge, MANIFEST};
use super::{normalize, Entry};

/// The entries a one-cartridge verify loads, and the contracts it runs.
type Solo = (Vec<Entry>, Vec<(String, &'static str, String)>);

impl Host {
	/// Load a profile, invoke one provided service, then dispose every instance.
	pub async fn run(
		self: &Arc<Self>,
		key: &str,
		args: serde_json::Value,
	) -> Result<serde_json::Value> {
		self.run_mode(key, args, |v| async { Ok(v) }).await
	}

	/// Load the profile and call every contract its manifests declare, then
	/// dispose every instance. Returns how many ran and one line per failure,
	/// each naming the cartridge, the obligation and the key. A contract fails
	/// when its call errors or returns `false`; a cartridge that declares none
	/// is never called.
	pub async fn verify(self: &Arc<Self>) -> Result<(usize, Vec<String>)> {
		let contracts = self.contracts()?;
		self.run_contracts(contracts).await
	}

	/// Verify **one** cartridge in isolation: the ledger resolves its
	/// dependencies and the document states its contract, and the profile is
	/// never read. The host is given the target plus only the cartridges its
	/// needs bind to, so the thing just written is tested against its own
	/// contract without a system assembled around it. Returns the same pair
	/// [`Host::verify`] does, over the target's contracts alone.
	pub async fn verify_one(self: &Arc<Self>, target: &str) -> Result<(usize, Vec<String>)> {
		let (entries, contracts) = self.solo(target)?;
		*self.solo.lock() = Some(entries);
		let result = self.run_contracts(contracts).await;
		*self.solo.lock() = None;
		result
	}

	/// The entries a one-cartridge verify loads, and the contracts it runs —
	/// the target's alone. Providers pulled in for the target's needs are
	/// enabled and nothing else: they are loaded to be reached, not to be
	/// graded, so their own contracts are not this run's business.
	pub(super) fn solo(self: &Arc<Self>, target: &str) -> Result<Solo> {
		let ledger = Ledger::scan(&self.dir);
		let at = match ledger.get(target) {
			Some(e) => e.path.clone(),
			None => {
				// A path that is not a ledger path may still name the folder the
				// cartridge sits in, so the ask is answered either way the writer
				// knew it.
				let asked = normalize(&self.dir.join(target));
				let found = ledger
					.entries()
					.find(|e| normalize(&e.dir) == asked)
					.ok_or_else(|| {
						Error::Profile(format!(
							"`{target}` is not a cartridge under {}",
							self.dir.display()
						))
					})?;
				found.path.clone()
			}
		};
		// The closure of the target's needs, resolved the way the tree resolves
		// them: the asker's subtree first, then outward. A provider is loaded
		// with its own needs answered too, so the walk reaches the whole chain
		// the target would draw on when installed. Nothing outside the closure
		// is touched, which is what makes the run an isolation and not a
		// profile under another name.
		let mut chosen: Vec<Entry> = Vec::new();
		let mut frontier = vec![at.clone()];
		while let Some(path) = frontier.pop() {
			let e = ledger.get(&path).expect("ledger entry");
			for key in &e.needs {
				match ledger.resolve(&e.path, key) {
					Bound::None => {
						return Err(Error::Profile(format!(
							"{path}: need `{key}` binds to nothing in the tree"
						)))
					}
					Bound::Clashed(offered) => {
						return Err(Error::Profile(format!(
							"{path}: need `{key}` is ambiguous ({})",
							offered
								.iter()
								.map(|p| p.path.as_str())
								.collect::<Vec<_>>()
								.join(", ")
						)))
					}
					Bound::One(provider) => {
						if chosen.iter().all(|entry| entry.id != provider.path) {
							chosen.push(Self::solo_entry(provider));
							frontier.push(provider.path.clone());
						}
					}
				}
			}
			if !chosen.iter().any(|entry| entry.id == path) {
				chosen.insert(0, Self::solo_entry(e));
			}
		}
		// The document states the contract, read where the entry must already
		// be in place: a document that cannot be read, or an entry missing, is
		// what verification exists to catch, so neither is a silent pass.
		let (cartridge, _) = Cartridge::read(&self.dir.join(&at).join(MANIFEST))
			.map_err(|e| Error::Profile(format!("{at}: {e}")))?;
		let contracts = [
			("selftest", cartridge.selftest),
			("integration", cartridge.integration),
		]
		.into_iter()
		.filter_map(|(obligation, key)| key.map(|key| (at.clone(), obligation, key)))
		.collect();
		Ok((chosen, contracts))
	}

	/// One ledger entry as a profile entry, by its path from the root and
	/// enabled: a solo run grades its cartridges by loading them, which is the
	/// one thing the ledger's *available, not started* default never does.
	pub(super) fn solo_entry(installed: &Installed) -> Entry {
		Entry {
			id: installed.path.clone(),
			path: installed.path.clone(),
			config: serde_json::Value::Null,
			disabled: false,
			isolate: Vec::new(),
			inject: Vec::new(),
		}
	}

	pub(super) async fn run_contracts(
		self: &Arc<Self>,
		contracts: Vec<(String, &'static str, String)>,
	) -> Result<(usize, Vec<String>)> {
		let mut failures = Vec::new();
		let mut lifecycle = self.rt.lifecycle();
		self.reconcile().await?;
		let settled = tokio::time::timeout(crate::settings::host().verify_timeout(), async {
			while self
				.rt
				.fibers()
				.iter()
				.any(|f| f.state == crate::runtime::State::Loading)
			{
				let _ = lifecycle.recv().await;
			}
		})
		.await;
		if settled.is_err() {
			failures.push(format!(
				"profile did not settle: {}",
				self.stalled(&self.rt.fibers()).join("; ")
			));
		}
		for (id, obligation, key) in &contracts {
			match self.call(key, serde_json::Value::Null).await {
				Ok(serde_json::Value::Bool(false)) => {
					failures.push(format!("{id} {obligation} `{key}` returned false"))
				}
				Ok(_) => {}
				Err(e) => failures.push(format!("{id} {obligation} `{key}`: {e}")),
			}
		}
		let fibers: Vec<_> = self
			.loaded
			.lock()
			.drain(..)
			.filter_map(|l| l.fiber)
			.collect();
		futures::future::join_all(fibers.iter().map(FiberHandle::dispose)).await;
		Ok((contracts.len(), failures))
	}

	/// `(entry id, obligation, key)` for every contract the profile's enabled
	/// folder cartridges declare. Entries naming a bare Lua file have no
	/// manifest and so declare nothing.
	pub(super) fn contracts(self: &Arc<Self>) -> Result<Vec<(String, &'static str, String)>> {
		let mut out = Vec::new();
		for entry in self.entries()?.iter().filter(|e| !e.disabled) {
			let manifest = normalize(&classify(&self.dir.join(&entry.path)));
			if !manifest.ends_with(MANIFEST) {
				continue;
			}
			// `Cartridge::read` and not `resolve`: this walks the profile for the
			// obligations a document declares, and `passed_on` is deliberately not
			// run here. A re-export is checked once, where the cartridge enters the
			// graph through `resolve`; re-checking it would make a contract listing
			// fail on a neighbour's bad `export`, which is not this call's business.
			let (cartridge, _) = Cartridge::read(&manifest)?;
			for (obligation, key) in [
				("selftest", cartridge.selftest),
				("integration", cartridge.integration),
			] {
				if let Some(key) = key {
					out.push((entry.id.clone(), obligation, key));
				}
			}
		}
		Ok(out)
	}

	/// Like `run`, but `then` receives the reply while every instance is still
	/// alive; instances are disposed once it completes.
	pub async fn run_then<F, Fut>(
		self: &Arc<Self>,
		key: &str,
		args: serde_json::Value,
		then: F,
	) -> Result<serde_json::Value>
	where
		F: FnOnce(serde_json::Value) -> Fut,
		Fut: std::future::Future<Output = Result<serde_json::Value>>,
	{
		self.run_mode(key, args, then).await
	}

	/// One line per entry that is not serving: load failures, then every
	/// fiber that is not active with its state, error and unmet injections.
	/// The reader fixes the named entry instead of guessing the chain.
	pub(super) fn stalled(&self, fibers: &[crate::runtime::FiberInfo]) -> Vec<String> {
		let provided: std::collections::HashSet<&str> = fibers
			.iter()
			.filter(|f| f.state == crate::runtime::State::Active)
			.flat_map(|f| f.provide.iter().map(String::as_str))
			.collect();
		let mut out: Vec<String> = self
			.loaded
			.lock()
			.iter()
			.filter(|l| !l.entry.disabled && l.fiber.is_none())
			.map(|l| format!("{} failed to load", l.entry.id))
			.collect();
		for f in fibers
			.iter()
			.filter(|f| f.state != crate::runtime::State::Active)
		{
			let missing: Vec<&str> = f
				.inject
				.iter()
				.map(String::as_str)
				.filter(|k| !provided.contains(k))
				.collect();
			let mut line = format!("{} {}", f.name, format!("{:?}", f.state).to_lowercase());
			if let Some(e) = &f.error {
				line.push_str(&format!(" ({e})"));
			}
			if !missing.is_empty() {
				line.push_str(&format!(" waiting for {}", missing.join(", ")));
			}
			out.push(line);
		}
		out
	}

	pub(super) async fn run_mode<F, Fut>(
		self: &Arc<Self>,
		key: &str,
		args: serde_json::Value,
		then: F,
	) -> Result<serde_json::Value>
	where
		F: FnOnce(serde_json::Value) -> Fut,
		Fut: std::future::Future<Output = Result<serde_json::Value>>,
	{
		let mut watcher = None;
		let result = async {
			let mut lifecycle = self.rt.lifecycle();
			self.reconcile().await?;
			// Hot reload is the host's normal behaviour: sources are watched
			// and changed cartridges replaced in place for every foreground run.
			watcher = Some(self.watch_mode()?);
			tokio::time::timeout(crate::settings::host().verify_timeout(), async {
				loop {
					let fibers = self.rt.fibers();
					if fibers.iter().any(|f| {
						f.state == crate::runtime::State::Active
							&& f.provide.iter().any(|p| p == key)
					}) {
						return Ok(());
					}
					if !fibers
						.iter()
						.any(|f| f.state == crate::runtime::State::Loading)
					{
						return Err(Error::Unavailable {
							key: key.to_owned(),
							why: self.stalled(&fibers).join("; "),
						});
					}
					let _ = lifecycle.recv().await;
				}
			})
			.await
			.map_err(|_| Error::Unavailable {
				key: key.to_owned(),
				why: "did not become active".into(),
			})??;
			let reply = self.call(key, args).await?;
			then(reply).await
		}
		.await;
		if let Some(watcher) = watcher {
			watcher.abort();
			let _ = watcher.await;
		}
		let fibers: Vec<_> = self
			.loaded
			.lock()
			.drain(..)
			.filter_map(|l| l.fiber)
			.collect();
		futures::future::join_all(fibers.iter().map(FiberHandle::dispose)).await;
		result
	}
}
