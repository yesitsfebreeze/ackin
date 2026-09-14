//! What an entry will run as, and the directory each cartridge is handed.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use mlua::{Function, LuaSerdeExt};
use transport::cartridge::{Address, Directory, Grant as Access, Scope};

use crate::error::{Error, Result};
use crate::loader::{Declared, Entry, Grant};

use super::Host;

#[derive(Clone)]
pub enum Kind {
	Process(Vec<String>),
	Lua(Function),
}

#[derive(Clone)]
pub struct Plan {
	pub id: String,
	pub name: String,
	pub root: PathBuf,
	pub kind: Kind,
	pub provide: Vec<String>,
	pub needs: Vec<String>,
	pub on: Vec<String>,
	pub config: serde_json::Value,
	pub grant: Grant,
	pub sources: Vec<PathBuf>,
}

impl Plan {
	/// Everything that changes what the other cartridges are told.
	pub(crate) fn wiring(&self) -> (&[String], &[String], &[String]) {
		(&self.provide, &self.needs, &self.on)
	}
}

fn keys(lua: &mlua::Lua, table: &mlua::Table, field: &str) -> mlua::Result<Vec<String>> {
	match table.get::<mlua::Value>(field)? {
		mlua::Value::Nil => Ok(Vec::new()),
		value => lua.from_value(value),
	}
}

fn exact(keys: &[String]) -> Result<()> {
	for key in keys {
		if key.trim().is_empty() || key.contains('*') {
			return Err(Error::Profile(format!(
				"`{key}` must be a nonempty exact key (wildcards are unsupported)"
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
	/// Evaluate an entry's Lua and settle its configuration, without starting it.
	pub(crate) fn plan(self: &Arc<Self>, entry: &Entry) -> Result<Plan> {
		crate::loader::entries::validate(entry)?;
		let declared = crate::loader::resolve(&self.dir.join(&entry.path))?;
		let config = self.settle_config(&declared, entry.config.clone())?;
		let source = std::fs::read_to_string(&declared.entry)
			.map_err(|e| Error::file(&declared.entry, e))?;
		let module: mlua::Value = self
			.lua
			.load(&source)
			.set_name(declared.entry.to_string_lossy())
			.eval()?;
		let root = declared
			.entry
			.parent()
			.expect("resolved entry has a folder")
			.to_path_buf();
		let mut sources = declared.sources.clone();
		let (kind, mut provide, mut needs, extra, mut on) = match &module {
			mlua::Value::UserData(descriptor) => {
				let process = descriptor.borrow::<crate::lua::Process>()?;
				let mut command = process.command.clone();
				let executable = executable(&command[0], &root)?;
				command[0] = executable.to_string_lossy().into_owned();
				sources.push(executable);
				(
					Kind::Process(command),
					Vec::new(),
					Vec::new(),
					process.inject.clone(),
					Vec::new(),
				)
			}
			mlua::Value::Table(table) => {
				let apply: Function = table.get("apply")?;
				let mut needs = keys(&self.lua, table, "needs")?;
				needs.extend(keys(&self.lua, table, "inject")?);
				(
					Kind::Lua(apply),
					keys(&self.lua, table, "provide")?,
					needs,
					Vec::new(),
					keys(&self.lua, table, "on")?,
				)
			}
			_ => {
				return Err(Error::Profile(format!(
					"{}: an entry returns a table with `apply` or cartridge.process(...)",
					declared.entry.display()
				)))
			}
		};
		if !declared.provide.is_empty() {
			provide = declared.provide.clone();
		}
		if !declared.needs.is_empty() {
			needs = declared.needs.clone();
		}
		if !declared.on.is_empty() {
			on = declared.on.clone();
		}
		needs.extend(extra);
		needs.extend(entry.inject.iter().cloned());
		exact(&provide)?;
		exact(&needs)?;
		exact(&on)?;
		dedup(&mut needs);
		dedup(&mut on);
		let mut seen = std::collections::HashSet::new();
		for key in &provide {
			if !seen.insert(key) {
				return Err(Error::Profile(format!(
					"duplicate provide declaration `{key}`"
				)));
			}
		}
		needs.retain(|key| !provide.contains(key));
		Ok(Plan {
			id: entry.id.clone(),
			name: declared.name.clone(),
			root,
			kind,
			provide,
			needs,
			on,
			config,
			grant: declared.grant.clone(),
			sources,
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

	pub(crate) fn token(&self, from: &str, to: &str, scope: Scope) -> String {
		self.edges
			.lock()
			.entry((from.to_owned(), to.to_owned(), scope))
			.or_insert_with(transport::token)
			.clone()
	}
}

/// Which plan provides each key. A key provided twice is an error on the later entry.
pub(crate) fn providers(plans: &[Arc<Plan>]) -> (HashMap<String, String>, HashMap<String, String>) {
	let mut providers = HashMap::new();
	let mut clashes = HashMap::new();
	for plan in plans {
		for key in &plan.provide {
			match providers.get(key) {
				Some(first) => {
					clashes.insert(
						plan.id.clone(),
						format!("`{key}` is already provided by `{first}`"),
					);
				}
				None => {
					providers.insert(key.clone(), plan.id.clone());
				}
			}
		}
	}
	(providers, clashes)
}

/// The directory of `plan` within `plans`.
pub(crate) fn directory(
	host: &Host,
	plan: &Plan,
	plans: &[Arc<Plan>],
	providers: &HashMap<String, String>,
) -> Directory {
	let by_id: HashMap<&str, &Arc<Plan>> = plans.iter().map(|p| (p.id.as_str(), p)).collect();
	let mut directory = Directory {
		host: Some(Address {
			cartridge: "host".into(),
			socket: host.inner_socket(),
			token: host.token(&plan.id, "host", Scope::Call),
		}),
		..Directory::default()
	};
	for key in &plan.needs {
		if let Some(provider) = providers.get(key) {
			directory.needs.insert(
				key.clone(),
				Address {
					cartridge: provider.clone(),
					socket: host.socket(provider),
					token: host.token(&plan.id, provider, Scope::Call),
				},
			);
		}
	}
	for listener in plans.iter().filter(|p| !p.on.is_empty()) {
		for name in &listener.on {
			directory
				.events
				.entry(name.clone())
				.or_default()
				.push(Address {
					cartridge: listener.id.clone(),
					socket: host.socket(&listener.id),
					token: host.token(&plan.id, &listener.id, Scope::Event),
				});
		}
	}
	let mut callers: BTreeMap<String, Vec<String>> = BTreeMap::new();
	for other in plans {
		for key in &other.needs {
			if providers.get(key).is_some_and(|p| *p == plan.id) {
				callers
					.entry(other.id.clone())
					.or_default()
					.push(key.clone());
			}
		}
	}
	for (from, names) in callers {
		directory.accept.insert(
			host.token(&from, &plan.id, Scope::Call),
			Access {
				from,
				scope: Scope::Call,
				names,
			},
		);
	}
	if !plan.on.is_empty() {
		for other in by_id.keys() {
			directory.accept.insert(
				host.token(other, &plan.id, Scope::Event),
				Access {
					from: (*other).to_owned(),
					scope: Scope::Event,
					names: plan.on.clone(),
				},
			);
		}
	}
	directory
}

/// Resolve a program name: the cartridge's `.cartridge/bin` and `bin`, beside
/// the running host, then `PATH`. A path with a separator is relative to the cartridge.
pub fn executable(program: &str, root: &std::path::Path) -> Result<PathBuf> {
	use std::os::unix::fs::PermissionsExt;
	let path = std::path::Path::new(program);
	let candidates: Vec<PathBuf> = if path.components().count() > 1 || path.is_absolute() {
		vec![root.join(path)]
	} else {
		let beside = std::env::current_exe()
			.ok()
			.and_then(|exe| exe.parent().map(|dir| dir.join(program)));
		[
			root.join(".cartridge/bin").join(program),
			root.join("bin").join(program),
		]
		.into_iter()
		.chain(beside)
		.chain(std::env::var_os("PATH").into_iter().flat_map(|paths| {
			std::env::split_paths(&paths)
				.map(|dir| dir.join(program))
				.collect::<Vec<_>>()
		}))
		.collect()
	};
	candidates
		.into_iter()
		.find(|path| {
			path.metadata()
				.is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
		})
		.ok_or_else(|| {
			Error::process(
				program,
				"process executable was not found or is not executable",
			)
		})?
		.canonicalize()
		.map_err(|e| Error::file(program, e))
}
