use super::*;
use std::path::PathBuf;

#[test]
fn an_empty_command_is_refused_before_launch() {
	let error = command(&[], &Grant::default(), &root(), None).unwrap_err();
	assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

#[cfg(target_os = "macos")]
fn wall_fixture() -> (tempfile::TempDir, PathBuf, Vec<String>) {
	use std::os::unix::fs::PermissionsExt;
	let temporary = tempfile::tempdir().unwrap();
	let directory = temporary.path().canonicalize().unwrap();
	let root = directory.join("cartridge");
	std::fs::create_dir(&root).unwrap();
	let script = root.join("child.sh");
	std::fs::write(
		&script,
		format!(
			"#!/bin/sh\nif echo escaped > '{}/outside'; then echo allowed; else echo denied; fi\n",
			directory.display(),
		),
	)
	.unwrap();
	std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
	let cmd = vec![script.display().to_string()];
	(temporary, root, cmd)
}

#[cfg(target_os = "macos")]
#[test]
fn the_synchronous_command_enforces_the_empty_grant() {
	let (fixture, root, cmd) = wall_fixture();
	let output = command(&cmd, &Grant::default(), &root, None)
		.unwrap()
		.output()
		.unwrap();
	assert!(output.status.success(), "{:?}", output);
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "denied");
	assert!(!fixture.path().join("outside").exists());
}

fn root() -> PathBuf {
	std::env::temp_dir().join("cartridge-sandbox-tests")
}

#[test]
fn an_empty_grant_builds_no_allowance_beyond_the_runtime() {
	let text = profile(&Grant::default(), &root(), Path::new("/bin/tool"), None);
	assert!(text.contains("(deny default)"), "{text}");
	assert!(!text.contains("file-write*"), "{text}");
	assert!(!text.contains("network"), "{text}");
	assert!(
		text.contains("(allow process-exec (literal \"/bin/tool\"))"),
		"{text}"
	);
	assert!(text.contains("(allow file-read*"), "{text}");
}

#[test]
fn a_write_grant_names_the_canonicalized_path_and_implies_the_read() {
	let root = std::env::temp_dir().canonicalize().unwrap();
	let grant = Grant {
		read: vec![],
		write: vec!["cache".into()],
		net: vec![],
		exec: vec![],
	};
	let text = profile(&grant, &root, Path::new("/bin/tool"), None);
	let line = format!("(subpath \"{}\")", root.join("cache").display());
	assert!(text.contains(&line), "{text}");
}

#[test]
fn a_net_grant_turns_the_network_on_and_an_empty_one_leaves_it_off() {
	let grant = Grant {
		read: vec![],
		write: vec![],
		net: vec!["api.host".into()],
		exec: vec![],
	};
	let on = profile(&grant, &root(), Path::new("/bin/tool"), None);
	assert!(on.contains("(allow network*)"), "{on}");
	let off = profile(&Grant::default(), &root(), Path::new("/bin/tool"), None);
	assert!(!off.contains("network"), "{off}");
}

#[test]
fn an_exec_grant_that_resolves_builds_a_literal_and_one_that_does_not_builds_nothing() {
	let grant = Grant {
		read: vec![],
		write: vec![],
		net: vec![],
		exec: vec!["ls".into(), "no-such-program-xyz".into()],
	};
	let text = profile(&grant, &root(), Path::new("/bin/tool"), None);
	let resolved = granted_exec("ls", &root()).expect("ls is on PATH");
	assert!(
		text.contains(&format!("(literal \"{}\")", resolved.display())),
		"{text}"
	);
	assert!(!text.contains("no-such-program-xyz"), "{text}");
}

#[test]
fn a_script_names_its_interpreter() {
	let dir = tempfile::tempdir().unwrap();
	let script = dir.path().join("cart.sh");
	std::fs::write(&script, "#!/bin/sh\necho hi\n").unwrap();
	let text = profile(&Grant::default(), dir.path(), &script, None);
	assert!(text.contains("(literal \"/bin/sh\")"), "{text}");
	assert!(
		text.contains(&format!(
			"(literal \"{}\")",
			script.canonicalize().unwrap().display()
		)),
		"{text}"
	);
}
