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
	/// Optional contract: a key this cartridge provides that proves its own
	/// behaviour. Declaring it is opt-in; [`Host::verify`] calls it.
	pub selftest: Option<String>,
	/// Optional contract: a key this cartridge provides that proves its wiring
	/// to what it injects, by calling a real dependency.
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
	/// Keys this cartridge offers. Private to its own subtree unless a parent
	/// re-exports them: two cartridges may provide the same key without
	/// colliding so long as neither subtree passes it into the other.
	#[serde(default)]
	pub provide: Vec<String>,
	/// Keys this cartridge asks for. Resolved against its own subtree first,
	/// then outward.
	#[serde(default)]
	pub needs: Vec<String>,
	/// Keys provided somewhere inside this cartridge's subtree that it passes
	/// outward under its own name. The second naming the nesting rule costs.
	#[serde(default)]
	pub export: Vec<String>,
	/// The capability request: the same declaration the resolver grants and the
	/// sandbox confines to. Absent means nothing is asked for, which is the
	/// tightest policy and not the loosest.
	#[serde(default)]
	pub grant: Grant,
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
#[derive(Default, serde::Deserialize)]
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
		let mut seen = std::collections::HashSet::new();
		for k in &self.provide {
			key("provide", k)?;
			if !seen.insert(k.as_str()) {
				return Err(at(&format!("duplicate provide declaration `{k}`")));
			}
		}
		for k in &self.export {
			key("export", k)?;
			if self.provide.contains(k) {
				return Err(at(&format!(
					"`{k}` is provided here, so it is not re-exported from inside"
				)));
			}
			if !seen.insert(k.as_str()) {
				return Err(at(&format!("duplicate export declaration `{k}`")));
			}
		}
		let mut asked = std::collections::HashSet::new();
		for k in &self.needs {
			key("needs", k)?;
			if !asked.insert(k.as_str()) {
				return Err(at(&format!("duplicate needs declaration `{k}`")));
			}
			if self.provide.contains(k) {
				return Err(at(&format!(
					"`{k}` is provided here, so it is not also needed from outside"
				)));
			}
		}
		for (field, keys) in [
			("selftest", &self.selftest),
			("integration", &self.integration),
		] {
			if let Some(k) = keys {
				// The guard on an empty `provide` is required, not forgotten: `harness`
				// in ~/dev/sys/builtin declares `selftest: "harness.selftest"` with no
				// document-level `provide`, because its Lua entry is what provides.
				// Dropping the guard would refuse a manifest that is already written.
				if !self.provide.is_empty() && !self.provide.contains(k) {
					return Err(at(&format!(
						"`{field}` names `{k}`, which this cartridge does not provide"
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

/// The layout rule, stated once: **a nested cartridge is a direct child
/// directory of its parent's folder holding a `cartridge.json`.** One level and
/// no deeper — `outer/inner/cartridge.json` is nested in `outer`, while
/// `outer/vendor/inner/cartridge.json` is nested in `outer/vendor` and is
/// hidden from `outer` until `outer/vendor` passes its key on. Nothing here
/// descends, because descending would make a grandchild's subtree visible to a
/// grandparent that never named it, which is the rule's whole point.
///
/// Sorted, so what a subtree offers does not depend on directory order.
pub(super) fn nested(root: &Path) -> Vec<PathBuf> {
	let Ok(dir) = std::fs::read_dir(root) else {
		return Vec::new();
	};
	let mut manifests: Vec<PathBuf> = dir
		.flatten()
		.map(|e| e.path().join(MANIFEST))
		.filter(|m| m.is_file())
		.collect();
	manifests.sort();
	manifests
}

impl Cartridge {
	/// What this cartridge's own subtree offers it: every nested cartridge's
	/// `provide`, plus whatever each of those passes on from deeper in. One
	/// level at a time, because a nested cartridge's own subtree is hidden from
	/// everything outside it — including from this cartridge's parent.
	pub fn offered(root: &Path) -> Result<Vec<String>> {
		let mut keys = Vec::new();
		for manifest in nested(root) {
			// The child is read as a document and not resolved: what it offers is a
			// declaration, and a child whose Lua entry is missing has still made it.
			let child = Self::document(&manifest)?;
			keys.extend(child.provide);
			keys.extend(child.export);
		}
		Ok(keys)
	}

	/// Every re-export names a key the subtree actually offers. This is the
	/// second naming the nesting rule costs, and the check that it was paid.
	/// Crate-visible so the ledger's read runs it too: one validation, one
	/// verdict, so the two listings refuse the same trees.
	pub(crate) fn passed_on(&self, root: &Path, manifest: &Path) -> Result<()> {
		if self.export.is_empty() {
			return Ok(());
		}
		let offered = Self::offered(root)?;
		for key in &self.export {
			if !offered.contains(key) {
				return Err(Error::document(
					manifest,
					format!("`{key}` is passed on, but nothing inside this cartridge offers it"),
				));
			}
		}
		Ok(())
	}
}

impl Grant {
	fn check(&self, at: &dyn Fn(&str) -> Error) -> Result<()> {
		for (field, paths) in [("read", &self.read), ("write", &self.write)] {
			for p in paths {
				let path = Path::new(p);
				// A path is blank on the same terms a key is: `trim()` on both, so
				// `"   "` is refused in a grant exactly as it is in `provide`.
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

/// What the document says the *entry* is and declares, with every file it names
/// resolved. A bare Lua entry has no document, so it declares nothing and the
/// entry stays the only source.
///
/// `export` and `grant` are not here: they are read by the listing, which must
/// be able to read them off a document whose files are not all in place, and
/// that is [`document`]'s job rather than this one's.
pub(crate) struct Declared {
	pub(crate) entry: PathBuf,
	pub(crate) name: String,
	pub(crate) sources: Vec<PathBuf>,
	pub(crate) provide: Vec<String>,
	pub(crate) needs: Vec<String>,
	pub(crate) config: serde_json::Value,
	/// What this cartridge contributes to the configuration. Travels with the
	/// document so the value a component is handed is already settled against
	/// it, wherever the component was loaded from.
	pub(crate) settings: crate::settings::Specs,
}

/// What a profile entry's document declares, read from the document and its own
/// subtree alone. Narrower than [`resolve`] on purpose: no Lua entry is
/// resolved and no declared `ui` file is looked for, so an `Err` here means the
/// **document** could not be read — never that the tree around it is
/// incomplete, and never that the cartridge asked for nothing.
///
/// A bare `.lua` path has no document, so it declares nothing and that is not
/// an error.
pub(crate) fn document(path: &Path) -> Result<(Vec<String>, Grant)> {
	let path = normalize(&classify(path));
	if path.extension().is_some_and(|ext| ext == "lua") {
		return Ok((Vec::new(), Grant::default()));
	}
	let cartridge = Cartridge::document(&path)?;
	let root = path
		.parent()
		.ok_or_else(|| Error::document(&path, "no cartridge folder"))?;
	cartridge.passed_on(root, &path)?;
	Ok((cartridge.export, cartridge.grant))
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
			provide: Vec::new(),
			needs: Vec::new(),
			config: serde_json::Value::Null,
			settings: Default::default(),
		});
	}
	let (manifest, entry) = Cartridge::read(&path)?;
	let root = path.parent().expect("manifest has a folder");
	manifest.passed_on(root, &path)?;
	let mut sources = vec![path.clone(), entry.clone()];
	if let Some(ui) = &manifest.ui {
		let ui = root.join(ui);
		sources.push(ui.canonicalize().map_err(|e| Error::file(&ui, e))?);
	}
	Ok(Declared {
		entry,
		name: manifest.name,
		sources,
		provide: manifest.provide,
		needs: manifest.needs,
		config: manifest.config,
		settings: manifest.settings,
	})
}
