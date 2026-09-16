use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

use super::normalize;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cartridge {
	pub name: String,
	pub entry: String,
	pub description: Option<String>,
	#[serde(default)]
	pub commands: std::collections::BTreeMap<String, Command>,
	pub binary: Option<String>,
	#[serde(default)]
	pub contracts: Vec<String>,
	pub setup: Option<String>,
	pub doctor: Option<String>,
	pub source: Option<String>,
	#[serde(default)]
	pub config: serde_json::Value,
	#[serde(default)]
	pub settings: crate::settings::Specs,
	#[serde(default)]
	pub events: std::collections::BTreeMap<String, Event>,
	#[serde(default)]
	pub needs: Vec<String>,
	#[serde(default)]
	pub listen: Vec<String>,
	#[serde(default)]
	pub grant: Grant,
	/// A dotted config key naming a `host:port` the host binds once and keeps
	/// across this cartridge's restarts; the node inherits it as
	/// `CARTRIDGE_LISTENER_FD`, so a connection made during a restart waits in
	/// the backlog instead of being refused.
	#[serde(default)]
	pub listener: Option<String>,
}

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

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
	pub argv: Vec<String>,
	pub cwd: String,
	pub description: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
	#[serde(default)]
	pub read: Vec<String>,
	#[serde(default)]
	pub write: Vec<String>,
	#[serde(default)]
	pub net: Vec<String>,
	#[serde(default)]
	pub exec: Vec<String>,
	#[serde(default)]
	pub env: Vec<String>,
	/// The microphone. macOS hands a denied capture silence rather than an
	/// error, so a cartridge that records without this reads zeroes forever.
	#[serde(default)]
	pub audio: bool,
}

impl Cartridge {
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

	pub fn read(manifest: &Path) -> Result<(Cartridge, PathBuf)> {
		let (cartridge, entry, _) = Self::read_verified(manifest)?;
		Ok((cartridge, entry))
	}

	/// [`Cartridge::read`], plus the entry's own verified bytes: [`resolve`]
	/// needs the digest of exactly what was checked here, not a fresh,
	/// unverified read of the entry taken after the fact.
	fn read_verified(manifest: &Path) -> Result<(Cartridge, PathBuf, Vec<u8>)> {
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

pub const MANIFEST: &str = "cartridge.json";

impl Grant {
	fn check(&self, at: &dyn Fn(&str) -> Error) -> Result<()> {
		for (field, paths) in [("read", &self.read), ("write", &self.write)] {
			for p in paths {
				let path = Path::new(p);
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

pub(super) fn classify(path: &Path) -> PathBuf {
	if path.extension().is_some_and(|ext| ext == "lua")
		|| path.file_name().is_some_and(|name| name == MANIFEST)
	{
		return path.to_path_buf();
	}
	path.join(MANIFEST)
}

#[derive(Debug)]
pub(crate) struct Declared {
	pub(crate) entry: PathBuf,
	pub(crate) entry_sha256: String,
	pub(crate) name: String,
	pub(crate) sources: Vec<PathBuf>,
	pub(crate) events: std::collections::BTreeMap<String, Event>,
	pub(crate) needs: Vec<String>,
	pub(crate) listen: Vec<String>,
	pub(crate) config: serde_json::Value,
	pub(crate) settings: crate::settings::Specs,
	pub(crate) grant: Grant,
	pub(crate) listener: Option<String>,
}

pub fn is_bare_name(name: &str) -> bool {
	let mut parts = Path::new(name).components();
	!name.is_empty()
		&& !name.contains('\0')
		&& matches!(parts.next(), Some(std::path::Component::Normal(_)))
		&& parts.next().is_none()
}

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
			listener: None,
		});
	}
	let (manifest, entry, entry_bytes) = Cartridge::read_verified(&path)?;
	// A built native module is not a source: `cargo test` rewrites it too, and
	// only `cartridge reload <id>` restarts a cartridge onto a new build.
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
		listener: manifest.listener,
	})
}
