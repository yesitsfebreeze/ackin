//! Project trust: the SHA-256 a person approved for every file the base
//! evaluates or takes authority from — project `*.lua` and every
//! `cartridge.json`, because the manifest carries the grant. The digest is the
//! file's bytes alone, so `shasum -a 256 <file>` reproduces any stored value.
//!
//! A node reads no store: the base hands it the digest of the entry it
//! verified, and the node refuses an entry whose bytes no longer match.

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

/// `$CARTRIDGE_HOME`, else `~/.cartridge`: the store, the global `config.lua`
/// and the catalog. Creates nothing. Refuses a home that is not absolute: a
/// relative one resolves against the project, making the project "the
/// person's own" and putting the store inside it.
pub fn home() -> Result<PathBuf> {
	let home = match std::env::var_os("CARTRIDGE_HOME") {
		Some(home) => PathBuf::from(home),
		None => std::env::var_os("HOME")
			.map_or_else(PathBuf::new, |home| PathBuf::from(home).join(".cartridge")),
	};
	match home.is_absolute() {
		true => Ok(home),
		false => Err(Error::Descriptor(format!(
			"the cartridge home `{}` is not an absolute path; set CARTRIDGE_HOME or HOME",
			home.display()
		))),
	}
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

/// The bytes' SHA-256, lowercase hex.
pub fn digest_bytes(bytes: &[u8]) -> String {
	hex(&Sha256::digest(bytes))
}

/// The file's SHA-256, lowercase hex.
pub fn digest(path: &Path) -> Result<String> {
	Ok(digest_bytes(
		&std::fs::read(path).map_err(|e| Error::file(path, e))?,
	))
}

/// The project a refusal should name: the nearest ancestor holding a descriptor,
/// by the rule of [`crate::loader::root`], else the file's own folder.
fn nearest_project(file: &Path) -> PathBuf {
	file.ancestors()
		.skip(1)
		.find(|dir| dir.join(".cartridge").join("init.lua").is_file())
		.or_else(|| file.parent())
		.unwrap_or(file)
		.to_path_buf()
}

/// Whether the file is the person's own: under the home as written or as
/// resolved. As-written matters because a dotfiles setup symlinks its config
/// elsewhere, and the paths here are spelled by the base itself.
fn own(path: &Path, file: &Path) -> Result<bool> {
	let home = home()?;
	Ok(
		path.starts_with(&home)
			|| file.starts_with(home.canonicalize().as_deref().unwrap_or(&home)),
	)
}

/// Refuse a file no trusted directory above it recorded with its current hash.
/// Any ancestor's record authorises, so a nested record never shadows a fresher
/// outer one.
pub fn verify(path: &Path) -> Result<()> {
	let file = path.canonicalize().map_err(|e| Error::file(path, e))?;
	if own(path, &file)? {
		return Ok(());
	}
	checked(&file, &digest(&file)?)
}

/// Whether the file sits under a directory the walk never records, between the
/// project root and the file, so `cartridge trust` cannot fix its refusal.
/// Directories above the project — a tempdir's `.tmp…`, a home's dotfiles —
/// are not project source.
fn unwalked(project: &Path, file: &Path) -> bool {
	file.strip_prefix(project).is_ok_and(|rel| {
		rel.ancestors().skip(1).any(|dir| {
			let name = dir
				.file_name()
				.map_or(String::new(), |n| n.to_string_lossy().into_owned());
			["target", "node_modules", ".git"].contains(&name.as_str())
				|| (name.starts_with('.') && name != ".cartridge")
		})
	})
}

/// The canonical file against every record above it.
fn checked(file: &Path, digest: &str) -> Result<()> {
	let mut refusal = None;
	for dir in file.ancestors().skip(1) {
		let at = record_path(dir)?;
		let Ok(text) = std::fs::read_to_string(&at) else {
			continue;
		};
		let record: Record = serde_json::from_str(&text)
			.map_err(|e| Error::Settings(format!("{}: {e}", at.display())))?;
		let why = match record.files.get(file) {
			Some(known) if known == digest => return Ok(()),
			Some(_) => "has changed since it was trusted",
			None => "is not in this project's trust record",
		};
		refusal.get_or_insert((record.project, why));
	}
	let never_records =
		"is under a directory `cartridge trust` never records; move it or link its folder";
	let project = refusal
		.as_ref()
		.map_or_else(|| nearest_project(file), |(project, _)| project.clone());
	let why = match refusal {
		Some((_, why)) if !unwalked(&project, file) => why,
		_ if unwalked(&project, file) => never_records,
		_ => "is in no trusted project",
	};
	Err(Error::Untrusted {
		project,
		file: file.to_path_buf(),
		why,
	})
}

/// A file's text, once it is trusted — hashed once, for the bytes it returns.
pub fn read(path: &Path) -> Result<String> {
	let bytes = std::fs::read(path).map_err(|e| Error::file(path, e))?;
	let file = path.canonicalize().map_err(|e| Error::file(path, e))?;
	if !own(path, &file)? {
		checked(&file, &digest_bytes(&bytes))?;
	}
	String::from_utf8(bytes).map_err(|e| Error::file(path, std::io::Error::other(e)))
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
	let mut found = Vec::new();
	collect(
		&dir.canonicalize().map_err(|e| Error::file(dir, e))?,
		&mut found,
	)?;
	record_files(dir, &found)
}

/// Approve exactly the named files as they are now, under one directory,
/// replacing any earlier record of it. The caller says what it approves:
/// setup records what it wrote and chose, not what a tree happens to hold.
/// Every file must live under `dir`; a missing one is an error, not a skip.
pub fn record_files(dir: &Path, files: &[PathBuf]) -> Result<Record> {
	let project = dir.canonicalize().map_err(|e| Error::file(dir, e))?;
	let files = files
		.iter()
		.map(|file| {
			let canonical = file.canonicalize().map_err(|e| Error::file(file, e))?;
			if !canonical.starts_with(&project) {
				return Err(Error::Descriptor(format!(
					"{} is not under {}",
					canonical.display(),
					project.display()
				)));
			}
			let digest = digest(&canonical)?;
			Ok((canonical, digest))
		})
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

/// The canonical spelling of a path that may no longer exist: the deepest
/// existing ancestor resolved, the rest joined on, so a deleted `/var/…`
/// still matches the record `/private/var/…`.
fn resolved(dir: &Path) -> Result<PathBuf> {
	if let Ok(canonical) = dir.canonicalize() {
		return Ok(canonical);
	}
	let mut rest = Vec::new();
	let mut existing = dir.to_path_buf();
	while let Some(name) = existing.file_name() {
		match existing.canonicalize() {
			Ok(canonical) => {
				let mut out = canonical;
				for part in rest.iter().rev() {
					out.push(part);
				}
				return Ok(out);
			}
			Err(_) => {
				rest.push(name.to_owned());
				existing.pop();
			}
		}
	}
	std::path::absolute(dir).map_err(|e| Error::file(dir, e))
}

/// Forget a directory and every record beneath it, without needing the
/// directory on disk — a deleted project is untrusted by its absolute
/// spelling. Zero when nothing matched.
pub fn revoke(dir: &Path) -> Result<usize> {
	let want = resolved(dir)?;
	let mut gone = 0;
	for record in list()? {
		if record.project == want || record.project.starts_with(&want) {
			std::fs::remove_file(record_path(&record.project)?)
				.map_err(|e| Error::file(&record.project, e))?;
			gone += 1;
		}
	}
	Ok(gone)
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
#[path = "../../.cartridge/tests/unit/src/trust/tests.rs"]
mod tests;
