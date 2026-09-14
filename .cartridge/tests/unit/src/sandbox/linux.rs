use super::*;
use std::os::unix::fs::PermissionsExt;

/// The policy travels as `<base binary> __confine <json> -- <cmd…>`: the shape
/// `confine` parses, with the trampoline on `cmd[0]` — the one caller's binary —
/// and never `current_exe()`, which under the test harness is the harness.
#[test]
fn the_trampoline_carries_the_policy_on_argv() {
	let fixture = tempfile::tempdir().unwrap();
	let directory = fixture.path().canonicalize().unwrap();
	let root = directory.join("cart");
	std::fs::create_dir(&root).unwrap();
	let base = directory.join("base");
	std::fs::write(&base, "#!/bin/sh\n").unwrap();
	std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o755)).unwrap();
	let sockets = directory.join("sockets");
	std::fs::create_dir(&sockets).unwrap();
	let cmd = vec![base.display().to_string(), "node".into()];
	let command = command(&cmd, &Grant::default(), &root, Some(&sockets)).unwrap();
	assert_eq!(command.get_program(), base.as_os_str());
	let argv: Vec<String> = command
		.get_args()
		.map(|arg| arg.to_string_lossy().into_owned())
		.collect();
	assert_eq!(argv[0], "__confine", "{argv:?}");
	assert_eq!(argv[2], "--", "{argv:?}");
	assert_eq!(argv[3], base.display().to_string(), "{argv:?}");
	assert_eq!(argv[4], "node", "{argv:?}");
	let policy: Policy = serde_json::from_str(&argv[1]).unwrap();
	assert!(!policy.net);
	assert!(
		policy.write.contains(&sockets),
		"the socket directory is writable: {argv:?}"
	);
}

/// A granted path that is not there refuses, naming the path: Landlock is
/// inode-based, and a dropped rule would be a silently ungranted one.
#[test]
fn a_grant_naming_a_path_that_is_not_there_is_refused() {
	let missing = tempfile::tempdir()
		.unwrap()
		.path()
		.canonicalize()
		.unwrap()
		.join("does-not-exist");
	let policy = Policy {
		read: vec![missing.clone()],
		write: vec![],
		exec: vec![],
		net: false,
	};
	let error = rules(&policy).unwrap_err();
	assert!(
		error.contains(&missing.display().to_string()),
		"{missing:?}: {error}"
	);
}

/// The socket filter is all or nothing: a nonempty `net` allows the internet
/// domains, an empty one allows unix sockets only. (Applying the filter would
/// restrict the test process; the two programs differ.)
#[test]
fn an_empty_net_grant_allows_only_unix_sockets() {
	let fields = |program: BpfProgram| -> Vec<(u16, u8, u8, u32)> {
		program
			.iter()
			.map(|rule| (rule.code, rule.jt, rule.jf, rule.k))
			.collect()
	};
	assert_ne!(
		fields(filter(false).unwrap()),
		fields(filter(true).unwrap())
	);
}

/// `exec: ["*"]` without the read-everything grant is refused, naming the grant
/// that makes it honest.
#[test]
fn an_exec_everything_grant_without_reading_everything_is_refused() {
	let root = std::env::temp_dir().join("cartridge-sandbox-linux");
	let grant = Grant {
		read: vec![],
		write: vec![],
		net: vec![],
		exec: vec!["*".into()],
	};
	let error = command(&["/bin/sh".into()], &grant, &root, None).unwrap_err();
	assert!(error.to_string().contains("read: [\"/\"]"), "{}", error);
}
