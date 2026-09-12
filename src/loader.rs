use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::runtime::{Component, FiberHandle};
use mlua::{LuaSerdeExt, Table};
use notify::{RecursiveMode, Watcher};

use crate::lua::Host;

/// A profile entry names a cartridge folder; its manifest selects the Lua entry.
/// Explicit Lua files remain supported for programmatic composition.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
	pub id: String,
	pub path: String,
	#[serde(default)]
	pub config: serde_json::Value,
	#[serde(default)]
	pub disabled: bool,
	#[serde(default)]
	pub isolate: Vec<String>,
	#[serde(default)]
	pub inject: Vec<String>,
}

impl Entry {
	pub fn file(&self, dir: &Path) -> PathBuf {
		normalize(&classify(&dir.join(&self.path)))
	}
}

/// One entry of the profile as the files describe it, before any fiber runs.
pub struct CartridgeInfo {
	pub entry: Entry,
	pub inject: Vec<String>,
	pub provide: Vec<String>,
	pub error: Option<String>,
}

pub(crate) struct Loaded {
	pub(crate) entry: Entry,
	pub(crate) fiber: Option<FiberHandle>,
	pub(crate) sources: Vec<Source>,
	pub(crate) error: Option<String>,
	reload: crate::reload::Reload,
}

#[derive(Clone)]
pub(crate) struct Source {
	pub(crate) path: PathBuf,
	pub(crate) stamp: Option<(u64, std::time::SystemTime)>,
}

impl Source {
	fn new(path: PathBuf) -> Self {
		let stamp = std::fs::metadata(&path)
			.ok()
			.and_then(|m| Some((m.len(), m.modified().ok()?)));
		Self { path, stamp }
	}

	pub(crate) fn changed(&self) -> bool {
		self.stamp != Self::new(self.path.clone()).stamp
	}
}

fn normalize(path: &Path) -> PathBuf {
	path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Cartridge metadata is data, so reading it never evaluates the entry point.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cartridge {
	pub name: String,
	pub entry: String,
	/// Optional executable basename when it differs from the cartridge folder.
	pub binary: Option<String>,
	/// Optional Solid UI module, relative to this cartridge's folder.
	pub ui: Option<String>,
	/// Optional contract: a key this cartridge provides that proves its own
	/// behaviour. Declaring it is opt-in; [`Host::verify`] calls it.
	pub selftest: Option<String>,
	/// Optional contract: a key this cartridge provides that proves its wiring
	/// to what it injects, by calling a real dependency.
	pub integration: Option<String>,
	/// Repository URL or other retrieval reference when source is not installed.
	pub source: Option<String>,
}

impl Cartridge {
	/// Read a manifest and resolve the entry it names. The loader and the
	/// bundler share this, so what ships and what loads agree on the format.
	pub fn read(manifest: &Path) -> Result<(Cartridge, PathBuf), String> {
		let source =
			std::fs::read_to_string(manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
		let cartridge: Cartridge =
			serde_json::from_str(&source).map_err(|e| format!("{}: {e}", manifest.display()))?;
		if let Some(binary) = &cartridge.binary {
			if binary.is_empty()
				|| binary.contains('\0')
				|| Path::new(binary).components().count() != 1
				|| !matches!(
					Path::new(binary).components().next(),
					Some(std::path::Component::Normal(_))
				) {
				return Err(format!(
					"{}: binary must be an executable basename",
					manifest.display()
				));
			}
		}
		let entry = Path::new(&cartridge.entry);
		let named = !cartridge.name.trim().is_empty() && !cartridge.name.contains('\0');
		let relative = !entry.as_os_str().is_empty()
			&& entry
				.components()
				.all(|part| matches!(part, std::path::Component::Normal(_)))
			&& entry.extension().is_some_and(|ext| ext == "lua");
		if !named || !relative {
			return Err(format!(
				"{}: expected a nonempty name and a relative Lua entry inside the cartridge folder",
				manifest.display()
			));
		}
		let root = manifest
			.parent()
			.ok_or_else(|| format!("{}: no cartridge folder", manifest.display()))?
			.canonicalize()
			.map_err(|e| format!("{}: {e}", manifest.display()))?;
		let entry = root
			.join(entry)
			.canonicalize()
			.map_err(|e| format!("{}: {e}", manifest.display()))?;
		if !entry.starts_with(&root) {
			return Err(format!(
				"{}: cartridge entry escapes its folder",
				manifest.display()
			));
		}
		if let Some(ui) = &cartridge.ui {
			let path = Path::new(ui);
			if path.as_os_str().is_empty()
				|| !path
					.components()
					.all(|p| matches!(p, std::path::Component::Normal(_)))
				|| !path
					.extension()
					.is_some_and(|e| e == "tsx" || e == "ts" || e == "js" || e == "jsx")
			{
				return Err(format!(
					"{}: ui must be a relative JavaScript/TypeScript module",
					manifest.display()
				));
			}
			let resolved = root
				.join(path)
				.canonicalize()
				.map_err(|e| format!("{}: {e}", root.join(path).display()))?;
			if !resolved.starts_with(&root) || !resolved.is_file() {
				return Err(format!(
					"{}: ui must be a file inside its cartridge folder",
					manifest.display()
				));
			}
		}
		Ok((cartridge, entry))
	}
}

/// The file a path names: a Lua entry as written, otherwise the manifest of the
/// folder (or the manifest itself). One rule, so watching and loading agree.
fn classify(path: &Path) -> PathBuf {
	if path.extension().is_some_and(|ext| ext == "lua")
		|| path
			.file_name()
			.is_some_and(|name| name == "cartridge.json")
	{
		return path.to_path_buf();
	}
	path.join("cartridge.json")
}

pub(crate) fn resolve(path: &Path) -> mlua::Result<(PathBuf, String, Vec<PathBuf>)> {
	let path = normalize(&classify(path));
	if path.extension().is_some_and(|ext| ext == "lua") {
		let name = path
			.file_stem()
			.unwrap_or_default()
			.to_string_lossy()
			.into_owned();
		return Ok((path.clone(), name, vec![path]));
	}
	let (manifest, entry) = Cartridge::read(&path).map_err(mlua::Error::RuntimeError)?;
	let mut sources = vec![path.clone(), entry.clone()];
	if let Some(ui) = manifest.ui {
		sources.push(
			path
				.parent()
				.unwrap()
				.join(ui)
				.canonicalize()
				.map_err(mlua::Error::external)?,
		);
	}
	Ok((entry, manifest.name, sources))
}

fn validate(entry: &Entry) -> mlua::Result<()> {
	if entry.id.is_empty() || entry.path.is_empty() {
		return Err(mlua::Error::RuntimeError(
			"every entry needs a nonempty `id` and cartridge `path`".into(),
		));
	}
	Ok(())
}

/// Resolve a profile name under the project configuration directory.
/// An absolute name selects that exact directory.
pub fn profile(name: &str) -> PathBuf {
	Path::new(".zirkle").join(name)
}

/// Cartridges are loaded from the working directory unless `--dir` is supplied.
pub fn builtin() -> PathBuf {
	PathBuf::from("builtin")
}

impl Host {
	/// A profile explicitly grants the bridge client access to active plugins'
	/// own services. This avoids making every optional panel a required
	/// dependency.
	pub(crate) fn bridge_enabled(&self, uid: crate::runtime::Uid) -> bool {
		self.loaded.lock().iter().any(|slot| {
			slot.entry.config["bridge"] == true
				&& slot.fiber.as_ref().is_some_and(|fiber| fiber.uid() == uid)
		})
	}

	pub async fn bridge_call(
		&self,
		owner: &str,
		generation: u64,
		key: &str,
		args: serde_json::Value,
	) -> Result<serde_json::Value, String> {
		let value = {
			let loaded = self.loaded.lock();
			let slot = loaded
				.iter()
				.find(|slot| slot.entry.id == owner)
				.ok_or("bridge owner unloaded")?;
			let fiber = slot.fiber.as_ref().ok_or("bridge owner inactive")?;
			if fiber.uid() != generation || fiber.state() != Some(crate::runtime::State::Active) {
				return Err("bridge generation is no longer active".into());
			}
			if !slot.sources.iter().any(|s| {
				s.path
					.extension()
					.is_some_and(|e| e == "tsx" || e == "ts" || e == "js" || e == "jsx")
			}) {
				return Err("cartridge has no client module".into());
			}
			let reg = self.rt.reg.lock();
			reg
				.owns(generation, key)
				.and_then(|realm| reg.value(realm))
				.ok_or("a bridge client may only call services provided by its own cartridge")?
		};
		self.invoke(value, args).await
	}

	/// Client descriptors belong to committed, active generations. Failed
	/// candidates cannot change the status, and plugins without a client module
	/// have no descriptor.
	pub fn bridge_status(&self) -> serde_json::Value {
		let loaded = self.loaded.lock();
		serde_json::Value::Array(
			loaded
				.iter()
				.filter_map(|slot| {
					let fiber = slot.fiber.as_ref()?;
					if fiber.state() != Some(crate::runtime::State::Active) {
						return None;
					}
					let module = slot.sources.iter().find(|source| {
						source
							.path
							.extension()
							.is_some_and(|e| e == "tsx" || e == "ts" || e == "js" || e == "jsx")
					})?;
					let services: Vec<_> = self.rt.reg.lock().provided(fiber.uid()).into_iter().map(|(key, _)| key).collect();
					Some(
						serde_json::json!({"id": slot.entry.id, "generation": fiber.uid(), "module": module.path, "services": services}),
					)
				})
				.collect(),
		)
	}

	/// Folders of enabled cartridges, for records they ship (`.zirkle/memos`).
	/// Disabled entries have no fiber and are absent.
	pub fn cartridges(&self) -> serde_json::Value {
		let loaded = self.loaded.lock();
		serde_json::Value::Array(
			loaded
				.iter()
				.filter(|slot| slot.fiber.is_some())
				.map(|slot| {
					let dir = slot.entry.file(&self.dir).parent().map(Path::to_path_buf);
					serde_json::json!({"id": slot.entry.id, "dir": dir})
				})
				.collect(),
		)
	}

	fn eval<T: serde::de::DeserializeOwned>(&self, path: &Path) -> Result<T, mlua::Error> {
		let source = std::fs::read_to_string(path).map_err(mlua::Error::external)?;
		let value: Table = self
			.lua
			.load(&source)
			.set_name(path.to_string_lossy())
			.eval()?;
		self.lua.from_value(mlua::Value::Table(value))
	}

	/// `init.lua` lists the entries; `config.lua`, if present, maps entry id to
	/// config fields laid over the entry's own.
	fn entries(self: &Arc<Self>) -> Result<Vec<Entry>, mlua::Error> {
		let mut entries: Vec<Entry> = self.eval(&self.profile.join("init.lua"))?;
		let mut ids = std::collections::HashSet::new();
		for entry in &entries {
			validate(entry)?;
			if !ids.insert(&entry.id) {
				return Err(mlua::Error::RuntimeError(format!(
					"duplicate entry id `{}`",
					entry.id
				)));
			}
		}
		let config = self.profile.join("config.lua");
		if config.is_file() {
			let mut overrides: serde_json::Map<String, serde_json::Value> = self.eval(&config)?;
			for entry in &mut entries {
				let Some(over) = overrides.remove(&entry.id) else {
					continue;
				};
				match (&mut entry.config, over) {
					(serde_json::Value::Object(base), serde_json::Value::Object(over)) => base.extend(over),
					(base, over) => *base = over,
				}
			}
		}
		self.expand(&mut entries)?;
		Ok(entries)
	}

	/// `inject = {"tool.*"}` in a profile entry means every key with that prefix
	/// the profile's other enabled entries provide. The profile already names the
	/// cartridges; restating their keys is a second list to keep in step, and the
	/// keys are known here, before any fiber starts, from each cartridge's own
	/// declarations. An entry never injects what it provides itself.
	fn expand(self: &Arc<Self>, entries: &mut [Entry]) -> Result<(), mlua::Error> {
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
					return Err(mlua::Error::RuntimeError(format!(
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

	/// Load a profile, invoke one provided service, then dispose every instance.
	pub async fn run(
		self: &Arc<Self>,
		key: &str,
		args: serde_json::Value,
	) -> Result<serde_json::Value, String> {
		self.run_mode(key, args, |v| async { Ok(v) }).await
	}

	/// Load the profile and call every contract its manifests declare, then
	/// dispose every instance. Returns how many ran and one line per failure,
	/// each naming the cartridge, the obligation and the key. A contract fails
	/// when its call errors or returns `false`; a cartridge that declares none
	/// is never called.
	pub async fn verify(self: &Arc<Self>) -> Result<(usize, Vec<String>), String> {
		let contracts = self.contracts().map_err(|e| e.to_string())?;
		let mut failures = Vec::new();
		let mut lifecycle = self.rt.lifecycle();
		self.reconcile().await.map_err(|e| e.to_string())?;
		let settled = tokio::time::timeout(Duration::from_secs(30), async {
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
	fn contracts(self: &Arc<Self>) -> Result<Vec<(String, &'static str, String)>, mlua::Error> {
		let mut out = Vec::new();
		for entry in self.entries()?.iter().filter(|e| !e.disabled) {
			let manifest = normalize(&classify(&self.dir.join(&entry.path)));
			if !manifest.ends_with("cartridge.json") {
				continue;
			}
			let (cartridge, _) = Cartridge::read(&manifest).map_err(mlua::Error::RuntimeError)?;
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
	) -> Result<serde_json::Value, String>
	where
		F: FnOnce(serde_json::Value) -> Fut,
		Fut: std::future::Future<Output = Result<serde_json::Value, String>>,
	{
		self.run_mode(key, args, then).await
	}

	/// One line per entry that is not serving: load failures, then every
	/// fiber that is not active with its state, error and unmet injections.
	/// The reader fixes the named entry instead of guessing the chain.
	fn stalled(&self, fibers: &[crate::runtime::FiberInfo]) -> Vec<String> {
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

	async fn run_mode<F, Fut>(
		self: &Arc<Self>,
		key: &str,
		args: serde_json::Value,
		then: F,
	) -> Result<serde_json::Value, String>
	where
		F: FnOnce(serde_json::Value) -> Fut,
		Fut: std::future::Future<Output = Result<serde_json::Value, String>>,
	{
		let mut watcher = None;
		let result = async {
			let mut lifecycle = self.rt.lifecycle();
			self.reconcile().await.map_err(|e| e.to_string())?;
			// Hot reload is the host's normal behaviour: sources are watched
			// and changed cartridges replaced in place for every foreground run.
			watcher = Some(self.watch_mode().map_err(|e| e.to_string())?);
			tokio::time::timeout(Duration::from_secs(30), async {
				loop {
					let fibers = self.rt.fibers();
					if fibers
						.iter()
						.any(|f| f.state == crate::runtime::State::Active && f.provide.iter().any(|p| p == key))
					{
						return Ok(());
					}
					if !fibers
						.iter()
						.any(|f| f.state == crate::runtime::State::Loading)
					{
						return Err(format!(
							"service `{key}` unavailable: {}",
							self.stalled(&fibers).join("; ")
						));
					}
					let _ = lifecycle.recv().await;
				}
			})
			.await
			.map_err(|_| format!("service `{key}` did not become active"))??;
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

	pub fn component_of(self: &Arc<Self>, entry: &Entry) -> Result<Component, mlua::Error> {
		self.load_entry(entry).map(|(component, _)| component)
	}

	fn load_entry(self: &Arc<Self>, entry: &Entry) -> mlua::Result<(Component, Vec<PathBuf>)> {
		validate(entry)?;
		let (mut component, files) = self.load_component(
			&self.dir.join(&entry.path),
			entry.config.clone(),
			&entry.inject,
		)?;
		component.resident = true;
		Ok((component, files))
	}

	/// Effective declarations. Enabled process wrappers run hello, never apply.
	/// Disabled entries are not evaluated and never spawn even a hello process.
	pub fn manifest(self: &Arc<Self>) -> Result<Vec<CartridgeInfo>, mlua::Error> {
		Ok(
			self
				.entries()?
				.into_iter()
				.map(|entry| {
					let (inject, provide, error) = if entry.disabled {
						(Vec::new(), Vec::new(), None)
					} else {
						match self.component_of(&entry) {
							Ok(c) => (c.inject, c.provide, None),
							Err(e) => (Vec::new(), Vec::new(), Some(e.to_string())),
						}
					};
					CartridgeInfo {
						entry,
						inject,
						provide,
						error,
					}
				})
				.collect(),
		)
	}

	/// The loaded slot for `id`, under the lock.
	fn slot<R>(&self, id: &str, f: impl FnOnce(&mut Loaded) -> R) -> Option<R> {
		self
			.loaded
			.lock()
			.iter_mut()
			.find(|l| l.entry.id == id)
			.map(f)
	}

	fn spawn(&self, entry: &Entry, component: Component) -> FiberHandle {
		entry
			.isolate
			.iter()
			.fold(self.rt.ctx(), |ctx, key| ctx.isolate(key))
			.cartridge(component)
	}

	fn instantiate(self: &Arc<Self>, entry: Entry) -> Loaded {
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
		match self.load_entry(&loaded.entry) {
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

	pub async fn reconcile(self: &Arc<Self>) -> Result<(), mlua::Error> {
		let _reload = self.reload_lock.lock().await;
		let wanted = self.entries()?;
		let removed: Vec<_> = {
			let mut loaded = self.loaded.lock();
			let mut removed = Vec::new();
			let mut index = 0;
			while index < loaded.len() {
				if wanted
					.iter()
					.any(|e| e.id == loaded[index].entry.id && !e.disabled && !loaded[index].entry.disabled)
				{
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
				Some(old) if old != entry => self.replace_entry(entry).await,
				Some(_) => {}
				None => {
					let loaded = self.instantiate(entry);
					self.loaded.lock().push(loaded);
				}
			}
		}
		Ok(())
	}

	pub async fn replace(self: &Arc<Self>, path: &Path) {
		self.replace_changed(&[normalize(path)]).await;
	}

	async fn replace_changed(self: &Arc<Self>, paths: &[PathBuf]) {
		let _reload = self.reload_lock.lock().await;
		let entries: Vec<Entry> = self
			.loaded
			.lock()
			.iter()
			.filter(|l| {
				!l.entry.disabled
					&& l.sources.iter().any(|source| {
						(paths.contains(&source.path) && source.changed())
							|| (source
								.path
								.extension()
								.is_some_and(|e| e == "tsx" || e == "ts" || e == "js" || e == "jsx")
								&& source
									.path
									.parent()
									.is_some_and(|parent| paths.iter().any(|path| path.starts_with(parent))))
					})
			})
			.map(|l| l.entry.clone())
			.collect();
		for entry in entries {
			self.replace_entry(entry).await;
		}
	}

	fn report_entry(&self, id: &str, error: impl std::fmt::Display) {
		let error = error.to_string();
		self.slot(id, |slot| slot.error = Some(error.clone()));
		self.report(id, error);
	}

	async fn replace_entry(self: &Arc<Self>, entry: Entry) {
		if self
			.slot(&entry.id, |l| l.entry.isolate != entry.isolate)
			.unwrap_or(false)
		{
			self.report_entry(
				&entry.id,
				"changing isolation requires removing and recomposing the entry",
			);
			return;
		}
		let (mut component, files) = match self.load_entry(&entry) {
			Ok(c) => c,
			Err(e) => {
				self.report_entry(&entry.id, e);
				return;
			}
		};
		let sources = files.into_iter().map(Source::new).collect();
		let Some((old, reload)) = self.slot(&entry.id, |l| (l.fiber.clone(), l.reload.clone())) else {
			return;
		};
		let Some(old) = old else {
			component.reload = reload;
			let fiber = self.spawn(&entry, component);
			self.slot(&entry.id, |l| {
				l.entry = entry.clone();
				l.fiber = Some(fiber);
				l.sources = sources;
				l.error = None;
			});
			return;
		};
		old.settled().await;
		if old.state() != Some(crate::runtime::State::Active) {
			// A failed/inactive entry has no live generation to switch. Permit a
			// corrected entry to recover on the same reload transaction.
			old.dispose().await;
			component.reload = reload;
			let fiber = self.spawn(&entry, component);
			self.slot(&entry.id, |l| {
				l.entry = entry.clone();
				l.fiber = Some(fiber);
				l.sources = sources;
				l.error = None;
			});
			return;
		}
		if !reload.begin() {
			return;
		}
		let mut values = {
			let reg = self.rt.reg.lock();
			reg
				.provided(old.uid())
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
					self.report_entry(&entry.id, e);
					reload.finish();
					return;
				}
			}
		}
		let _calls = reload.gate().write_owned().await;
		component.reload = reload.clone();
		component.staged = true;
		let keys = component.provide.clone();
		let ctx = entry
			.isolate
			.iter()
			.chain(keys.iter())
			.fold(self.rt.ctx(), |ctx, key| ctx.isolate(key));
		let candidate = ctx.cartridge(component);
		candidate.settled().await;
		let result = if let Some(error) = candidate.error() {
			Err(error)
		} else {
			self
				.rt
				.switch(old.uid(), candidate.uid())
				.map_err(|e| e.to_string())
		};
		match result {
			Ok(()) => {
				self.slot(&entry.id, |l| {
					l.entry = entry.clone();
					l.fiber = Some(candidate);
					l.sources = sources;
					l.error = None;
				});
				reload.finish();
				drop(_calls);
				old.dispose().await;
				return;
			}
			Err(error) => {
				candidate.dispose().await;
				for value in &values {
					if let Some(service) = value.downcast_ref::<crate::service::Service>() {
						service.cancel().await;
					}
				}
				self.report_entry(&entry.id, error);
			}
		}
		reload.finish();
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
					host.replace_entry(entry).await;
				}
			});
		}
	}

	pub fn fiber_of(&self, id: &str) -> Option<FiberHandle> {
		self.slot(id, |l| l.fiber.clone()).flatten()
	}

	fn watch_sources(
		&self,
		watcher: &mut impl Watcher,
		watched: &mut std::collections::HashSet<PathBuf>,
	) -> notify::Result<()> {
		let dirs: Vec<_> = self
			.loaded
			.lock()
			.iter()
			.flat_map(|l| l.sources.iter())
			.filter_map(|source| source.path.parent().map(Path::to_path_buf))
			.collect();
		for dir in dirs {
			if !watched.contains(&dir) && !dir.starts_with(&self.dir) {
				watcher.watch(&dir, RecursiveMode::NonRecursive)?;
				watched.insert(dir);
			}
		}
		Ok(())
	}


	pub fn watch(self: &Arc<Self>) -> notify::Result<()> {
		self.watch_mode().map(|_| ())
	}

	fn watch_mode(self: &Arc<Self>) -> notify::Result<tokio::task::JoinHandle<()>> {
		let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<PathBuf>();
		let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
			if let Ok(event) = event {
				if matches!(event.kind, notify::EventKind::Access(_)) {
					return;
				}
				for path in event.paths {
					let _ = tx.send(normalize(&path));
				}
			}
		})?;
		watcher.watch(&self.dir, RecursiveMode::Recursive)?;
		// A source tree beside the host rebuilds on edit; a bundle has none.
		for path in [
			std::path::Path::new("core"),
			std::path::Path::new("Cargo.toml"),
		] {
			if path.exists() {
				watcher.watch(path, RecursiveMode::Recursive)?;
			}
		}
		if self.profile != self.dir {
			watcher.watch(&self.profile, RecursiveMode::NonRecursive)?;
		}
		let mut watched = std::collections::HashSet::from([self.profile.clone()]);
		self.watch_sources(&mut watcher, &mut watched)?;
		let host = self.clone();
		let task = tokio::spawn(async move {
			while let Some(first) = rx.recv().await {
				let mut changed = vec![first];
				tokio::time::sleep(Duration::from_millis(500)).await;
				while let Ok(p) = rx.try_recv() {
					changed.push(p);
				}
				changed.sort();
				changed.dedup();
				if changed.iter().any(|path| {
					path.parent() == Some(host.profile.as_path())
						&& matches!(
							path.file_name().and_then(|n| n.to_str()),
							Some("init.lua" | "config.lua")
						)
				}) {
					if let Err(e) = host.reconcile().await {
						host.report("init.lua", e);
					}
				}
				host.replace_changed(&changed).await;
				if let Err(e) = host.watch_sources(&mut watcher, &mut watched) {
					host.report("watch", e);
				}
			}
		});
		Ok(task)
	}
}
