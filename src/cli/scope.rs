//! Ship the Python browser with the binary; never depend on a checkout path.
use std::path::Path;
use std::process::{Command, ExitCode};

use cartridge::{Error, Result};

const FILES: &[(&str, &str)] = &[
	(
		"edit_session.py",
		include_str!("../../ui/cartridge_scope/edit_session.py"),
	),
	(
		"editor.py",
		include_str!("../../ui/cartridge_scope/editor.py"),
	),
	(
		"filter_chain.py",
		include_str!("../../ui/cartridge_scope/filter_chain.py"),
	),
	(
		"disk_search.py",
		include_str!("../../ui/cartridge_scope/disk_search.py"),
	),
	(
		"search_engine.py",
		include_str!("../../ui/cartridge_scope/search_engine.py"),
	),
	(
		"finder_input.py",
		include_str!("../../ui/cartridge_scope/finder_input.py"),
	),
	(
		"__init__.py",
		include_str!("../../ui/cartridge_scope/__init__.py"),
	),
	(
		"__main__.py",
		include_str!("../../ui/cartridge_scope/__main__.py"),
	),
	("app.py", include_str!("../../ui/cartridge_scope/app.py")),
	(
		"commands.py",
		include_str!("../../ui/cartridge_scope/commands.py"),
	),
	(
		"conversation.py",
		include_str!("../../ui/cartridge_scope/conversation.py"),
	),
	(
		"client.py",
		include_str!("../../ui/cartridge_scope/client.py"),
	),
	(
		"model.py",
		include_str!("../../ui/cartridge_scope/model.py"),
	),
	(
		"detail.py",
		include_str!("../../ui/cartridge_scope/detail.py"),
	),
	(
		"tables.py",
		include_str!("../../ui/cartridge_scope/tables.py"),
	),
	(
		"scope.tcss",
		include_str!("../../ui/cartridge_scope/scope.tcss"),
	),
];

pub(super) fn run(project: &super::Project, once: bool, python: Option<&Path>) -> Result<ExitCode> {
	let mut nonce = [0u8; 16];
	getrandom::fill(&mut nonce).map_err(|error| Error::Argument(error.to_string()))?;
	let suffix: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
	let root = std::env::temp_dir().join(format!("cartridge-scope-{suffix}"));
	let mut builder = std::fs::DirBuilder::new();
	#[cfg(unix)]
	{
		use std::os::unix::fs::DirBuilderExt;
		builder.mode(0o700);
	}
	builder.create(&root)?;
	let outcome = (|| {
		let package = root.join("cartridge_scope");
		std::fs::create_dir(&package)?;
		for (name, source) in FILES {
			std::fs::write(package.join(name), source)?;
		}
		let mut command = if let Some(python) = python {
			Command::new(python)
		} else {
			let mut command = Command::new("uv");
			command.args([
				"run",
				"--no-project",
				"--with",
				"textual==8.2.8",
				"--with",
				"rapidfuzz==3.14.3",
				"--python",
				"3.11",
				"python",
			]);
			command
		};
		command
			.args(["-m", "cartridge_scope", "--dir"])
			.arg(&project.dir);
		if once {
			command.arg("--once");
		}
		let status = command.current_dir(&project.dir)
			.env("PYTHONPATH", &root)
			.env("CARTRIDGE_SCOPE_HOST", std::env::current_exe()?)
			.status().map_err(|error| Error::Argument(format!("Scope could not start: {error}. Install uv, or use --python with Python 3.11+, Textual 8.2.8 and RapidFuzz 3.14.3.")))?;
		Ok(ExitCode::from(
			status.code().unwrap_or(1).clamp(0, 255) as u8
		))
	})();
	let _ = std::fs::remove_dir_all(&root);
	outcome
}
