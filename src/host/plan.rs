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
	pub entry_sha256: String,
	pub events: BTreeMap<String, Event>,
	pub needs: Vec<String>,
	pub listen: Vec<String>,
	pub config: serde_json::Value,
	pub grant: Grant,
	pub sources: Vec<PathBuf>,
	/// The address the host binds for this node, from the manifest's
	/// `listener` key in the settled config; empty or absent means none.
	pub listener: Option<String>,
}

impl Plan {
	/// Includes each event's schema, not just its name: a schema-only change
	/// must still register as a wiring change.
	pub(crate) fn wiring(&self) -> (&BTreeMap<String, Event>, &[String], &[String]) {
		(&self.events, &self.needs, &self.listen)
	}

	pub(crate) fn sends(&self, name: &str) -> bool {
		self.events.contains_key(name) || self.needs.iter().any(|n| n == name)
	}
}

fn exact(keys: &[String]) -> Result<()> {
	for key in keys {
		if key.trim().is_empty() || key.contains('*') {
			return Err(Error::Descriptor(format!(
				"`{key}` must be a nonempty exact event name"
			)));
		}
	}
	Ok(())
}

fn expand(needs: Vec<String>, listened: &[String]) -> Result<Vec<String>> {
	let mut out = Vec::new();
	for key in needs {
		match key.strip_suffix('*') {
			None => out.push(key),
			Some("") => {
				return Err(Error::Descriptor(
					"a bare `*` in needs; a glob needs a prefix".into(),
				))
			}
			Some(prefix) => out.extend(listened.iter().filter(|k| k.starts_with(prefix)).cloned()),
		}
	}
	Ok(out)
}

fn configured(needs: Vec<String>, config: &serde_json::Value) -> Result<Vec<String>> {
	let mut out = Vec::new();
	for key in needs {
		let Some(setting) = key
			.strip_prefix("${config.")
			.and_then(|rest| rest.strip_suffix('}'))
		else {
			out.push(key);
			continue;
		};
		let value = crate::settings::get(config, setting)
			.and_then(serde_json::Value::as_str)
			.ok_or_else(|| Error::Descriptor(format!("need `{key}` names no string setting")))?;
		if !value.is_empty() {
			out.push(value.to_owned());
		}
	}
	Ok(out)
}

fn dedup(keys: &mut Vec<String>) {
	let mut seen = std::collections::HashSet::new();
	keys.retain(|key| seen.insert(key.clone()));
}

impl Host {
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
		needs = configured(needs, &config)?;
		exact(&needs)?;
		dedup(&mut needs);
		let grant = self.expand_grant(&declared.grant, &config)?;
		let listener = declared
			.listener
			.as_deref()
			.and_then(|key| crate::settings::get(&config, key))
			.and_then(serde_json::Value::as_str)
			.filter(|address| !address.is_empty())
			.map(str::to_owned);
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
			listener,
		})
	}

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

	fn expand_grant(&self, grant: &Grant, config: &serde_json::Value) -> Result<Grant> {
		let project = self
			.descriptor
			.parent()
			.unwrap_or(&self.descriptor)
			.to_path_buf();
		let expand = |path: &String| -> Result<String> {
			if let Some(key) = path
				.strip_prefix("${config.")
				.and_then(|rest| rest.strip_suffix('}'))
			{
				let value = crate::settings::get(config, key)
					.and_then(serde_json::Value::as_str)
					.ok_or_else(|| {
						Error::Descriptor(format!("grant `{path}` names no string setting"))
					})?;
				return Ok(project.join(value).to_string_lossy().into_owned());
			}
			if let Some(rest) = path.strip_prefix("$HOME") {
				if rest.is_empty() || rest.starts_with('/') {
					let home = crate::sandbox::home().filter(|home| home.is_absolute());
					let Some(home) = home else {
						let var = if cfg!(target_os = "windows") {
							"USERPROFILE"
						} else {
							"HOME"
						};
						return Err(Error::Descriptor(format!(
							"grant `{path}` expands $HOME, but {var} is not set to an absolute path"
						)));
					};
					return Ok(format!("{}{rest}", home.to_string_lossy()));
				}
			}
			let tmp = std::env::temp_dir();
			for (name, base) in [
				("$PROJECT", project.to_string_lossy().into_owned()),
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
		let env = configured(grant.env.clone(), config)?;
		if env.iter().any(|name| name == "*") {
			return Err(Error::Descriptor(
				"a bare `*` in grant.env; name a variable or a prefix".into(),
			));
		}
		Ok(Grant {
			read: all(&grant.read)?,
			write: all(&grant.write)?,
			net: grant.net.clone(),
			exec: all(&grant.exec)?,
			env,
		})
	}

	pub(crate) fn token(&self, from: &str, to: Option<&str>) -> String {
		self.tokens
			.lock()
			.entry((from.to_owned(), to.map(str::to_owned)))
			.or_insert_with(crate::transport::token)
			.clone()
	}
}

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
