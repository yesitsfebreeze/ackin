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
	/// Events this cartridge listens to that prove it; `cartridge verify` sends them.
	#[serde(default)]
	pub contracts: Vec<String>,
	/// An event this cartridge listens to that `cartridge setup` sends after
	/// installing it, to ask what this project must decide and take back the
	/// configuration to write. The exchange is described in `cli/setup.rs`.
	pub setup: Option<String>,
	/// An event this cartridge listens to that `cartridge doctor` sends to ask
	/// whether it is healthy here: `{"ok": bool, "problems": [text]}`.
	pub doctor: Option<String>,
	/// Repository URL or other retrieval reference when source is not installed.
	pub source: Option<String>,
	/// The cartridge's own configuration, carried by the document. The ledger
	/// shape has no descriptor `config.lua` to lay fields on at composition time,
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
/// absent, any payload passes. `timeout_ms` is how long a sender waits for
/// each listener; absent, the host's `event_timeout_ms`.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub schema: Option<serde_json::Value>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub timeout_ms: Option<u64>,
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
	/// Environment variables this cartridge may read, by exact name or by a
	/// `PREFIX*` glob. The base clears a node's environment and passes its own
	/// allow-list; a cartridge whose composition names a credential variable —
	/// a mailbox token, a roster credential — declares the shape of that name
	/// here, so the composition can choose it without the base handing over
	/// every secret the operator's shell held. A bare `*` is refused.
	#[serde(default)]
	pub env: Vec<String>,
}

impl Cartridge {
	/// The document, read and checked against itself and nothing else. Every
	/// failure here is a failure of the document: it will not parse, or it
	/// declares something the format refuses. Nothing on the filesystem around
	/// the cartridge is touched — the Lua entry is not resolved — because a
	/// cartridge declares what it declares whether or not the files it points
	/// at are in place.
	pub fn document(manifest: &Path) -> Result<Cartridge> {
		let source = std::fs::read_to_string(manifest).map_err(|e| Error::file(manifest, e))?;
		Self::parse(manifest, &source)
	}

	/// [`Cartridge::document`]'s checks, over text a caller already has —
	/// [`Cartridge::read`] hands in the bytes `trust::verify` checked, rather
	/// than reading the manifest a second, unchecked time.
	fn parse(manifest: &Path, source: &str) -> Result<Cartridge> {
		let cartridge: Cartridge =
			serde_json::from_str(source).map_err(|e| Error::document(manifest, e.to_string()))?;
		if let Some(binary) = &cartridge.binary {
			if !is_bare_name(binary) {
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
		cartridge.check(manifest)?;
		Ok(cartridge)
	}

	/// The document, plus the files it names: the Lua entry is resolved. An
	/// error from here may be a fact about the tree rather than about the
	/// document — which is why [`Cartridge::document`] exists beside it.
	pub fn read(manifest: &Path) -> Result<(Cartridge, PathBuf)> {
		let (cartridge, entry, _) = Self::read_verified(manifest)?;
		Ok((cartridge, entry))
	}

	/// [`Cartridge::read`], plus the entry's own verified bytes: [`resolve`]
	/// needs the digest of exactly what was checked here, not a fresh,
	/// unverified read of the entry taken after the fact.
	fn read_verified(manifest: &Path) -> Result<(Cartridge, PathBuf, Vec<u8>)> {
		// The grant and the entry take effect from here; listing a document does not.
		let source = crate::trust::verify(manifest)?;
		let source = String::from_utf8(source)
			.map_err(|e| Error::file(manifest, std::io::Error::other(e)))?;
		let cartridge = Self::parse(manifest, &source)?;
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
		let entry_bytes = crate::trust::verify(&entry)?;
		Ok((cartridge, entry, entry_bytes))
	}

	/// Every declaration this document carries, checked without reading anything
	/// else. A key is a nonempty exact string; a wildcard is not a declaration.
	fn check(&self, manifest: &Path) -> Result<()> {
		let at = |what: &str| Error::document(manifest, what);
		let key = |field: &str, k: &String| -> Result<()> {
			let glob = field == "needs" && k.len() > 1 && k.ends_with('*');
			let exact = if glob { &k[..k.len() - 1] } else { k.as_str() };
			if exact.trim().is_empty() || exact.contains('*') || exact.contains('\0') {
				return Err(at(&format!(
					"`{field}` entry `{k}` must be a nonempty exact key"
				)));
			}
			Ok(())
		};
		for (name, event) in &self.events {
			key("events", name)?;
			if event.timeout_ms == Some(0) {
				return Err(at(&format!(
					"`events.{name}.timeout_ms` must be at least 1"
				)));
			}
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
		let named = self.contracts.iter().map(|k| ("contracts", k));
		let named = named.chain(
			[("setup", &self.setup), ("doctor", &self.doctor)]
				.into_iter()
				.filter_map(|(field, k)| k.as_ref().map(|k| (field, k))),
		);
		for (field, k) in named {
			if !self.listen.contains(k) {
				return Err(at(&format!(
					"`{field}` names `{k}`, which this cartridge does not listen to"
				)));
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
#[derive(Debug)]
pub(crate) struct Declared {
	pub(crate) entry: PathBuf,
	/// The entry's SHA-256 as the base verified it, hex — what the node
	/// re-checks before it loads the bytes.
	pub(crate) entry_sha256: String,
	pub(crate) name: String,
	pub(crate) sources: Vec<PathBuf>,
	pub(crate) events: std::collections::BTreeMap<String, Event>,
	pub(crate) needs: Vec<String>,
	pub(crate) listen: Vec<String>,
	pub(crate) config: serde_json::Value,
	pub(crate) settings: crate::settings::Specs,
	pub(crate) grant: Grant,
}

/// Whether a name is one plain path segment: a manifest's `binary` and a
/// cartridge's own name are both names the host joins to a directory, so a
/// separator, `..` or a NUL in one would steer where it lands.
pub fn is_bare_name(name: &str) -> bool {
	let mut parts = Path::new(name).components();
	!name.is_empty()
		&& !name.contains('\0')
		&& matches!(parts.next(), Some(std::path::Component::Normal(_)))
		&& parts.next().is_none()
}

/// The grant a descriptor entry's document declares, without resolving its files.
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
		let bytes = crate::trust::verify(&path)?;
		let name = path
			.file_stem()
			.unwrap_or_default()
			.to_string_lossy()
			.into_owned();
		return Ok(Declared {
			entry: path.clone(),
			entry_sha256: crate::trust::digest_bytes(&bytes),
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
	let (manifest, entry, entry_bytes) = Cartridge::read_verified(&path)?;
	let sources = vec![path.clone(), entry.clone()];
	Ok(Declared {
		entry_sha256: crate::trust::digest_bytes(&entry_bytes),
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
