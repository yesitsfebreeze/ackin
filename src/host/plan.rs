//! What an entry declares, settled, and the directory each node is handed.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use crate::transport::cartridge::{Accept, Address, Directory, EventEntry};

use crate::error::{Error, Result};
use crate::loader::{Declared, Entry, Event, Grant};

use super::Host;

#[derive(Clone)]
pub struct Plan {
	pub id: String,
	pub name: String,
	pub root: PathBuf,
	pub entry: PathBuf,
	/// The entry's SHA-256 as the base verified it, hex — handed to the node,
	/// which refuses bytes that no longer match.
	pub entry_sha256: String,
	pub events: BTreeMap<String, Event>,
	pub needs: Vec<String>,
	pub listen: Vec<String>,
	pub config: serde_json::Value,
	pub grant: Grant,
	pub sources: Vec<PathBuf>,
}

impl Plan {
	/// Everything that changes what the other cartridges are told.
	pub(crate) fn wiring(&self) -> (&BTreeMap<String, Event>, &[String], &[String]) {
		(&self.events, &self.needs, &self.listen)
	}

	/// Whether this cartridge may send `name`: it defines or needs it.
	pub(crate) fn sends(&self, name: &str) -> bool {
		self.events.contains_key(name) || self.needs.iter().any(|n| n == name)
	}
}

fn exact(keys: &[String]) -> Result<()> {
	for key in keys {
		if key.trim().is_empty() || key.contains('*') {
			return Err(Error::Profile(format!(
				"`{key}` must be a nonempty exact event name"
			)));
		}
	}
	Ok(())
}

/// `tool.*` in `needs` names every event with that prefix another enabled
/// entry listens to.
fn expand(needs: Vec<String>, listened: &[String]) -> Result<Vec<String>> {
	let mut out = Vec::new();
	for key in needs {
		match key.strip_suffix('*') {
			None => out.push(key),
			Some("") => {
				return Err(Error::Profile(
					"a bare `*` in needs; a glob needs a prefix".into(),
				))
			}
			Some(prefix) => out.extend(listened.iter().filter(|k| k.starts_with(prefix)).cloned()),
		}
	}
	Ok(out)
}

fn dedup(keys: &mut Vec<String>) {
	let mut seen = std::collections::HashSet::new();
	keys.retain(|key| seen.insert(key.clone()));
}

impl Host {
	/// Read an entry's declaration and settle its configuration, without running it.
	pub(crate) fn plan(self: &Arc<Self>, entry: &Entry) -> Result<Plan> {
		crate::loader::entries::validate(entry)?;
		let declared = crate::loader::resolve(&self.dir.join(&entry.path))?;
		let config = self.settle_config(&declared, entry.config.clone())?;
		let root = declared
			.entry
			.parent()
			.expect("resolved entry has a folder")
			.to_path_buf();
		let mut listen = declared.listen.clone();
		exact(&listen)?;
		dedup(&mut listen);
		let mut needs = declared.needs.clone();
		if needs.iter().any(|k| k.ends_with('*')) {
			let mut listened = Vec::new();
			for other in self
				.entries()?
				.iter()
				.filter(|e| !e.disabled && e.id != entry.id)
			{
				if let Ok(declared) = crate::loader::resolve(&self.dir.join(&other.path)) {
					listened.extend(declared.listen);
				}
			}
			listened.sort();
			listened.dedup();
			needs = expand(needs, &listened)?;
		}
		exact(&needs)?;
		dedup(&mut needs);
		let grant = self.expand_grant(&declared.grant, &config)?;
		Ok(Plan {
			id: entry.id.clone(),
			name: declared.name.clone(),
			root,
			entry: declared.entry.clone(),
			entry_sha256: declared.entry_sha256.clone(),
			events: declared.events.clone(),
			needs,
			listen,
			config,
			grant,
			sources: declared.sources.clone(),
		})
	}

	/// Declared defaults, then the document's `config`, then the entry's.
	fn settle_config(
		&self,
		declared: &Declared,
		config: serde_json::Value,
	) -> Result<serde_json::Value> {
		let mut settled = crate::settings::defaults(&declared.settings);
		let mut carried = false;
		for layer in [declared.config.clone(), config] {
			if !layer.is_null() {
				crate::settings::merge(&mut settled, layer);
				carried = true;
			}
		}
		let config = match declared.settings.is_empty() && !carried {
			true => serde_json::Value::Null,
			false => crate::settings::apply(&declared.settings, settled, &declared.name)?,
		};
		Ok(config)
	}

	/// Grant paths may start with `$PROJECT`, `$HOME` or `$TMPDIR`, or be
	/// `${config.<key>}`, a settled config value; a relative config value is
	/// relative to the project root, where cartridges run.
	fn expand_grant(&self, grant: &Grant, config: &serde_json::Value) -> Result<Grant> {
		let project = self.profile.parent().unwrap_or(&self.profile).to_path_buf();
		let expand = |path: &String| -> Result<String> {
			if let Some(key) = path
				.strip_prefix("${config.")
				.and_then(|rest| rest.strip_suffix('}'))
			{
				let value = crate::settings::get(config, key)
					.and_then(serde_json::Value::as_str)
					.ok_or_else(|| {
						Error::Profile(format!("grant `{path}` names no string setting"))
					})?;
				return Ok(project.join(value).to_string_lossy().into_owned());
			}
			let home = std::env::var("HOME").unwrap_or_default();
			let tmp = std::env::temp_dir();
			for (name, base) in [
				("$PROJECT", project.to_string_lossy().into_owned()),
				("$HOME", home),
				(
					"$TMPDIR",
					tmp.to_string_lossy().trim_end_matches('/').to_owned(),
				),
			] {
				if let Some(rest) = path.strip_prefix(name) {
					if rest.is_empty() || rest.starts_with('/') {
						return Ok(format!("{base}{rest}"));
					}
				}
			}
			Ok(path.clone())
		};
		let all = |paths: &[String]| paths.iter().map(expand).collect::<Result<Vec<_>>>();
		Ok(Grant {
			read: all(&grant.read)?,
			write: all(&grant.write)?,
			net: grant.net.clone(),
			exec: all(&grant.exec)?,
		})
	}

	/// The token `from` presents to `to`, or to the base when `to` is `None`.
	/// One per edge, so a listener holds nothing it could present elsewhere.
	pub(crate) fn token(&self, from: &str, to: Option<&str>) -> String {
		self.tokens
			.lock()
			.entry((from.to_owned(), to.map(str::to_owned)))
			.or_insert_with(crate::transport::token)
			.clone()
	}
}

/// Every event the plans declare, by name, with its owner; and the plans that
/// declare one twice, with why they fail.
pub(crate) fn catalogue(
	plans: &[Arc<Plan>],
) -> (BTreeMap<String, (String, Event)>, HashMap<String, String>) {
	let mut catalogue = BTreeMap::new();
	let mut clashes = HashMap::new();
	for plan in plans {
		for (name, event) in &plan.events {
			match catalogue.get(name) {
				Some((first, _)) => {
					clashes.insert(
						plan.id.clone(),
						format!("`{name}` is already declared by `{first}`"),
					);
				}
				None => {
					catalogue.insert(name.clone(), (plan.id.clone(), event.clone()));
				}
			}
		}
	}
	(catalogue, clashes)
}

/// Why `plan` may not start against `catalogue`: a listened-to or needed event
/// nobody declares.
pub(crate) fn unmatched(
	plan: &Plan,
	catalogue: &BTreeMap<String, (String, Event)>,
) -> Option<String> {
	for (what, names) in [("listens to", &plan.listen), ("needs", &plan.needs)] {
		if let Some(name) = names.iter().find(|name| !catalogue.contains_key(*name)) {
			return Some(format!("{what} `{name}`, which no cartridge declares"));
		}
	}
	None
}

/// Which plans listen to each event, in composition order.
pub(crate) fn listeners(plans: &[Arc<Plan>]) -> HashMap<String, Vec<String>> {
	let mut listeners: HashMap<String, Vec<String>> = HashMap::new();
	for plan in plans {
		for name in &plan.listen {
			listeners
				.entry(name.clone())
				.or_default()
				.push(plan.id.clone());
		}
	}
	listeners
}

fn entry(owner: &str, event: &Event, listeners: Vec<Address>) -> EventEntry {
	EventEntry {
		owner: owner.to_owned(),
		description: event.description.clone(),
		schema: event.schema.clone(),
		timeout_ms: event
			.timeout_ms
			.unwrap_or(crate::settings::host().event_timeout_ms),
		listeners,
	}
}

/// The directory of `plan` within `plans`: listeners and tokens only for what
/// it may send, and a token for each cartridge that may send it something.
pub(crate) fn directory(
	host: &Host,
	plan: &Plan,
	plans: &[Arc<Plan>],
	catalogue: &BTreeMap<String, (String, Event)>,
) -> Directory {
	let mut directory = Directory {
		needs: plan.needs.clone(),
		host: Some(Address {
			cartridge: "host".into(),
			socket: host.socket_path(),
			token: host.token(&plan.id, None),
		}),
		..Directory::default()
	};
	for (name, (owner, event)) in catalogue {
		let mut listeners = Vec::new();
		if plan.sends(name) {
			directory.sends.push(name.clone());
			listeners = plans
				.iter()
				.filter(|p| p.listen.iter().any(|n| n == name))
				.map(|listener| Address {
					cartridge: listener.id.clone(),
					socket: host.socket(&listener.id),
					token: host.token(&plan.id, Some(&listener.id)),
				})
				.collect();
		}
		directory
			.events
			.insert(name.clone(), entry(owner, event, listeners));
	}
	for sender in plans {
		let events: Vec<String> = plan
			.listen
			.iter()
			.filter(|name| catalogue.contains_key(*name) && sender.sends(name))
			.cloned()
			.collect();
		if !events.is_empty() {
			directory.accept.insert(
				host.token(&sender.id, Some(&plan.id)),
				Accept {
					from: sender.id.clone(),
					events,
				},
			);
		}
	}
	directory
}

/// What the base sends with: every event, to its active listeners, as the host.
pub(crate) fn host_directory(
	host: &Host,
	active: &[Arc<Plan>],
	catalogue: &BTreeMap<String, (String, Event)>,
) -> Directory {
	let mut directory = Directory::default();
	for (name, (owner, event)) in catalogue {
		directory.sends.push(name.clone());
		let listeners = active
			.iter()
			.filter(|p| p.listen.iter().any(|n| n == name))
			// The base answers bail/gather as a sender presenting the
			// listener's own node token.
			.map(|listener| Address {
				cartridge: listener.id.clone(),
				socket: host.socket(&listener.id),
				token: host.node_token(&listener.id),
			})
			.collect();
		directory
			.events
			.insert(name.clone(), entry(owner, event, listeners));
	}
	directory
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/host/plan.rs"]
mod tests;
