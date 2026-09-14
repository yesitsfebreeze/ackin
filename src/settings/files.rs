//! The configuration files: the machine's under the user's home, the
//! project's beside its profile, and which of the two settled a key.

use std::path::{Path, PathBuf};

use serde_json::{json, Value as Json};

use super::{get, merge};
use crate::error::{Error, Result};

/// The global configuration file: `$CARTRIDGE_HOME/config.lua`, or
/// `~/.cartridge/config.lua`. One per machine, under the user's own home, so a
/// preference follows the person across projects without being committed to
/// any of them.
pub fn global_path() -> Result<PathBuf> {
	Ok(crate::trust::home()?.join("config.lua"))
}

/// The project configuration file, beside the profile that composes it.
pub fn project_path(profile: &Path) -> PathBuf {
	profile.join("config.lua")
}

/// Evaluate one configuration file into data. A missing file is an empty
/// table, not an error: not having a global configuration is the normal case.
///
/// Its own Lua, not the host's: settings are read before a host exists — by the
/// CLI listing them, and by a cartridge that has no host at all — and reading
/// a table of numbers must not depend on a composition being up.
pub fn read(path: &Path) -> Result<Json> {
	if !path.is_file() {
		return Ok(json!({}));
	}
	match crate::lua::evaluate::<Json>(path) {
		Ok(value) if value.is_object() => Ok(value),
		Ok(_) => Err(Error::Settings(format!(
			"{}: must return a table",
			path.display()
		))),
		// The chunk name carries the file for Lua errors; anything else — a
		// trust refusal — keeps its own words.
		Err(Error::Lua(e)) => Err(Error::Settings(format!("{}: {e}", path.display()))),
		Err(other) => Err(other),
	}
}

/// The global file laid under the project file: the configuration as the files
/// on this machine state it, before any declaration fills it in.
pub fn layers(profile: &Path) -> Result<Json> {
	let mut out = match global_path() {
		Ok(path) => read(&path)?,
		Err(e) => return Err(e),
	};
	merge(&mut out, read(&project_path(profile))?);
	Ok(out)
}

/// Which file settled a key, for the listing. Not a layer the merge knows
/// about — it recomputes the answer by asking each file in turn.
pub fn source(profile: &Path, key: &str, settled: &Json, declared: &Json) -> &'static str {
	let global = global_path()
		.ok()
		.and_then(|p| read(&p).ok())
		.is_some_and(|v| get(&v, key).is_some());
	let project = read(&project_path(profile))
		.ok()
		.is_some_and(|v| get(&v, key).is_some());
	match (project, global) {
		(true, _) => "project",
		(false, true) => "global",
		// Neither file names it, yet it is not what the declaration says: the
		// profile entry set it in `init.lua`, which composes rather than
		// configures and so has no line in either file to point at.
		(false, false) if settled != declared => "profile",
		(false, false) => "default",
	}
}
