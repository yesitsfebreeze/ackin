//! The descriptor: every installed cartridge as a disabled entry, `init.lua` over
//! them, and the configuration files laid over each entry.

use std::path::PathBuf;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::host::Host;

use super::document::Cartridge;
use super::{document, CartridgeInfo, Entry, SettingsInfo};

pub(crate) fn validate(entry: &Entry) -> Result<()> {
	if entry.id.is_empty() || entry.path.is_empty() {
		return Err(Error::Descriptor(
			"every entry needs a nonempty `id` and cartridge `path`".into(),
		));
	}
	Ok(())
}

impl Host {
	/// One disabled entry per installed top-level cartridge, keyed by its ledger path.
	fn derived(&self) -> Vec<Entry> {
		crate::ledger::Ledger::scan(&self.dir)
			.entries()
			.filter(|e| e.parent().is_none())
			.map(|e| Entry {
				id: e.path.clone(),
				path: e.path.clone(),
				config: serde_json::Value::Null,
				disabled: true,
			})
			.collect()
	}

	/// The ledger's entries, overridden and extended by `init.lua`, with
	/// `~/.cartridge/config.lua` and the project's `config.lua` laid over each
	/// entry's config.
	pub fn entries(self: &Arc<Self>) -> Result<Vec<Entry>> {
		if let Some(solo) = self.solo.lock().clone() {
			return Ok(solo);
		}
		let mut entries = self.derived();
		let descriptor = self.descriptor.join("init.lua");
		let overrides: Vec<Entry> = if descriptor.is_file() {
			crate::lua::evaluate(&descriptor)?
		} else {
			Vec::new()
		};
		let named: Vec<PathBuf> = overrides.iter().map(|e| e.file(&self.dir)).collect();
		entries.retain(|e| !named.contains(&e.file(&self.dir)));
		entries.extend(overrides);
		let mut ids = std::collections::HashSet::new();
		for entry in &entries {
			validate(entry)?;
			if !ids.insert(&entry.id) {
				return Err(Error::Descriptor(format!("duplicate entry id `{}`", entry.id)));
			}
		}
		let mut overrides = serde_json::Map::new();
		for file in crate::settings::global_path()
			.ok()
			.into_iter()
			.chain([crate::settings::project_path(&self.descriptor)])
		{
			if !file.is_file() {
				continue;
			}
			let layer: serde_json::Map<String, serde_json::Value> = crate::lua::evaluate(&file)?;
			for (id, over) in layer {
				match overrides.get_mut(&id) {
					Some(slot) => crate::settings::merge(slot, over),
					None => {
						overrides.insert(id, over);
					}
				}
			}
		}
		for entry in &mut entries {
			if let Some(over) = overrides.remove(&entry.id) {
				crate::settings::merge(&mut entry.config, over);
			}
		}
		Ok(entries)
	}

	/// Every configurable surface of the descriptor, read from documents only.
	pub fn settings(self: &Arc<Self>) -> Result<Vec<SettingsInfo>> {
		Ok(self
			.entries()?
			.into_iter()
			.map(|entry| {
				let manifest = entry.file(&self.dir);
				let (specs, author) = match Cartridge::document(&manifest) {
					Ok(doc) => (doc.settings, doc.config),
					Err(_) => (Default::default(), serde_json::Value::Null),
				};
				let mut settled = crate::settings::defaults(&specs);
				for layer in [author, entry.config.clone()] {
					if !layer.is_null() {
						crate::settings::merge(&mut settled, layer);
					}
				}
				let undeclared = crate::settings::undeclared(&specs, &settled);
				SettingsInfo {
					id: entry.id.clone(),
					path: entry.path.clone(),
					disabled: entry.disabled,
					specs,
					settled,
					undeclared,
				}
			})
			.collect())
	}

	/// Every entry's declarations. Disabled entries are read, never evaluated.
	pub fn manifest(self: &Arc<Self>) -> Result<Vec<CartridgeInfo>> {
		Ok(self
			.entries()?
			.into_iter()
			.map(|entry| {
				let (grant, unread) = match document(&self.dir.join(&entry.path)) {
					Ok(grant) => (Some(grant), None),
					Err(e) => (None, Some(e.to_string())),
				};
				let (needs, events, listen, error) = if entry.disabled {
					(Vec::new(), Vec::new(), Vec::new(), None)
				} else {
					match self.plan(&entry) {
						Ok(plan) => (
							plan.needs,
							plan.events.keys().cloned().collect(),
							plan.listen,
							None,
						),
						Err(e) => {
							let e = unread.is_none().then(|| e.to_string());
							(Vec::new(), Vec::new(), Vec::new(), e)
						}
					}
				};
				CartridgeInfo {
					entry,
					needs,
					events,
					listen,
					grant,
					unread,
					error,
				}
			})
			.collect())
	}
}
