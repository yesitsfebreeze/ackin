//! What an entry declares, settled, and the directory each node is handed.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use crate::transport::cartridge::{Address, Directory, EventEntry};

use crate::error::{Error, Result};
use crate::loader::{Declared, Entry, Event, Grant};

use super::Host;

#[derive(Clone)]
pub struct Plan {
	pub id: String,
	pub name: String,
	pub root: PathBuf,
	pub entry: PathBuf,
	pub events: BTreeMap<String, Event>,
	pub needs: Vec<String>,
	pub on: Vec<String>,
	pub config: serde_json::Value,
	pub grant: Grant,
	pub sources: Vec<PathBuf>,
}

impl Plan {
	/// Everything that changes what the other cartridges are told.
	pub(crate) fn wiring(&self) -> (Vec<&String>, &[String], &[String]) {
		(self.events.keys().collect(), &self.needs, &self.on)
	}
}

fn exact(keys: &[String]) -> Result<()> {
	for key in keys {
		if key.trim().is_empty() || key.contains('*') {
			return Err(Error::Profile(format!(
				"`{key}` must be a nonempty exact event name (wildcards are unsupported)"
			)));
		}
	}
	Ok(())
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
		let mut needs = declared.needs.clone();
		needs.extend(entry.inject.iter().cloned());
		let mut on = declared.on.clone();
		exact(&needs)?;
		exact(&on)?;
		dedup(&mut needs);
		dedup(&mut on);
		let grant = self.expand_grant(&declared.grant, &config)?;
		Ok(Plan {
			id: entry.id.clone(),
			name: declared.name.clone(),
			root,
			entry: declared.entry.clone(),
			events: declared.events.clone(),
			needs,
			on,
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
		let mut config = match declared.settings.is_empty() && !carried {
			true => serde_json::Value::Null,
			false => crate::settings::apply(&declared.settings, settled, &declared.name)?,
		};
		if self.yolo && matches!(declared.name.as_str(), "agent" | "memo") {
			if config.is_null() {
				config = serde_json::json!({});
			}
			config
				.as_object_mut()
				.ok_or_else(|| Error::Settings("yolo requires object cartridge config".into()))?
				.insert("yolo".into(), true.into());
		}
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

	/// The one token a node accepts from senders and presents to the base.
	pub(crate) fn token(&self, id: &str) -> String {
		self.tokens
			.lock()
			.entry(id.to_owned())
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
	for (what, names) in [("listens to", &plan.on), ("needs", &plan.needs)] {
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
		for name in &plan.on {
			listeners
				.entry(name.clone())
				.or_default()
				.push(plan.id.clone());
		}
	}
	listeners
}

/// The directory of `plan` within `plans`.
pub(crate) fn directory(
	host: &Host,
	plan: &Plan,
	plans: &[Arc<Plan>],
	catalogue: &BTreeMap<String, (String, Event)>,
) -> Directory {
	let mut directory = Directory {
		needs: plan.needs.clone(),
		token: host.token(&plan.id),
		host: Some(Address {
			cartridge: "host".into(),
			socket: host.socket_path(),
			token: host.token(&plan.id),
		}),
		..Directory::default()
	};
	for (name, (owner, event)) in catalogue {
		let listeners = plans
			.iter()
			.filter(|p| p.on.iter().any(|n| n == name))
			.map(|listener| Address {
				cartridge: listener.id.clone(),
				socket: host.socket(&listener.id),
				token: host.token(&listener.id),
			})
			.collect();
		directory.events.insert(
			name.clone(),
			EventEntry {
				owner: owner.clone(),
				description: event.description.clone(),
				schema: event.schema.clone(),
				listeners,
			},
		);
	}
	directory
}
