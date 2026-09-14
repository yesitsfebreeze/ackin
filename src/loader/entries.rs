//! The profile: every installed cartridge as a disabled entry, `init.lua` over
//! them, and the configuration files laid over each entry.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mlua::{LuaSerdeExt, Table};

use crate::error::{Error, Result};
use crate::host::Host;

use super::document::Cartridge;
use super::{document, CartridgeInfo, Entry, SettingsInfo};

pub(crate) fn validate(entry: &Entry) -> Result<()> {
	if entry.id.is_empty() || entry.path.is_empty() {
		return Err(Error::Profile(
			"every entry needs a nonempty `id` and cartridge `path`".into(),
		));
	}
	Ok(())
}

impl Host {
	pub(crate) fn eval<T: serde::de::DeserializeOwned>(&self, path: &Path) -> Result<T> {
		let source = std::fs::read_to_string(path).map_err(|e| Error::file(path, e))?;
		let value: Table = self
			.lua
			.load(&source)
			.set_name(path.to_string_lossy())
			.eval()?;
		Ok(self.lua.from_value(mlua::Value::Table(value))?)
	}

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
				inject: Vec::new(),
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
		let profile = self.profile.join("init.lua");
		let overrides: Vec<Entry> = if profile.is_file() {
			self.eval(&profile)?
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
				return Err(Error::Profile(format!("duplicate entry id `{}`", entry.id)));
			}
		}
		let mut overrides = serde_json::Map::new();
		for file in crate::settings::global_path()
			.into_iter()
			.chain([crate::settings::project_path(&self.profile)])
		{
			if !file.is_file() {
				continue;
			}
			let layer: serde_json::Map<String, serde_json::Value> = self.eval(&file)?;
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
		self.expand(&mut entries)?;
		Ok(entries)
	}

	/// `inject = {"tool.*"}` names every declared event with that prefix another
	/// enabled entry listens to.
	fn expand(self: &Arc<Self>, entries: &mut [Entry]) -> Result<()> {
		let globbed = |entry: &Entry| entry.inject.iter().any(|key| key.ends_with('*'));
		if !entries.iter().any(globbed) {
			return Ok(());
		}
		let listened: Vec<(String, Vec<String>)> = entries
			.iter()
			.filter(|entry| !entry.disabled)
			.map(|entry| {
				let mut exact = entry.clone();
				exact.inject.retain(|key| !key.ends_with('*'));
				let on = self.plan(&exact).map(|plan| plan.on).unwrap_or_default();
				(entry.id.clone(), on)
			})
			.collect();
		for entry in entries.iter_mut().filter(|entry| globbed(entry)) {
			let mut keys: Vec<String> = Vec::new();
			for key in std::mem::take(&mut entry.inject) {
				let Some(prefix) = key.strip_suffix('*') else {
					if !keys.contains(&key) {
						keys.push(key);
					}
					continue;
				};
				if prefix.is_empty() {
					return Err(Error::Profile(format!(
						"entry `{}` may not inject a bare `*`; a glob needs a prefix",
						entry.id
					)));
				}
				let mut matched: Vec<String> = listened
					.iter()
					.filter(|(id, _)| *id != entry.id)
					.flat_map(|(_, on)| on.iter())
					.filter(|key| key.starts_with(prefix))
					.cloned()
					.collect();
				matched.sort();
				matched.dedup();
				for key in matched {
					if !keys.contains(&key) {
						keys.push(key);
					}
				}
			}
			entry.inject = keys;
		}
		Ok(())
	}

	/// Every configurable surface of the profile, read from documents only.
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
				let (needs, events, on, error) = if entry.disabled {
					(Vec::new(), Vec::new(), Vec::new(), None)
				} else {
					match self.plan(&entry) {
						Ok(plan) => (
							plan.needs,
							plan.events.keys().cloned().collect(),
							plan.on,
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
					on,
					grant,
					unread,
					error,
				}
			})
			.collect())
	}
}
