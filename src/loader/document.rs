//! `cartridge.json`: the format, read as data and checked against itself, and
//! the resolution of the files it names.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

use super::normalize;

/// Cartridge metadata is data, so reading it never evaluates the entry point.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cartridge {
	pub name: String,
	pub entry: String,
	/// Human-readable purpose, alongside the executable behavior declaration.
	pub description: Option<String>,
	/// Documented commands; reading a manifest never executes them.
	#[serde(default)]
	pub commands: std::collections::BTreeMap<String, Command>,
	/// Optional executable basename when it differs from the cartridge folder.
	pub binary: Option<String>,
	/// Optional Solid UI module, relative to this cartridge's folder.
	pub ui: Option<String>,
	/// A contract: an event this cartridge listens to that proves its behaviour;
	/// `cartridge verify` emits it.
	pub selftest: Option<String>,
	/// A contract: an event this cartridge listens to that proves its wiring.
	pub integration: Option<String>,
	/// Repository URL or other retrieval reference when source is not installed.
	pub source: Option<String>,
	/// The cartridge's own configuration, carried by the document. The ledger
	/// shape has no profile `config.lua` to lay fields on at composition time,
	/// so the document is where an author's configuration travels; the caller's
	/// own config, when it names one, is laid over it.
	#[serde(default)]
	pub config: serde_json::Value,
	/// The keys this cartridge contributes to the configuration: dotted name to
	/// `{type, default, min, max, doc}`. What is declared here is filled with
	/// its default before the cartridge starts, checked against its kind and
	/// bounds, listed by `cartridge settings`, and settable in the global
	/// `~/.cartridge/config.lua` or the project's own — under this entry's id.
	/// A cartridge that declares its keys carries no fallbacks of its own.
	#[serde(default)]
	pub settings: crate::settings::Specs,
	/// Events this cartridge defines: name to description and payload schema.
	#[serde(default)]
	pub events: std::collections::BTreeMap<String, Event>,
	/// Events that must have a listener before this cartridge starts.
	#[serde(default)]
	pub needs: Vec<String>,
	/// Events this cartridge listens to.
	#[serde(default)]
	pub listen: Vec<String>,
	/// The capability request: the same declaration the resolver grants and the
	/// sandbox confines to. Absent means nothing is asked for, which is the
	/// tightest policy and not the loosest.
	#[serde(default)]
	pub grant: Grant,
}

/// One event a cartridge defines. `schema` is a JSON Schema for the payload;
/// absent, any payload passes.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub schema: Option<serde_json::Value>,
}

/// An argument array to run from `cwd`, relative to the manifest's folder.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
	pub argv: Vec<String>,
	pub cwd: String,
	pub description: Option<String>,
}

/// What a cartridge asks the machine for. Read twice — once to grant, once to
/// confine — out of this one document, so there is no second policy file.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
	/// Filesystem paths readable by this cartridge, relative to its own folder
	/// unless absolute.
	#[serde(default)]
	pub read: Vec<String>,
	/// Filesystem paths writable by this cartridge. A writable path is readable.
	#[serde(default)]
	pub write: Vec<String>,
	/// Hosts this cartridge may reach. `*` is every host.
	#[serde(default)]
	pub net: Vec<String>,
	/// Programs this cartridge may execute, by basename or path.
	#[serde(default)]
	pub exec: Vec<String>,
}

impl Cartridge {
	/// The document, read and checked against itself and nothing else. Every
	/// failure here is a failure of the document: it will not parse, or it
	/// declares something the format refuses. Nothing on the filesystem around
	/// the cartridge is touched — the Lua entry is not resolved and a declared
	/// `ui` file is not looked for — because a cartridge declares what it
	/// declares whether or not the files it points at are in place.
	pub fn document(manifest: &Path) -> Result<Cartridge> {
		let source = std::fs::read_to_string(manifest).map_err(|e| Error::file(manifest, e))?;
		let cartridge: Cartridge =
			serde_json::from_str(&source).map_err(|e| Error::document(manifest, e.to_string()))?;
		if let Some(binary) = &cartridge.binary {
			if binary.is_empty()
				|| binary.contains('\0')
				|| Path::new(binary).components().count() != 1
				|| !matches!(
					Path::new(binary).components().next(),
					Some(std::path::Component::Normal(_))
				) {
				return Err(Error::document(
					manifest,
					"binary must be an executable basename",
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
			return Err(Error::document(
				manifest,
				"expected a nonempty name and a relative Lua entry inside the cartridge folder",
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
				return Err(Error::document(
					manifest,
					"ui must be a relative JavaScript/TypeScript module",
				));
			}
		}
		cartridge.check(manifest)?;
		Ok(cartridge)
	}

	/// The document, plus the files it names: the Lua entry is resolved and a
	/// declared `ui` is located. The loader and the bundler share this, so what
	/// ships and what loads agree on the format. An error from here may be a
	/// fact about the tree rather than about the document — which is why
	/// [`Cartridge::document`] exists beside it.
	pub fn read(manifest: &Path) -> Result<(Cartridge, PathBuf)> {
		let cartridge = Self::document(manifest)?;
		let entry = Path::new(&cartridge.entry);
		let root = manifest
			.parent()
			.ok_or_else(|| Error::document(manifest, "no cartridge folder"))?
			.canonicalize()
			.map_err(|e| Error::file(manifest, e))?;
		let entry = root
			.join(entry)
			.canonicalize()
			.map_err(|e| Error::file(manifest, e))?;
		if !entry.starts_with(&root) {
			return Err(Error::document(
				manifest,
				"cartridge entry escapes its folder",
			));
		}
		if let Some(ui) = &cartridge.ui {
			let path = Path::new(ui);
			let resolved = root
				.join(path)
				.canonicalize()
				.map_err(|e| Error::file(root.join(path), e))?;
			if !resolved.starts_with(&root) || !resolved.is_file() {
				return Err(Error::document(
					manifest,
					"ui must be a file inside its cartridge folder",
				));
			}
		}
		Ok((cartridge, entry))
	}

	/// Every declaration this document carries, checked without reading anything
	/// else. A key is a nonempty exact string; a wildcard is not a declaration.
	fn check(&self, manifest: &Path) -> Result<()> {
		let at = |what: &str| Error::document(manifest, what);
		let key = |field: &str, k: &String| -> Result<()> {
			if k.trim().is_empty() || k.contains('*') || k.contains('\0') {
				return Err(at(&format!(
					"`{field}` entry `{k}` must be a nonempty exact key"
				)));
			}
			Ok(())
		};
		for (name, event) in &self.events {
			key("events", name)?;
			if let Some(schema) = &event.schema {
				jsonschema::validator_for(schema).map_err(|e| {
					at(&format!("`events.{name}.schema` is not a JSON Schema: {e}"))
				})?;
			}
		}
		let mut heard = std::collections::HashSet::new();
		for k in &self.listen {
			key("listen", k)?;
			if !heard.insert(k.as_str()) {
				return Err(at(&format!("duplicate listen declaration `{k}`")));
			}
		}
		let mut asked = std::collections::HashSet::new();
		for k in &self.needs {
			key("needs", k)?;
			if !asked.insert(k.as_str()) {
				return Err(at(&format!("duplicate needs declaration `{k}`")));
			}
		}
		for (field, contract) in [
			("selftest", &self.selftest),
			("integration", &self.integration),
		] {
			if let Some(k) = contract {
				if !self.listen.contains(k) {
					return Err(at(&format!(
						"`{field}` names `{k}`, which this cartridge does not listen to"
					)));
				}
			}
		}
		self.grant.check(&at)
	}
}

/// The document's name on disk. A folder is a cartridge exactly when it holds
/// one, and every reader of the format agrees on this one spelling.
pub const MANIFEST: &str = "cartridge.json";

impl Grant {
	fn check(&self, at: &dyn Fn(&str) -> Error) -> Result<()> {
		for (field, paths) in [("read", &self.read), ("write", &self.write)] {
			for p in paths {
				let path = Path::new(p);
				// A path is blank on the same terms a key is: `trim()` on both, so
				// `"   "` is refused in a grant exactly as it is in `listen`.
				if p.trim().is_empty() || p.contains('\0') {
					return Err(at(&format!(
						"`grant.{field}` entry `{p}` must be a nonempty exact path"
					)));
				}
				if !path.is_absolute()
					&& !path
						.components()
						.all(|c| matches!(c, std::path::Component::Normal(_)))
				{
					return Err(at(&format!(
						"`grant.{field}` path `{p}` must be absolute or stay inside the cartridge folder"
					)));
				}
			}
		}
		for host in &self.net {
			if host.trim().is_empty() || host.contains('\0') || (host.contains('*') && host != "*")
			{
				return Err(at(&format!(
					"`grant.net` entry `{host}` must be a host name or `*`"
				)));
			}
		}
		for program in &self.exec {
			if program.trim().is_empty() || program.contains('\0') {
				return Err(at("`grant.exec` needs a nonempty program"));
			}
		}
		Ok(())
	}
}

/// The file a path names: a Lua entry as written, otherwise the manifest of the
/// folder (or the manifest itself). One rule, so watching and loading agree.
pub(super) fn classify(path: &Path) -> PathBuf {
	if path.extension().is_some_and(|ext| ext == "lua")
		|| path.file_name().is_some_and(|name| name == MANIFEST)
	{
		return path.to_path_buf();
	}
	path.join(MANIFEST)
}

/// What an entry declares, with every file it names resolved. A bare Lua entry
/// has no document and declares through the table it returns.
pub(crate) struct Declared {
	pub(crate) entry: PathBuf,
	pub(crate) name: String,
	pub(crate) sources: Vec<PathBuf>,
	pub(crate) events: std::collections::BTreeMap<String, Event>,
	pub(crate) needs: Vec<String>,
	pub(crate) listen: Vec<String>,
	pub(crate) config: serde_json::Value,
	pub(crate) settings: crate::settings::Specs,
	pub(crate) grant: Grant,
}

/// The grant a profile entry's document declares, without resolving its files.
pub(crate) fn document(path: &Path) -> Result<Grant> {
	let path = normalize(&classify(path));
	if path.extension().is_some_and(|ext| ext == "lua") {
		return Ok(Grant::default());
	}
	Ok(Cartridge::document(&path)?.grant)
}

pub(crate) fn resolve(path: &Path) -> Result<Declared> {
	let path = normalize(&classify(path));
	if path.extension().is_some_and(|ext| ext == "lua") {
		let name = path
			.file_stem()
			.unwrap_or_default()
			.to_string_lossy()
			.into_owned();
		return Ok(Declared {
			entry: path.clone(),
			name,
			sources: vec![path],
			events: Default::default(),
			needs: Vec::new(),
			listen: Vec::new(),
			config: serde_json::Value::Null,
			settings: Default::default(),
			grant: Grant::default(),
		});
	}
	let (manifest, entry) = Cartridge::read(&path)?;
	let root = path.parent().expect("manifest has a folder");
	let mut sources = vec![path.clone(), entry.clone()];
	if let Some(ui) = &manifest.ui {
		let ui = root.join(ui);
		sources.push(ui.canonicalize().map_err(|e| Error::file(&ui, e))?);
	}
	Ok(Declared {
		entry,
		name: manifest.name,
		sources,
		events: manifest.events,
		needs: manifest.needs,
		listen: manifest.listen,
		config: manifest.config,
		settings: manifest.settings,
		grant: manifest.grant,
	})
}
