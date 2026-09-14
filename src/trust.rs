//! Project trust: the SHA-256 a person approved for every file the base
//! evaluates or takes authority from — project `*.lua` and every
//! `cartridge.json`, because the manifest carries the grant. The digest is the
//! file's bytes alone, so `shasum -a 256 <file>` reproduces any stored value.
//!
//! A node does not verify: it runs inside a grant the base verified, and its
//! sandbox cannot read the store.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

/// One directory, as the person who ran `cartridge trust` approved it.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Record {
	pub project: PathBuf,
	/// Seconds since the epoch.
	pub trusted_at: u64,
	/// Canonical path to the file's SHA-256, lowercase hex.
	pub files: BTreeMap<PathBuf, String>,
}

fn hex(bytes: &[u8]) -> String {
	bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `$CARTRIDGE_HOME`, else `~/.cartridge`. Creates nothing: every gated read
/// asks for it.
pub fn home() -> Result<PathBuf> {
	if let Some(home) = std::env::var_os("CARTRIDGE_HOME") {
		return Ok(PathBuf::from(home));
	}
	std::env::var_os("HOME")
		.map(|home| PathBuf::from(home).join(".cartridge"))
		.ok_or_else(|| {
			Error::Profile("no CARTRIDGE_HOME and no HOME: the trust store has no home".into())
		})
}

fn store() -> Result<PathBuf> {
	Ok(home()?.join("trust"))
}

fn ensure_store() -> Result<PathBuf> {
	use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
	let store = store()?;
	std::fs::DirBuilder::new()
		.recursive(true)
		.mode(0o700)
		.create(&store)
		.map_err(|e| Error::file(&store, e))?;
	// `DirBuilder` leaves an existing, wider store as it is.
	std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o700))
		.map_err(|e| Error::file(&store, e))?;
	Ok(store)
}

/// The record for a canonical directory.
fn record_path(dir: &Path) -> Result<PathBuf> {
	let key = hex(&Sha256::digest(dir.as_os_str().as_encoded_bytes()));
	Ok(store()?.join(format!("{key}.json")))
}

/// The file's SHA-256, lowercase hex.
pub fn digest(path: &Path) -> Result<String> {
	let bytes = std::fs::read(path).map_err(|e| Error::file(path, e))?;
	Ok(hex(&Sha256::digest(bytes)))
}

/// The project a refusal should name: the nearest ancestor holding a profile,
/// by the rule of [`crate::loader::root`], else the file's own folder.
fn nearest_project(file: &Path) -> PathBuf {
	file.ancestors()
		.skip(1)
		.find(|dir| dir.join(".cartridge").join("init.lua").is_file())
		.or_else(|| file.parent())
		.unwrap_or(file)
		.to_path_buf()
}

/// Refuse a file no trusted directory above it recorded with its current hash.
/// Any ancestor's record authorises, so a nested record never shadows a fresher
/// outer one.
pub fn verify(path: &Path) -> Result<()> {
	let file = path.canonicalize().map_err(|e| Error::file(path, e))?;
	let home = home()?;
	if file.starts_with(home.canonicalize().unwrap_or(home)) {
		return Ok(());
	}
	let digest = digest(&file)?;
	let mut refusal = None;
	for dir in file.ancestors().skip(1) {
		let at = record_path(dir)?;
		let Ok(text) = std::fs::read_to_string(&at) else {
			continue;
		};
		let record: Record = serde_json::from_str(&text)
			.map_err(|e| Error::Settings(format!("{}: {e}", at.display())))?;
		let why = match record.files.get(&file) {
			Some(known) if *known == digest => return Ok(()),
			Some(_) => "has changed since it was trusted",
			None => "is not in this project's trust record",
		};
		refusal.get_or_insert((record.project, why));
	}
	let (project, why) =
		refusal.unwrap_or_else(|| (nearest_project(&file), "is in no trusted project"));
	Err(Error::Untrusted { project, file, why })
}

/// A file's text, once it is trusted.
pub fn read(path: &Path) -> Result<String> {
	verify(path)?;
	std::fs::read_to_string(path).map_err(|e| Error::file(path, e))
}

/// Every file the trust set names, lexically: nothing is evaluated, no link is
/// followed, and build output is not project source.
fn collect(dir: &Path, into: &mut Vec<PathBuf>) -> Result<()> {
	for entry in std::fs::read_dir(dir).map_err(|e| Error::file(dir, e))? {
		let entry = entry.map_err(|e| Error::file(dir, e))?;
		let kind = entry
			.file_type()
			.map_err(|e| Error::file(entry.path(), e))?;
		let name = entry.file_name();
		let name = name.to_string_lossy();
		if kind.is_dir() {
			let skipped = ["target", "node_modules", ".git"].contains(&&*name)
				|| (name.starts_with('.') && name != ".cartridge");
			if !skipped {
				collect(&entry.path(), into)?;
			}
		} else if kind.is_file() && (name == crate::loader::MANIFEST || name.ends_with(".lua")) {
			into.push(entry.path());
		}
	}
	Ok(())
}

/// Approve a directory as it is now, replacing any earlier record of it.
pub fn record(dir: &Path) -> Result<Record> {
	let project = dir.canonicalize().map_err(|e| Error::file(dir, e))?;
	let mut found = Vec::new();
	collect(&project, &mut found)?;
	let files = found
		.into_iter()
		.map(|file| Ok((file.clone(), digest(&file)?)))
		.collect::<Result<_>>()?;
	let trusted_at = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map_or(0, |d| d.as_secs());
	let record = Record {
		project,
		trusted_at,
		files,
	};
	ensure_store()?;
	crate::host::socket::write_private(
		&record_path(&record.project)?,
		&serde_json::to_string_pretty(&record)?,
	)?;
	Ok(record)
}

/// The files under a directory `verify` would refuse right now.
pub fn pending(dir: &Path) -> Result<Vec<PathBuf>> {
	let project = dir.canonicalize().map_err(|e| Error::file(dir, e))?;
	let mut found = Vec::new();
	collect(&project, &mut found)?;
	found.retain(|file| verify(file).is_err());
	Ok(found)
}

/// Forget a directory. `false` when it was not trusted.
pub fn revoke(dir: &Path) -> Result<bool> {
	let dir = dir.canonicalize().map_err(|e| Error::file(dir, e))?;
	let at = record_path(&dir)?;
	match std::fs::remove_file(&at) {
		Ok(()) => Ok(true),
		Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
		Err(e) => Err(Error::file(&at, e)),
	}
}

/// Every directory this machine trusts, by path.
pub fn list() -> Result<Vec<Record>> {
	let store = store()?;
	let entries = match std::fs::read_dir(&store) {
		Ok(entries) => entries,
		Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
		Err(e) => return Err(Error::file(&store, e)),
	};
	let mut records: Vec<Record> = entries
		.filter_map(|entry| std::fs::read_to_string(entry.ok()?.path()).ok())
		.filter_map(|text| serde_json::from_str(&text).ok())
		.collect();
	records.sort_by(|a, b| a.project.cmp(&b.project));
	Ok(records)
}

#[cfg(test)]
#[path = "../.cartridge/tests/unit/src/trust/tests.rs"]
mod tests;
