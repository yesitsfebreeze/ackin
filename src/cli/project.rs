use std::path::{Path, PathBuf};

use cartridge::loader;
use cartridge::{Error, Result};

pub(crate) struct Project {
	pub(crate) dir: PathBuf,
	pub(crate) descriptor: PathBuf,
}

pub(crate) fn locate(dir: Option<PathBuf>) -> Result<Project> {
	let dir = dir.map(|dir| absolute(&dir));
	let root = loader::root();
	let dir = dir.unwrap_or_else(|| root.clone());
	std::env::set_current_dir(&root).map_err(|e| Error::file(&root, e))?;
	let descriptor = loader::descriptor();
	use std::io::IsTerminal;
	if std::io::stdin().is_terminal() && root.join(&descriptor).join("init.lua").is_file() {
		super::trust::ask(&root)?;
	}
	// Must run before settle, which turns an error from these files into a warning.
	for name in ["init.lua", "config.lua"] {
		let file = root.join(&descriptor).join(name);
		if file.is_file() {
			cartridge::trust::verify(&file)?;
		}
	}
	cartridge::settings::settle(&descriptor);
	Ok(Project { dir, descriptor })
}

fn absolute(path: &Path) -> PathBuf {
	std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}
