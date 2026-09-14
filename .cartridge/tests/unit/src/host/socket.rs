use std::os::unix::fs::{MetadataExt, PermissionsExt};

use super::*;

#[test]
fn an_owner_only_directory_is_created_and_repaired() {
	let tmp = tempfile::tempdir().unwrap();
	let dir = tmp.path().join("sockets");
	owner_only_dir(&dir).unwrap();
	let meta = std::fs::metadata(&dir).unwrap();
	// SAFETY: `getuid` cannot fail and touches no memory.
	assert_eq!(meta.uid(), unsafe { libc::getuid() });
	assert_eq!(
		meta.permissions().mode() & 0o777,
		0o700,
		"created owner-only"
	);
	std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
	owner_only_dir(&dir).unwrap();
	assert_eq!(
		std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
		0o700,
		"a widened directory is narrowed again, not accepted"
	);
}

#[test]
fn a_symlinked_socket_directory_is_refused() {
	let tmp = tempfile::tempdir().unwrap();
	let real = tmp.path().join("real");
	std::fs::create_dir(&real).unwrap();
	let link = tmp.path().join("link");
	std::os::unix::fs::symlink(&real, &link).unwrap();
	let error = owner_only_dir(&link)
		.expect_err("a link standing in for the socket directory is a substitution");
	assert!(
		error.contains("not a directory owned by this user"),
		"{error}"
	);
}

#[test]
fn a_run_directory_is_owner_only() {
	let tmp = tempfile::tempdir().unwrap();
	let dir = run_dir(tmp.path()).unwrap();
	let mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
	let _ = std::fs::remove_dir_all(&dir);
	assert_eq!(
		mode, 0o700,
		"a run's sockets sit in a directory only this user can enter"
	);
}
