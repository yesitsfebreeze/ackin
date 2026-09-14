//! `cartridge trust`: record, revoke or list what this machine will run.

use std::path::Path;
use std::process::ExitCode;

use cartridge::loader::MANIFEST;
use cartridge::{trust, Error, Result};

use super::{fail, FAILED};

pub(crate) fn run(path: Option<&Path>, revoke: bool, list: bool) -> Result<ExitCode> {
	if revoke && list {
		return Err(Error::Argument(
			"--revoke and --list ask for different things".into(),
		));
	}
	if list {
		for record in trust::list()? {
			println!("{}  {} files", record.project.display(), record.files.len());
		}
		return Ok(ExitCode::SUCCESS);
	}
	let dir = path.map_or_else(cartridge::loader::root, Path::to_path_buf);
	if revoke {
		return Ok(match trust::revoke(&dir)? {
			true => {
				println!("revoked {}", dir.display());
				ExitCode::SUCCESS
			}
			false => fail(FAILED, format!("{} was not trusted", dir.display())),
		});
	}
	// A stray `cartridge trust` outside any project must not hash a home directory.
	if !dir.join(".cartridge/init.lua").is_file() && !dir.join(MANIFEST).is_file() {
		return Err(Error::Argument(format!(
			"{}: neither a project (.cartridge/init.lua) nor a cartridge ({MANIFEST})",
			dir.display()
		)));
	}
	let record = trust::record(&dir)?;
	println!(
		"trusted {}: {} files",
		record.project.display(),
		record.files.len()
	);
	Ok(ExitCode::SUCCESS)
}
