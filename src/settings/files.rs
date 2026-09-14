//! The configuration files: the machine's under the user's home, the
//! project's beside its descriptor, and which of the two settled a key.

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

/// The project configuration file, beside the descriptor that composes it.
pub fn project_path(descriptor: &Path) -> PathBuf {
	descriptor.join("config.lua")
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
pub fn layers(descriptor: &Path) -> Result<Json> {
	let mut out = read(&global_path()?)?;
	merge(&mut out, read(&project_path(descriptor))?);
	Ok(out)
}

/// Both configuration files, read once, so a listing asks them per key without
/// verifying trust and building two Lua states per row. Not a layer the merge
/// knows about.
pub struct Sources {
	global: Json,
	project: Json,
}

impl Sources {
	/// A file that will not read answers for no key, as it did when read per key.
	pub fn read(descriptor: &Path) -> Self {
		Sources {
			global: global_path()
				.ok()
				.and_then(|path| read(&path).ok())
				.unwrap_or_else(|| json!({})),
			project: read(&project_path(descriptor)).unwrap_or_else(|_| json!({})),
		}
	}

	/// Which file settled `key`, for the listing.
	pub fn of(&self, key: &str, settled: &Json, declared: &Json) -> &'static str {
		match (
			get(&self.project, key).is_some(),
			get(&self.global, key).is_some(),
		) {
			(true, _) => "project",
			(false, true) => "global",
			// Neither file names it, yet it is not what the declaration says:
			// the descriptor entry set it in `init.lua`, which composes rather
			// than configures and so has no line in either file to point at.
			(false, false) if settled != declared => "descriptor",
			(false, false) => "default",
		}
	}
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/settings/files.rs"]
mod tests;
