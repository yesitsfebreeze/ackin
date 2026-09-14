use super::*;

/// A project of this test's own, under the binary's one trust home.
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
	// `printf 'return {}\n' | shasum -a 256`
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
