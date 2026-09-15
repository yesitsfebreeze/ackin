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
	assert!(text.contains("file-write*"), "{text}");
	assert!(
		text.contains("(allow file-write* (literal \"/dev/null\"))"),
		"{text}"
	);
	assert!(!text.contains("network"), "{text}");
	assert!(
		text.contains("(allow process-exec (literal \"/bin/tool\"))"),
		"{text}"
	);
	assert!(text.contains("(allow file-read*"), "{text}");
}

#[cfg(target_os = "macos")]
#[test]
fn a_write_grant_names_the_canonicalized_path_and_implies_the_read() {
	let root = std::env::temp_dir().canonicalize().unwrap();
	let grant = Grant {
		write: vec!["cache".into()],
		..Grant::default()
	};
	let text = profile(&grant, &root, Path::new("/bin/tool"), None);
	let line = format!("(subpath \"{}\")", root.join("cache").display());
	assert!(text.contains(&line), "{text}");
}

#[test]
fn a_net_grant_turns_the_network_on_and_an_empty_one_leaves_it_off() {
	let grant = Grant {
		net: vec!["api.host".into()],
		..Grant::default()
	};
	let on = profile(&grant, &root(), Path::new("/bin/tool"), None);
	assert!(on.contains("(allow network*)"), "{on}");
	let off = profile(&Grant::default(), &root(), Path::new("/bin/tool"), None);
	assert!(!off.contains("network"), "{off}");
}

#[cfg(target_os = "macos")]
#[test]
fn an_exec_grant_that_resolves_builds_a_literal_and_one_that_does_not_builds_nothing() {
	let grant = Grant {
		exec: vec!["ls".into(), "no-such-program-xyz".into()],
		..Grant::default()
	};
	let text = profile(&grant, &root(), Path::new("/bin/tool"), None);
	let resolved = granted_exec("ls", &root()).expect("ls is on PATH");
	assert!(
		text.contains(&format!("(literal \"{}\")", resolved.display())),
		"{text}"
	);
	assert!(!text.contains("no-such-program-xyz"), "{text}");
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
#[test]
fn an_interpreter_reaches_its_own_installation() {
	use std::os::unix::fs::PermissionsExt;
	let fixture = tempfile::tempdir().unwrap();
	let directory = fixture.path().canonicalize().unwrap();
	let prefix = directory.join("fake");
	std::fs::create_dir_all(prefix.join("bin")).unwrap();
	std::fs::create_dir_all(prefix.join("lib")).unwrap();
	std::fs::write(prefix.join("lib/data.txt"), "runtime\n").unwrap();
	let tool = prefix.join("bin/tool");
	std::fs::write(
		&tool,
		"#!/bin/sh\nread -r line < \"${0%/bin/tool}/lib/data.txt\" && echo \"$line\"\n",
	)
	.unwrap();
	std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
	let root = directory.join("cart");
	std::fs::create_dir(&root).unwrap();
	let output = command(
		&[tool.display().to_string()],
		&Grant::default(),
		&root,
		None,
	)
	.unwrap()
	.output()
	.unwrap();
	assert!(output.status.success(), "{:?}", output);
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "runtime");
}

#[test]
fn the_prefix_stops_at_a_system_directory() {
	let shared = profile(&Grant::default(), &root(), Path::new("/usr/bin/env"), None);
	assert!(
		!shared.contains("(subpath \"/usr\")"),
		"/usr is shared, not one program's: {shared}"
	);
	let own = profile(
		&Grant::default(),
		&root(),
		Path::new("/opt/x/bin/tool"),
		None,
	);
	assert!(own.contains("(subpath \"/opt/x\")"), "{own}");

	let fixture = tempfile::tempdir().unwrap();
	let home = fixture.path().canonicalize().unwrap();
	let previous = std::env::var_os("HOME");
	std::env::set_var("HOME", &home);
	let text = profile(&Grant::default(), &root(), &home.join("bin/tool"), None);
	match previous {
		Some(value) => std::env::set_var("HOME", value),
		None => std::env::remove_var("HOME"),
	}
	assert!(
		!text.contains(&format!("(subpath \"{}\")", home.display())),
		"the user's home is not one program's installation: {text}"
	);
}

#[cfg(target_os = "macos")]
#[test]
fn the_runtime_is_named_canonically() {
	let text = profile(&Grant::default(), &root(), Path::new("/bin/tool"), None);
	assert!(
		text.contains("(subpath \"/private/var/db/timezone\")"),
		"{text}"
	);
	assert!(text.contains("(subpath \"/private/etc\")"), "{text}");
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn an_async_spawn_is_confined_like_the_synchronous_one() {
	let (fixture, root, cmd) = wall_fixture();
	let output =
		tokio::process::Command::from(command(&cmd, &Grant::default(), &root, None).unwrap())
			.output()
			.await
			.unwrap();
	assert!(output.status.success(), "{:?}", output);
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "denied");
	assert!(!fixture.path().join("outside").exists());
}
