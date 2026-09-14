//! The profile: the ledger as the manifest of record, `init.lua` as the
//! override on it, the configuration files laid over each entry, and every
//! entry loaded as a component.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mlua::{LuaSerdeExt, Table};

use crate::error::{Error, Result};
use crate::lua::Host;
use crate::runtime::Component;

use super::document::Cartridge;
use super::{document, CartridgeInfo, Entry, SettingsInfo};

pub(super) fn validate(entry: &Entry) -> Result<()> {
	if entry.id.is_empty() || entry.path.is_empty() {
		return Err(Error::Profile(
			"every entry needs a nonempty `id` and cartridge `path`".into(),
		));
	}
	Ok(())
}

impl Host {
	pub(super) fn eval<T: serde::de::DeserializeOwned>(&self, path: &Path) -> Result<T> {
		let source = std::fs::read_to_string(path).map_err(|e| Error::file(path, e))?;
		let value: Table = self
			.lua
			.load(&source)
			.set_name(path.to_string_lossy())
			.eval()?;
		Ok(self.lua.from_value(mlua::Value::Table(value))?)
	}

	/// One entry per installed cartridge, straight off the ledger, before any
	/// profile is read. The id is the ledger path, because the path is the
	/// identity: a bare name is not unique across a tree of subtrees and was
	/// never meant to be.
	pub(super) fn derived(&self) -> Vec<Entry> {
		crate::ledger::Ledger::scan(&self.dir)
			.entries()
			.filter(|e| e.parent().is_none())
			.map(|e| Entry {
				id: e.path.clone(),
				path: e.path.clone(),
				config: serde_json::Value::Null,
				// A newly installed cartridge is *available, not started* — it
				// is listed and can be asked for, and nothing
				// of it runs until something needs it. `disabled` is the line
				// between the two: the entry exists, its document is read and
				// its declarations carry, but nothing is evaluated and nothing
				// spawns. Stopping it does not mean taking it away, which is
				// why an installed cartridge stays put when `init.lua` does not
				// name it — the ledger is the record, not the launch order.
				disabled: true,
				isolate: Vec::new(),
				inject: Vec::new(),
			})
			.collect()
	}

	/// The ledger is the manifest of record and the profile is an override on
	/// top of it: every cartridge installed under the root is an entry before
	/// `init.lua` is read at all, so installing one is putting it where the
	/// ledger looks and nothing edits a list. `init.lua`, if present, then
	/// overrides — an entry whose `path` names a ledger entry replaces the
	/// derived one under whatever `id` the profile gives it, and an entry whose
	/// `path` names something else (a bare `.lua` file, a folder outside the
	/// root) is added, which is how programmatic composition survives.
	/// `config.lua` maps entry id to config fields laid over the entry's own.
	pub(super) fn entries(self: &Arc<Self>) -> Result<Vec<Entry>> {
		// A solo run hands over its own entry set and the profile is never
		// read: verifying one cartridge in isolation is the one ask where the
		// manifest of record is the caller's choice and not `init.lua`'s.
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
		// An override is matched against the *derived* entries only, and by the
		// file each one resolves to rather than by the string written: a profile
		// may name the folder or the `cartridge.json` inside it, and `Entry::file`
		// is the one rule that already makes those the same cartridge. The
		// derived count is frozen before the `retain`, so two profile entries
		// naming one path are two instances of it and do not eat each other.
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
		// Two configuration files, laid one over the other: the machine's, then
		// this project's. A preference that follows the person lives in
		// `~/.cartridge/config.lua`; what this checkout needs lives beside the
		// profile that composes it, and wins where both speak. Both are keyed by
		// entry id, and both merge field by field, so naming one key in a
		// project file keeps every other key the global file settled.
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
			let Some(over) = overrides.remove(&entry.id) else {
				continue;
			};
			crate::settings::merge(&mut entry.config, over);
		}
		self.expand(&mut entries)?;
		Ok(entries)
	}

	/// `inject = {"tool.*"}` in a profile entry means every key with that prefix
	/// the profile's other enabled entries provide. The profile already names the
	/// cartridges; restating their keys is a second list to keep in step, and the
	/// keys are known here, before any fiber starts, from each cartridge's own
	/// declarations. An entry never injects what it provides itself.
	pub(super) fn expand(self: &Arc<Self>, entries: &mut [Entry]) -> Result<()> {
		let globbed = |entry: &Entry| entry.inject.iter().any(|key| key.ends_with('*'));
		if !entries.iter().any(globbed) {
			return Ok(());
		}
		// A glob is not a key a component may declare, so an entry's declarations
		// are read without it: an entry that globs a surface can provide part of
		// that surface itself, and asking with the glob still in place would fail
		// and hide what it provides from every other glob.
		let provided: Vec<(String, Vec<String>)> = entries
			.iter()
			.filter(|entry| !entry.disabled)
			.map(|entry| {
				let mut exact = entry.clone();
				exact.inject.retain(|key| !key.ends_with('*'));
				let provide = self
					.component_of(&exact)
					.map(|component| component.provide)
					.unwrap_or_default();
				(entry.id.clone(), provide)
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
				let mut matched: Vec<String> = provided
					.iter()
					.filter(|(id, _)| *id != entry.id)
					.flat_map(|(_, provide)| provide.iter())
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

	pub fn component_of(self: &Arc<Self>, entry: &Entry) -> Result<Component> {
		self.load_entry(entry).map(|(component, _)| component)
	}

	pub(super) fn load_entry(self: &Arc<Self>, entry: &Entry) -> Result<(Component, Vec<PathBuf>)> {
		validate(entry)?;
		let (mut component, files) = self.load_component(
			&self.dir.join(&entry.path),
			entry.config.clone(),
			&entry.inject,
		)?;
		component.resident = true;
		Ok((component, files))
	}

	pub(super) async fn load_entry_async(
		self: &Arc<Self>,
		entry: Entry,
	) -> Result<(Component, Vec<PathBuf>)> {
		let host = self.clone();
		tokio::task::spawn_blocking(move || host.load_entry(&entry))
			.await
			.map_err(|e| Error::Profile(format!("loading a profile entry: {e}")))?
	}

	/// Every configurable surface this profile has, without running any of it.
	///
	/// The listing `cartridge settings` prints. It reads documents and the two
	/// configuration files and stops there — no Lua entry is evaluated and no
	/// `hello` process is spawned — because what a cartridge *can be told* is a
	/// property of its declaration, and a cartridge that will not start still
	/// has settings worth reading.
	pub fn settings(self: &Arc<Self>) -> Result<Vec<SettingsInfo>> {
		Ok(self
			.entries()?
			.into_iter()
			.map(|entry| {
				let manifest = entry.file(&self.dir);
				let (specs, author) = match Cartridge::document(&manifest) {
					Ok(doc) => (doc.settings, doc.config),
					// A bare `.lua` entry has no document and declares nothing; so
					// does a document that will not read. Either way the configured
					// values are still shown, under `undeclared`.
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

	/// Effective declarations. Enabled process wrappers run hello, never apply.
	/// Disabled entries are not evaluated and never spawn even a hello process.
	pub fn manifest(self: &Arc<Self>) -> Result<Vec<CartridgeInfo>> {
		Ok(self
			.entries()?
			.into_iter()
			.map(|entry| {
				// The document is data, so the request is readable whether or not the
				// entry is evaluated — a disabled cartridge still declares what it asks for.
				// A document that cannot be read declares nothing *knowable*, which is
				// not the same as declaring nothing: the error is carried, and the
				// grant is absent rather than empty, so the two never read alike.
				// Absent, never empty: an empty grant is the tightest policy, so a
				// document that would not read must not be able to produce one. A
				// disabled cartridge has still declared, so this is read either way.
				let (export, grant, unread) = match document(&self.dir.join(&entry.path)) {
					Ok((export, grant)) => (export, Some(grant), None),
					Err(e) => (Vec::new(), None, Some(e.to_string())),
				};
				let (inject, provide, error) = if entry.disabled {
					(Vec::new(), Vec::new(), None)
				} else {
					match self.component_of(&entry) {
						Ok(c) => (c.inject, c.provide, None),
						// The document's own failure is already carried by `unread`, so
						// it is not repeated here: one fact, printed once.
						Err(e) => {
							let e = (unread.is_none()).then(|| e.to_string());
							(Vec::new(), Vec::new(), e)
						}
					}
				};
				CartridgeInfo {
					entry,
					inject,
					provide,
					export,
					grant,
					unread,
					error,
				}
			})
			.collect())
	}
}
