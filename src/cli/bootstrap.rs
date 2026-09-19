use cartridge::ledger::Ledger;
use cartridge::{Error, Result};

use super::{setup, Project};

pub(crate) async fn launch(project: &Project) -> Result<()> {
	if project.descriptor.join("init.lua").is_file() {
		return Ok(());
	}
	let root = std::env::current_dir()?;
	let exe = std::env::current_exe()?;
	let source = if Ledger::scan(&project.dir).is_empty() {
		exe.ancestors()
			.skip(1)
			.find(|dir| !Ledger::scan(dir).is_empty())
			.unwrap_or(&project.dir)
			.to_path_buf()
	} else {
		project.dir.clone()
	};
	let with = setup::candidates(std::slice::from_ref(&source), Vec::new())
		.into_iter()
		.map(|candidate| candidate.name)
		.collect();
	let result = setup::setup(
		&root,
		&project.dir,
		setup::Ask {
			from: vec![source],
			with,
			yes: true,
			catalog: None,
		},
	)
	.await?;
	if result != std::process::ExitCode::SUCCESS {
		return Err(Error::Argument(
			"automatic project setup failed; use cartridge setup --from <cartridge-folder>".into(),
		));
	}
	Ok(())
}
