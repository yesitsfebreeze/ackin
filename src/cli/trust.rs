use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;

use cartridge::loader::MANIFEST;
use cartridge::{trust, Error, Result};

use super::{fail, FAILED};

pub(crate) fn run(path: Option<&Path>, revoke: bool, list: bool, ask: bool) -> Result<ExitCode> {
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
	let dir = match path {
		Some(path) => path.to_path_buf(),
		None => {
			let dir = cartridge::loader::root();
			// A stray `cartridge trust` outside any project must not hash a home directory.
			if !dir.join(".cartridge/init.lua").is_file() && !dir.join(MANIFEST).is_file() {
				return Err(Error::Argument(format!(
					"{}: neither a project (.cartridge/init.lua) nor a cartridge ({MANIFEST})",
					dir.display()
				)));
			}
			dir
		}
	};
	if revoke {
		return Ok(match trust::revoke(&dir)? {
			1 => {
				println!("revoked {}", dir.display());
				ExitCode::SUCCESS
			}
			0 => fail(FAILED, format!("{} was not trusted", dir.display())),
			gone => {
				println!(
					"revoked {} and {} record(s) beneath it",
					dir.display(),
					gone - 1
				);
				ExitCode::SUCCESS
			}
		});
	}
	if ask {
		return Ok(match self::ask(&dir)? {
			true => ExitCode::SUCCESS,
			false => fail(FAILED, format!("{} is not trusted", dir.display())),
		});
	}
	let record = trust::record(&dir)?;
	println!(
		"trusted {}: {} files",
		record.project.display(),
		record.files.len()
	);
	Ok(ExitCode::SUCCESS)
}

/// Prompts on stderr, never stdout, so a command's stdout stays its answer.
/// Without a terminal, nothing is asked and the answer is no.
pub(crate) fn ask(dir: &Path) -> Result<bool> {
	let pending = trust::pending(dir)?;
	let terminal = std::io::stdin().is_terminal() && std::io::stderr().is_terminal();
	if cartridge::settings::yolo() {
		if !terminal {
			return Ok(pending.is_empty());
		}
		let project = dir.canonicalize().map_err(|e| Error::file(dir, e))?;
		let mut err = std::io::stderr();
		if !pending.is_empty() {
			let _ = writeln!(
				err,
				"{} has {} untrusted or changed file(s):",
				project.display(),
				pending.len()
			);
			for file in pending.iter().take(10) {
				let _ = writeln!(
					err,
					"  {}",
					file.strip_prefix(&project).unwrap_or(file).display()
				);
			}
			if pending.len() > 10 {
				let _ = writeln!(err, "  … and {} more", pending.len() - 10);
			}
			let _ = writeln!(err);
		}
		// This one answer both records trust and unlocks yolo's bypass.
		let _ = write!(
			err,
			"YOLO mode: trust {} and let every cartridge and tool run without further prompts? [Y/n] ",
			project.display()
		);
		let _ = err.flush();
		let mut answer = String::new();
		std::io::stdin()
			.lock()
			.read_line(&mut answer)
			.map_err(|e| Error::file("stdin", e))?;
		if !matches!(
			answer.trim().to_ascii_lowercase().as_str(),
			"" | "y" | "yes"
		) {
			return Ok(false);
		}
		let record = trust::record(&project)?;
		let _ = writeln!(
			err,
			"YOLO: trusted {} and running with everything allowed ({} files)",
			record.project.display(),
			record.files.len()
		);
		return Ok(true);
	}
	if pending.is_empty() {
		return Ok(true);
	}
	if !terminal {
		return Ok(false);
	}
	let project = dir.canonicalize().map_err(|e| Error::file(dir, e))?;
	let mut err = std::io::stderr();
	let _ = writeln!(
		err,
		"{} has {} untrusted or changed file(s); trusting lets them run with your permissions:",
		project.display(),
		pending.len()
	);
	for file in pending.iter().take(10) {
		let _ = writeln!(
			err,
			"  {}",
			file.strip_prefix(&project).unwrap_or(file).display()
		);
	}
	if pending.len() > 10 {
		let _ = writeln!(err, "  … and {} more", pending.len() - 10);
	}
	let _ = write!(err, "Trust {}? [y/N] ", project.display());
	let _ = err.flush();
	let mut answer = String::new();
	std::io::stdin()
		.lock()
		.read_line(&mut answer)
		.map_err(|e| Error::file("stdin", e))?;
	if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
		return Ok(false);
	}
	let record = trust::record(&project)?;
	let _ = writeln!(
		err,
		"trusted {}: {} files",
		record.project.display(),
		record.files.len()
	);
	Ok(true)
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/cli/trust.rs"]
mod tests;
