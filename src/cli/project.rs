//! Where a command runs: the cartridge root and the profile, decided once
//! before anything is loaded, and the host settings settled against them.

use std::path::{Path, PathBuf};

use cartridge::loader;
use cartridge::{Error, Result};

/// The project a command is bound to.
pub(crate) struct Project {
	pub(crate) dir: PathBuf,
	pub(crate) profile: PathBuf,
	pub(crate) yolo: bool,
}

/// Bind this invocation to its project.
///
/// `--dir` is the last path read against the directory this process was
/// started in; everything after is read against the project. A runtime is
/// bound to its project and dies with the terminal that started it, so the
/// project is decided once, here, before anything is loaded — and a command
/// typed in a subdirectory joins the runtime already serving that project
/// instead of starting a second one beside it.
pub(crate) fn locate(dir: Option<PathBuf>, yolo: bool) -> Result<Project> {
	let dir = absolute(&dir.unwrap_or_else(loader::builtin));
	let root = loader::root();
	std::env::set_current_dir(&root).map_err(|e| Error::file(&root, e))?;
	// One user profile, one composition: `mcp`, `launch` and `run` differ by the
	// entry point they call into this host, not by the cartridges it loads.
	let profile = loader::profile();
	// Settle the host's own settings against the profile that was just
	// resolved, before anything reads one. Everything downstream — including an
	// SDK child that has only a working directory — then reads this one answer.
	cartridge::settings::settle(&profile);
	Ok(Project { dir, profile, yolo })
}

/// A path pinned to the directory this process started in, lexically: the
/// answer must not change once the working directory moves to the project root,
/// and a directory that does not exist yet still has an address.
fn absolute(path: &Path) -> PathBuf {
	std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The binary a node re-enters: this invocation's own image.
pub(crate) fn exe() -> PathBuf {
	std::env::current_exe().expect("a running binary knows its own image")
}
