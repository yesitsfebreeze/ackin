#![cfg(unix)]

use std::path::Path;
use std::process::{Command, Output};

fn run(binary: &Path, project: &Path, home: &Path, args: &[&str]) -> Output {
	Command::new(binary)
		.current_dir(project)
		.env("CARTRIDGE_HOME", home)
		.env_remove("CARTRIDGE_YOLO")
		.args(args)
		.output()
		.unwrap()
}

#[test]
fn launch_initializes_a_fresh_project_and_preserves_it_on_repeat() {
	let temp = tempfile::tempdir().unwrap();
	let distribution = temp.path().join("distribution");
	let project = temp.path().join("new project");
	let home = temp.path().join("home");
	std::fs::create_dir_all(distribution.join("bin")).unwrap();
	std::fs::create_dir_all(&project).unwrap();
	std::fs::create_dir_all(&home).unwrap();
	std::fs::write(
		home.join("catalog.json"),
		r#"{"obsolete":{"repository":"/nonexistent/cartridge-catalog-entry"}}"#,
	)
	.unwrap();
	let binary = distribution.join("bin/cartridge");
	std::fs::hard_link(env!("CARGO_BIN_EXE_cartridge"), &binary).unwrap();
	let proxy = distribution.join("proxy.ctg");
	std::fs::create_dir_all(&proxy).unwrap();
	std::fs::write(
		proxy.join("cartridge.json"),
		r#"{"name":"proxy","entry":"init.lua","events":{"proxy":{}},"listen":["proxy"]}"#,
	)
	.unwrap();
	std::fs::write(
		proxy.join("init.lua"),
		r#"
cartridge.listen("proxy", function(input)
  assert(input.passthrough == true)
  return { program = "/bin/sh", args = { "-c", "pwd -P; exit 7" } }
end)
"#,
	)
	.unwrap();
	let first = run(&binary, &project, &home, &["launch", "codex", "-ps"]);
	let _ = run(&binary, &project, &home, &["stop"]);
	assert_eq!(
		first.status.code(),
		Some(7),
		"{}\n{}",
		String::from_utf8_lossy(&first.stdout),
		String::from_utf8_lossy(&first.stderr)
	);
	assert!(String::from_utf8_lossy(&first.stdout)
		.contains(project.canonicalize().unwrap().to_str().unwrap()));
	assert!(project.join(".cartridge/daemon.log").is_file());
	let init = project.join(".cartridge/init.lua");
	let original = std::fs::read_to_string(&init).unwrap();
	assert!(original.contains("proxy"));
	assert!(!original.contains("obsolete"));
	let second = run(
		&binary,
		&project,
		&home,
		&["launch", "codex", "--yolo", "-ps", "--", "--yolo"],
	);
	let _ = run(&binary, &project, &home, &["stop"]);
	assert_eq!(
		second.status.code(),
		Some(7),
		"{}",
		String::from_utf8_lossy(&second.stderr)
	);
	assert_eq!(std::fs::read_to_string(&init).unwrap(), original);
	assert!(!String::from_utf8_lossy(&second.stdout).contains("wrote"));
}
