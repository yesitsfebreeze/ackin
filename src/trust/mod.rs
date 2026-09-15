use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Record {
	pub project: PathBuf,
	pub trusted_at: u64,
	pub files: BTreeMap<PathBuf, String>,
}

fn hex(bytes: &[u8]) -> String {
	bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Refuses a home that is not absolute: a relative one would resolve against
/// the project, making the project "the person's own" and putting the store
/// inside it.
pub fn home() -> Result<PathBuf> {
	let home = match std::env::var_os("CARTRIDGE_HOME") {
		Some(home) => PathBuf::from(home),
		None => crate::sandbox::home().map_or_else(PathBuf::new, |home| home.join(".cartridge")),
	};
	match home.is_absolute() {
		true => Ok(home),
		false => Err(Error::Descriptor(format!(
			"the cartridge home `{}` is not an absolute path; set CARTRIDGE_HOME or {}",
			home.display(),
			crate::sandbox::home_var_names()[0],
		))),
	}
}

fn store() -> Result<PathBuf> {
	Ok(home()?.join("trust"))
}

fn ensure_store() -> Result<PathBuf> {
	let store = store()?;
	let mut builder = std::fs::DirBuilder::new();
	builder.recursive(true);
	#[cfg(unix)]
	{
		use std::os::unix::fs::DirBuilderExt;
		builder.mode(0o700);
	}
	builder.create(&store).map_err(|e| Error::file(&store, e))?;
	// `DirBuilder` leaves an existing, wider store as it is. Windows has no
	// mode to narrow it.
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o700))
			.map_err(|e| Error::file(&store, e))?;
	}
	Ok(store)
}

fn record_path(dir: &Path) -> Result<PathBuf> {
	let key = hex(&Sha256::digest(dir.as_os_str().as_encoded_bytes()));
	Ok(store()?.join(format!("{key}.json")))
}

pub fn digest_bytes(bytes: &[u8]) -> String {
	hex(&Sha256::digest(bytes))
}

pub fn digest(path: &Path) -> Result<String> {
	Ok(digest_bytes(
		&std::fs::read(path).map_err(|e| Error::file(path, e))?,
	))
}

fn nearest_project(file: &Path) -> PathBuf {
	file.ancestors()
		.skip(1)
		.find(|dir| dir.join(".cartridge").join("init.lua").is_file())
		.or_else(|| file.parent())
		.unwrap_or(file)
		.to_path_buf()
}

/// Checked as-written, not just resolved: a dotfiles setup symlinks its
/// config elsewhere, and a file under the home only in its resolved form
/// would wrongly skip the trust check.
fn own(path: &Path, file: &Path) -> Result<bool> {
	let home = home()?;
	Ok(
		path.starts_with(&home)
			|| file.starts_with(home.canonicalize().as_deref().unwrap_or(&home)),
	)
}

/// Hands back the bytes checked — the same read — so a caller never opens the
/// file a second, unchecked time (TOCTOU).
pub fn verify(path: &Path) -> Result<Vec<u8>> {
	let bytes = std::fs::read(path).map_err(|e| Error::file(path, e))?;
	let file = path.canonicalize().map_err(|e| Error::file(path, e))?;
	if !own(path, &file)? {
		checked(&file, &digest_bytes(&bytes))?;
	}
	Ok(bytes)
}

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

fn checked(file: &Path, digest: &str) -> Result<()> {
	let mut refusal = None;
	for dir in file.ancestors().skip(1) {
		let at = record_path(dir)?;
		let Ok(text) = std::fs::read_to_string(&at) else {
			continue;
		};
		// Keeps walking past a corrupt record instead of returning here: a
		// valid outer record must still be able to authorise.
		let Ok(record) = serde_json::from_str::<Record>(&text) else {
			refusal.get_or_insert((dir.to_path_buf(), "has an unreadable trust record"));
			continue;
		};
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

/// Hashed once, for the bytes it returns.
pub fn read(path: &Path) -> Result<String> {
	String::from_utf8(verify(path)?).map_err(|e| Error::file(path, std::io::Error::other(e)))
}

/// No link is followed: a symlink into a directory outside the project must
/// not pull untrusted files into it as project source.
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

pub fn record(dir: &Path) -> Result<Record> {
	let mut found = Vec::new();
	collect(
		&dir.canonicalize().map_err(|e| Error::file(dir, e))?,
		&mut found,
	)?;
	record_files(dir, &found)
}

/// A missing file is an error, not a skip: silently trusting less than the
/// caller approved would be worse than failing loud.
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

pub fn pending(dir: &Path) -> Result<Vec<PathBuf>> {
	let project = dir.canonicalize().map_err(|e| Error::file(dir, e))?;
	let mut found = Vec::new();
	collect(&project, &mut found)?;
	found.retain(|file| verify(file).is_err());
	Ok(found)
}

/// The one implementation of this cleaning: `setup::absolute` calls it rather
/// than keeping its own copy.
pub fn absolute_clean(path: &Path) -> PathBuf {
	let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
	let mut out = PathBuf::new();
	for part in abs.components() {
		match part {
			std::path::Component::CurDir => {}
			std::path::Component::ParentDir => {
				out.pop();
			}
			other => out.push(other),
		}
	}
	out
}

/// The deepest existing ancestor resolved, the rest joined on: a deleted
/// `/var/…` must still match a record stored as `/private/var/…`.
fn resolved(dir: &Path) -> Result<PathBuf> {
	if let Ok(canonical) = dir.canonicalize() {
		return Ok(canonical);
	}
	// Lexically cleaned of `.` and `..`: `absolute` alone normalises only
	// `.`, and a path ending in a literal `..` has no `file_name`, so the walk
	// below would stop before reaching an existing ancestor.
	let cleaned = absolute_clean(dir);
	let mut rest = Vec::new();
	let mut existing = cleaned.clone();
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
	Ok(cleaned)
}

/// Matched on [`project_of`] rather than on a whole [`Record`], so a record
/// too stale to parse is still removed instead of being left behind.
pub fn revoke(dir: &Path) -> Result<usize> {
	let want = resolved(dir)?;
	let mut gone = 0;
	for (path, text) in stored()? {
		if project_of(&text).is_some_and(|project| project.starts_with(&want)) {
			std::fs::remove_file(&path).map_err(|e| Error::file(&path, e))?;
			gone += 1;
		}
	}
	Ok(gone)
}

/// A file that cannot be read is not a record.
fn stored() -> Result<Vec<(PathBuf, String)>> {
	let store = store()?;
	let entries = match std::fs::read_dir(&store) {
		Ok(entries) => entries,
		Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
		Err(e) => return Err(Error::file(&store, e)),
	};
	let mut records = Vec::new();
	for entry in entries {
		let path = entry.map_err(|e| Error::file(&store, e))?.path();
		if let Ok(text) = std::fs::read_to_string(&path) {
			records.push((path, text));
		}
	}
	Ok(records)
}

/// Whose record this is, even when the rest of it no longer parses: a stale
/// shape or a bad digest still names its project, and a record `list` cannot
/// show is still one `revoke` must remove.
fn project_of(text: &str) -> Option<PathBuf> {
	serde_json::from_str::<serde_json::Value>(text)
		.ok()?
		.get("project")?
		.as_str()
		.map(PathBuf::from)
}

pub fn list() -> Result<Vec<Record>> {
	let mut records: Vec<Record> = stored()?
		.iter()
		.filter_map(|(_, text)| serde_json::from_str(text).ok())
		.collect();
	records.sort_by(|a, b| a.project.cmp(&b.project));
	Ok(records)
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/trust/tests.rs"]
mod tests;
