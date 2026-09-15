use std::path::{Path, PathBuf};

use serde_json::{json, Value as Json};

use super::{get, merge};
use crate::error::{Error, Result};

pub fn global_path() -> Result<PathBuf> {
	Ok(crate::trust::home()?.join("config.lua"))
}

pub fn project_path(descriptor: &Path) -> PathBuf {
	descriptor.join("config.lua")
}

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
		Err(Error::Lua(e)) => Err(Error::Settings(format!("{}: {e}", path.display()))),
		Err(other) => Err(other),
	}
}

pub fn layers(descriptor: &Path) -> Result<Json> {
	let mut out = read(&global_path()?)?;
	merge(&mut out, read(&project_path(descriptor))?);
	Ok(out)
}

pub struct Sources {
	global: Json,
	project: Json,
}

impl Sources {
	pub fn read(descriptor: &Path) -> Self {
		Sources {
			global: global_path()
				.ok()
				.and_then(|path| read(&path).ok())
				.unwrap_or_else(|| json!({})),
			project: read(&project_path(descriptor)).unwrap_or_else(|_| json!({})),
		}
	}

	pub fn of(&self, key: &str, settled: &Json, declared: &Json) -> &'static str {
		match (
			get(&self.project, key).is_some(),
			get(&self.global, key).is_some(),
		) {
			(true, _) => "project",
			(false, true) => "global",
			(false, false) if settled != declared => "descriptor",
			(false, false) => "default",
		}
	}
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/settings/files.rs"]
mod tests;
