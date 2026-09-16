use super::*;

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
	crate::tests::home();
	let dir = tempfile::tempdir().unwrap();
	for (name, body) in files {
		crate::tests::write(dir.path(), name, body);
	}
	dir
}

#[test]
fn an_untrusted_project_is_refused_until_it_is_trusted() {
	let dir = project(&[(".cartridge/init.lua", "return {}")]);
	let init = dir.path().join(".cartridge/init.lua");
	let refused = verify(&init).unwrap_err().to_string();
	let canonical = dir.path().canonicalize().unwrap();
	assert!(
		refused.contains(&format!("`cartridge trust {}`", canonical.display())),
		"{refused}"
	);
	assert!(
		refused.contains(&init.canonicalize().unwrap().display().to_string()),
		"{refused}"
	);
	record(dir.path()).unwrap();
	verify(&init).unwrap();
}

#[test]
fn a_changed_file_stops_being_trusted() {
	let dir = project(&[(".cartridge/init.lua", "return {}")]);
	record(dir.path()).unwrap();
	crate::tests::write(dir.path(), ".cartridge/init.lua", "return { {} }");
	let refused = verify(&dir.path().join(".cartridge/init.lua")).unwrap_err();
	assert!(
		refused.contains("has changed since it was trusted"),
		"{refused}"
	);
}

#[test]
fn a_widened_grant_untrusts_its_manifest() {
	let dir = project(&[
		(".cartridge/init.lua", "return {}"),
		("cart/cartridge.json", r#"{"name":"c","entry":"init.lua"}"#),
		("cart/init.lua", ""),
	]);
	record(dir.path()).unwrap();
	let manifest = dir.path().join("cart/cartridge.json");
	crate::loader::Cartridge::read(&manifest).unwrap();
	crate::tests::write(
		dir.path(),
		"cart/cartridge.json",
		r#"{"name":"c","entry":"init.lua","grant":{"exec":["/bin/sh"]}}"#,
	);
	let refused = crate::loader::Cartridge::read(&manifest).err().unwrap();
	assert!(
		refused.contains("has changed since it was trusted"),
		"{refused}"
	);
}

#[test]
fn a_file_the_record_does_not_name_is_refused() {
	let dir = project(&[(".cartridge/init.lua", "return {}")]);
	record(dir.path()).unwrap();
	crate::tests::write(dir.path(), "cart/extra.lua", "");
	let refused = verify(&dir.path().join("cart/extra.lua")).unwrap_err();
	assert!(
		refused.contains("is not in this project's trust record"),
		"{refused}"
	);
}

#[cfg(unix)]
#[test]
fn every_spelling_of_a_directory_is_one_record() {
	let dir = project(&[(".cartridge/init.lua", "return {}"), ("sub/x.lua", "")]);
	record(&dir.path().join(".")).unwrap();
	let link = tempfile::tempdir().unwrap();
	let linked = link.path().join("project");
	std::os::unix::fs::symlink(dir.path(), &linked).unwrap();
	verify(&linked.join(".cartridge/init.lua")).unwrap();
	verify(&dir.path().join("sub/../.cartridge/init.lua")).unwrap();
	let canonical = dir.path().canonicalize().unwrap();
	let records = list().unwrap();
	assert_eq!(records.iter().filter(|r| r.project == canonical).count(), 1);
}

#[test]
fn a_digest_is_the_files_own_sha_256() {
	let dir = project(&[("x.lua", "return {}\n")]);
	assert_eq!(
		digest(&dir.path().join("x.lua")).unwrap(),
		"1232d8379de77e154ca533689af2e42629dd7574bda5a0a390799849f07607c3"
	);
}

#[test]
fn the_users_own_home_needs_no_trust() {
	// Not `home/config.lua` itself: host tests read that concurrently.
	crate::tests::write(crate::tests::home(), "own/config.lua", "return {}");
	verify(&crate::tests::home().join("own/config.lua")).unwrap();
}

#[cfg(unix)]
#[test]
fn the_store_is_private_to_this_user() {
	use std::os::unix::fs::PermissionsExt;
	let dir = project(&[(".cartridge/init.lua", "return {}")]);
	record(dir.path()).unwrap();
	let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
	assert_eq!(mode(&store().unwrap()), 0o700);
	let at = record_path(&dir.path().canonicalize().unwrap()).unwrap();
	assert_eq!(mode(&at), 0o600);
}

#[cfg(unix)]
#[test]
fn the_walk_skips_build_output_and_links() {
	let dir = project(&[
		(".cartridge/init.lua", "return {}"),
		("target/x.lua", ""),
		(".git/x.lua", ""),
		("node_modules/x.lua", ""),
	]);
	let elsewhere = project(&[("x.lua", "")]);
	std::os::unix::fs::symlink(elsewhere.path(), dir.path().join("linked")).unwrap();
	assert_eq!(record(dir.path()).unwrap().files.len(), 1);
}

#[test]
fn a_nested_record_does_not_shadow_a_fresh_outer_one() {
	let dir = project(&[
		(".cartridge/init.lua", "return {}"),
		("cart/cartridge.json", r#"{"name":"c","entry":"init.lua"}"#),
		("cart/init.lua", ""),
	]);
	record(&dir.path().join("cart")).unwrap();
	crate::tests::write(dir.path(), "cart/init.lua", "-- edited");
	record(dir.path()).unwrap();
	verify(&dir.path().join("cart/init.lua")).unwrap();
}

#[test]
fn a_relative_or_empty_home_is_refused_rather_than_trust_disabling() {
	let bin = crate::tests::built(&["--bin", "cartridge"]);
	let dir = tempfile::tempdir().unwrap();
	crate::tests::write(dir.path(), ".cartridge/init.lua", "return {}");
	for (home_var, home) in [
		("CARTRIDGE_HOME", ""),
		("CARTRIDGE_HOME", "."),
		("HOME", ""),
	] {
		let mut command = std::process::Command::new(&bin);
		command
			.arg("list")
			.current_dir(dir.path())
			.env_remove("CARTRIDGE_HOME")
			.env_remove("HOME")
			.env(home_var, home)
			.stdout(std::process::Stdio::null())
			.stderr(std::process::Stdio::piped());
		let said = String::from_utf8_lossy(&command.output().unwrap().stderr).into_owned();
		assert!(
			said.contains("is not an absolute path"),
			"{home_var}=`{home}`: {said}"
		);
	}
}

#[test]
fn read_refuses_a_file_that_changed_after_it_was_trusted() {
	let dir = project(&[("x.lua", "return {}\n")]);
	record(dir.path()).unwrap();
	crate::tests::write(dir.path(), "x.lua", "return { {} }\n");
	let refused = read(&dir.path().join("x.lua")).unwrap_err();
	assert!(
		refused.contains("has changed since it was trusted"),
		"{refused}"
	);
}

#[cfg(unix)]
#[test]
fn a_global_config_symlinked_out_of_the_home_still_passes() {
	let elsewhere = project(&[("config.lua", "return {}")]);
	let link = crate::tests::home().join("config.lua");
	let _ = std::fs::remove_file(&link);
	std::os::unix::fs::symlink(elsewhere.path().join("config.lua"), &link).unwrap();
	verify(&link).unwrap();
	let _ = std::fs::remove_file(&link);
}

#[test]
fn an_untrusted_bare_lua_entry_is_refused_at_its_resolve() {
	let dir = project(&[("x.lua", "return {}")]);
	let refused = crate::loader::resolve(&dir.path().join("x.lua"))
		.unwrap_err()
		.to_string();
	assert!(refused.contains("is in no trusted project"), "{refused}");
}

#[test]
fn a_deleted_project_can_still_be_untrusted() {
	let dir = project(&[(".cartridge/init.lua", "return {}")]);
	let canonical = dir.path().canonicalize().unwrap();
	record(dir.path()).unwrap();
	let inside = dir.keep();
	std::fs::remove_dir_all(&inside).unwrap();
	assert!(revoke(&inside).unwrap() >= 1);
	assert!(list()
		.unwrap()
		.iter()
		.all(|record| record.project != canonical));
}

#[test]
fn a_deleted_project_is_untrusted_by_a_relative_path() {
	let dir = project(&[(".cartridge/init.lua", "return {}")]);
	let canonical = dir.path().canonicalize().unwrap();
	record(dir.path()).unwrap();
	let inside = dir.keep();
	let parent = inside.parent().unwrap().to_path_buf();
	let name = inside.file_name().unwrap().to_owned();
	std::fs::remove_dir_all(&inside).unwrap();
	let bin = crate::tests::built(&["--bin", "cartridge"]);
	let output = std::process::Command::new(&bin)
		.arg("trust")
		.arg("--revoke")
		.arg(&name)
		.current_dir(&parent)
		.env("CARTRIDGE_HOME", crate::tests::home())
		.output()
		.unwrap();
	assert!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert!(list()
		.unwrap()
		.iter()
		.all(|record| record.project != canonical));
}

#[test]
fn revoking_a_project_takes_its_nested_records() {
	let dir = project(&[(".cartridge/init.lua", "return {}"), ("cart/init.lua", "")]);
	record(&dir.path().join("cart")).unwrap();
	record(dir.path()).unwrap();
	revoke(dir.path()).unwrap();
	let refused = verify(&dir.path().join("cart/init.lua")).unwrap_err();
	assert!(refused.contains("run `cartridge trust"), "{refused}");
}

#[test]
fn a_file_under_a_skipped_directory_names_the_real_dead_end() {
	let dir = project(&[(".cartridge/init.lua", "return {}"), ("target/x.lua", "")]);
	record(dir.path()).unwrap();
	let refused = verify(&dir.path().join("target/x.lua")).unwrap_err();
	assert!(
		refused
			.to_string()
			.contains("never records; move it or link its folder"),
		"{refused}"
	);
}

#[cfg(not(windows))]
#[test]
fn a_built_native_module_is_a_source_so_its_rebuild_restarts_the_cartridge() {
	let dir = project(&[
		(
			"x/cartridge.json",
			r#"{"name": "x-y", "entry": "init.lua"}"#,
		),
		("x/init.lua", "return {}"),
		("x/target/debug/libx_y.dylib", ""),
	]);
	record(&dir.path().join("x")).unwrap();
	let declared = crate::loader::resolve(&dir.path().join("x")).unwrap();
	let module = dir
		.path()
		.join("x/target/debug/libx_y.dylib")
		.canonicalize()
		.unwrap();
	assert!(declared.sources.contains(&module), "{:?}", declared.sources);
	assert_eq!(
		declared.sources.len(),
		3,
		"absent candidates are not sources"
	);
}
