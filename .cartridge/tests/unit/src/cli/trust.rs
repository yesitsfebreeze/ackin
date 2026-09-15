use super::*;

/// A folder that is neither a project nor a cartridge is trusted by an
/// explicit path — what a bare-Lua refusal suggests trying.
#[test]
fn an_explicit_path_trusts_a_folder_that_is_neither() {
	let _turn = crate::cli::setup::tests::trust_home();
	let tmp = tempfile::tempdir().unwrap();
	std::env::set_var("CARTRIDGE_HOME", tmp.path().join("home"));
	let folder = tmp.path().join("folder");
	std::fs::create_dir_all(&folder).unwrap();
	std::fs::write(folder.join("x.lua"), "return {}").unwrap();
	let exit = run(Some(&folder), false, false, false).unwrap();
	assert_eq!(exit, ExitCode::SUCCESS);
	cartridge::trust::verify(&folder.join("x.lua")).unwrap();
}
