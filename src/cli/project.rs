//! Where a command runs: the cartridge root and the profile, decided once
//! before anything is loaded, and the host settings settled against them.

use std::path::{Path, PathBuf};

use cartridge::loader;
use cartridge::{Error, Result};

/// The project a command is bound to.
pub(crate) struct Project {
	pub(crate) dir: PathBuf,
	pub(crate) profile: PathBuf,
}

/// Bind this invocation to its project.
///
/// `--dir` is the last path read against the directory this process was
/// started in; everything after is read against the project. A runtime is
/// bound to its project and dies with the terminal that started it, so the
/// project is decided once, here, before anything is loaded — and a command
/// typed in a subdirectory joins the runtime already serving that project
/// instead of starting a second one beside it.
pub(crate) fn locate(dir: Option<PathBuf>) -> Result<Project> {
	// Cartridges are found under the project itself unless `--dir` names another root.
	let dir = dir.map(|dir| absolute(&dir));
	let root = loader::root();
	let dir = dir.unwrap_or_else(|| root.clone());
	std::env::set_current_dir(&root).map_err(|e| Error::file(&root, e))?;
	let profile = loader::profile();
	// An interactive command offers to trust its project instead of only refusing.
	use std::io::IsTerminal;
	if std::io::stdin().is_terminal() && root.join(&profile).join("init.lua").is_file() {
		super::trust::ask(&root)?;
	}
	// Here, because `settle` turns an error from these files into a warning.
	for name in ["init.lua", "config.lua"] {
		let file = root.join(&profile).join(name);
		if file.is_file() {
			cartridge::trust::verify(&file)?;
		}
	}
	cartridge::settings::settle(&profile);
	Ok(Project { dir, profile })
}

/// A path pinned to the directory this process started in, lexically: the
/// answer must not change once the working directory moves to the project root,
/// and a directory that does not exist yet still has an address.
fn absolute(path: &Path) -> PathBuf {
	std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}
