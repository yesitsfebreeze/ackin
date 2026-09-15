use std::sync::Arc;

use crate::error::{Error, Result};
use crate::ledger::{Bound, Installed, Ledger};
use crate::loader::{normalize, Cartridge, Entry, MANIFEST};

use super::{Host, State, Status};

type Contract = (String, String);

impl Host {
	pub async fn settled(&self, timeout: std::time::Duration) -> bool {
		let mut lifecycle = self.lifecycle();
		tokio::time::timeout(timeout, async {
			while self.status().iter().any(|s| s.state == State::Starting) {
				let _ = lifecycle.recv().await;
			}
		})
		.await
		.is_ok()
	}

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
		let mut watcher = None;
		let body = async {
			self.reconcile().await?;
			watcher = Some(self.watch()?);
			self.settled(crate::settings::host().verify_timeout()).await;
			let reply = match self.bail(key, args).await {
				Ok(answer) => answer.unwrap_or(serde_json::Value::Null),
				Err(Error::Unavailable { key, .. }) => {
					return Err(Error::Unavailable {
						key,
						why: stalled(&self.status()).join("; "),
					})
				}
				Err(error) => return Err(error),
			};
			then(reply).await
		};
		let result = tokio::select! {
			result = body => result,
			() = self.stopped() => Err(Error::Stopped),
		};
		if let Some(watcher) = watcher {
			watcher.abort();
		}
		self.stop().await;
		result
	}

	pub async fn run(
		self: &Arc<Self>,
		key: &str,
		args: serde_json::Value,
	) -> Result<serde_json::Value> {
		self.run_then(key, args, |reply| async { Ok(reply) }).await
	}

	pub async fn verify(self: &Arc<Self>) -> Result<(usize, Vec<String>)> {
		let contracts = self.contracts()?;
		self.run_contracts(contracts).await
	}

	pub async fn verify_one(self: &Arc<Self>, target: &str) -> Result<(usize, Vec<String>)> {
		let (entries, contracts) = self.solo(target)?;
		*self.solo.lock() = Some(entries);
		let result = self.run_contracts(contracts).await;
		*self.solo.lock() = None;
		result
	}

	fn solo(&self, target: &str) -> Result<(Vec<Entry>, Vec<Contract>)> {
		let ledger = Ledger::scan(&self.dir);
		let at = match ledger.get(target) {
			Some(entry) => entry.path.clone(),
			None => {
				let asked = normalize(&self.dir.join(target));
				ledger
					.entries()
					.find(|e| normalize(&e.dir) == asked)
					.ok_or_else(|| {
						Error::Descriptor(format!(
							"`{target}` is not a cartridge under {}",
							self.dir.display()
						))
					})?
					.path
					.clone()
			}
		};
		let mut chosen: Vec<Entry> = Vec::new();
		let mut frontier = vec![at.clone()];
		while let Some(path) = frontier.pop() {
			let installed = ledger.get(&path).expect("ledger entry");
			for key in &installed.needs {
				match ledger.resolve(&installed.path, key) {
					Bound::None => {
						return Err(Error::Descriptor(format!(
							"{path}: need `{key}` binds to nothing in the tree"
						)))
					}
					Bound::Clashed(offered) => {
						let offered: Vec<&str> = offered.iter().map(|p| p.path.as_str()).collect();
						return Err(Error::Descriptor(format!(
							"{path}: need `{key}` is ambiguous ({})",
							offered.join(", ")
						)));
					}
					Bound::One(provider) => {
						if chosen.iter().all(|entry| entry.id != provider.path) {
							chosen.push(solo_entry(provider));
							frontier.push(provider.path.clone());
						}
					}
				}
			}
			if !chosen.iter().any(|entry| entry.id == path) {
				chosen.insert(0, solo_entry(installed));
			}
		}
		let (cartridge, _) = Cartridge::read(&self.dir.join(&at).join(MANIFEST))
			.map_err(|e| Error::Descriptor(format!("{at}: {e}")))?;
		let contracts = cartridge
			.contracts
			.into_iter()
			.map(|key| (at.clone(), key))
			.collect();
		Ok((chosen, contracts))
	}

	fn contracts(self: &Arc<Self>) -> Result<Vec<Contract>> {
		let mut out = Vec::new();
		for entry in self.entries()?.iter().filter(|e| !e.disabled) {
			let manifest = entry.file(&self.dir);
			if !manifest.ends_with(MANIFEST) {
				continue;
			}
			let (cartridge, _) = Cartridge::read(&manifest)?;
			out.extend(
				cartridge
					.contracts
					.into_iter()
					.map(|key| (entry.id.clone(), key)),
			);
		}
		Ok(out)
	}

	async fn run_contracts(
		self: &Arc<Self>,
		contracts: Vec<Contract>,
	) -> Result<(usize, Vec<String>)> {
		let body = async {
			let mut failures = Vec::new();
			self.reconcile().await?;
			if !self.settled(crate::settings::host().verify_timeout()).await {
				failures.push(format!(
					"descriptor did not settle: {}",
					stalled(&self.status()).join("; ")
				));
			}
			for (id, key) in &contracts {
				match self.send_to(id, key, serde_json::Value::Null).await {
					Ok(serde_json::Value::Bool(false)) => {
						failures.push(format!("{id} contract `{key}` returned false"))
					}
					Ok(_) => {}
					Err(e) => failures.push(format!("{id} contract `{key}`: {e}")),
				}
			}
			Ok(failures)
		};
		let result = tokio::select! {
			result = body => result,
			() = self.stopped() => Err(Error::Stopped),
		};
		self.stop().await;
		result.map(|failures| (contracts.len(), failures))
	}
}

fn solo_entry(installed: &Installed) -> Entry {
	Entry {
		id: installed.path.clone(),
		path: installed.path.clone(),
		config: serde_json::Value::Null,
		disabled: false,
	}
}

pub fn stalled(status: &[Status]) -> Vec<String> {
	status
		.iter()
		.filter(|s| !matches!(s.state, State::Active | State::Disabled))
		.map(|s| {
			let mut line = format!(
				"{} {}",
				s.id,
				serde_json::to_value(s.state)
					.unwrap_or_default()
					.as_str()
					.unwrap_or("")
			);
			if let Some(error) = &s.error {
				line.push_str(&format!(" ({error})"));
			}
			if !s.waiting.is_empty() {
				line.push_str(&format!(" waiting for {}", s.waiting.join(", ")));
			}
			line
		})
		.collect()
}
