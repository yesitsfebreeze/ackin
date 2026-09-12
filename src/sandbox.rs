//! The wall a cartridge's grant builds around its child process.
//!
//! A cartridge's `grant` is compiled into an operating-system policy and the
//! child is spawned inside it, so a cartridge reaching outside what it declared
//! is stopped by the OS rather than by review. On macOS the mechanism is
//! `sandbox-exec` with a generated profile; on Linux it is Landlock with
//! seccomp, which is not implemented here yet — a child that cannot be
//! confined on the running platform is refused, never spawned unconfined.
//!
//! **What is implicit.** A child cannot exist without reading its own binary,
//! its interpreter and the system runtime, so every profile allows exactly
//! that: the executed binary, the interpreter its shebang names, the cartridge
//! folder it was declared in, and the runtime directories dyld reads. That is
//! the machine's own plumbing, not a capability. Everything else — writes,
//! network, other programs — is allowed only where the grant names it.
//!
//! **The empty grant is the tightest policy.** A cartridge that declares
//! nothing can run, talk on the stdio wire, and reach nothing else.
//!
//! **What the OS on this platform cannot express.** `grant.net` names hosts,
//! and `sandbox-exec` accepts only `*` and `localhost` in a remote filter, so
//! a grant naming specific hosts cannot be confined host by host: a nonempty
//! `net` allows outbound network and an empty one allows none. The gap is the
//! platform's, and it is stated here rather than hidden.

use std::path::{Path, PathBuf};

use crate::loader::Grant;

#[cfg(target_os = "linux")]
#[path = "sandbox/linux.rs"]
mod linux;

/// The operating-system policy launcher.
#[cfg(target_os = "macos")]
const SBIN: &str = "/usr/bin/sandbox-exec";

/// A grant path as the profile wants it: absolute, canonical when the path
/// exists, lexically cleaned when it does not yet — a cartridge may name a
/// directory it will create. Relative paths live inside the cartridge folder.
///
/// The profile carries **every spelling** of a granted path. The seatbelt
/// compares the path a child opened against the profile's own spelling, and a
/// clause written through a symlinked prefix (`/tmp`, `/var`) matches nothing:
/// only the canonical spelling — `/private/...` — opens. The spelling a child
/// constructs is its own choice, so the grant is rendered both ways: resolved
/// when the path exists, and resolved through its deepest existing ancestor
/// when it does not yet — a cartridge may name a file it will create. A child
/// that opens a path through a symlinked spelling is denied; a cartridge that
/// needs the path declares it canonically.
fn granted_paths(path: &str, root: &Path) -> Vec<PathBuf> {
	let path = Path::new(path);
	let absolute = if path.is_absolute() {
		path.to_path_buf()
	} else {
		root.join(path)
	};
	let mut clean = PathBuf::new();
	for part in absolute.components() {
		match part {
			std::path::Component::CurDir => {}
			other => clean.push(other.as_os_str()),
		}
	}
	let mut forms = vec![clean.clone()];
	// The canonical spelling: the path itself when it exists, else the
	// deepest existing ancestor resolved, with the remaining components
	// appended — the file a cartridge creates is still `/private/...`.
	let mut suffix = Vec::new();
	let mut walk = clean.as_path();
	let resolved = loop {
		if let Ok(canonical) = walk.canonicalize() {
			break canonical;
		}
		let Some(parent) = walk.parent() else {
			break clean;
		};
		if let Some(name) = walk.file_name() {
			suffix.push(name.to_os_string());
		}
		walk = parent;
	};
	let canonical = suffix
		.into_iter()
		.rev()
		.fold(resolved, |acc, part| acc.join(part));
	forms.push(canonical);
	forms.sort();
	forms.dedup();
	forms
}

/// The interpreter a script's shebang names, so a `#!/bin/sh` cartridge runs
/// when the sandbox allows exactly the programs it named plus its own entry.
/// A binary has no shebang and needs nothing; `env` is allowed as itself,
/// because resolving further would mean allowing every directory on PATH.
/// On macOS `/bin/sh` is a launcher that execs a variant — `/bin/bash` — so
/// the variant is allowed beside it: the same plumbing a child needs to exist.
fn interpreter(binary: &Path) -> Option<PathBuf> {
	let Ok(first) = std::fs::read(binary) else {
		return None;
	};
	if !first.starts_with(b"#!") {
		return None;
	}
	let line = String::from_utf8_lossy(&first[..first.len().min(160)]);
	let line = line.lines().next().unwrap_or("").trim_start_matches("#!");
	let program = line.split_whitespace().next()?;
	let path = PathBuf::from(program);
	if path.file_name().is_some_and(|name| name == "env") {
		return path.canonicalize().ok();
	}
	path.canonicalize().ok()
}

/// The interpreters one shebang resolves into, including the launcher
/// variants the platform inserts between the shebang and the real shell.
fn interpreters(binary: &Path) -> Vec<PathBuf> {
	let Some(first) = interpreter(binary) else {
		return Vec::new();
	};
	let mut all = vec![first.clone()];
	if first == Path::new("/bin/sh") {
		for variant in ["/bin/bash", "/bin/zsh"] {
			let variant = PathBuf::from(variant);
			if variant.is_file() {
				all.push(variant);
			}
		}
	}
	all
}

/// The executable basename or path a grant names, resolved the way the host
/// resolves a cartridge's own binary: the cartridge's `bin/`, then beside the
/// running cartridge, then PATH. Unresolvable entries build no line — the OS
/// denies what was never allowed.
fn granted_exec(program: &str, root: &Path) -> Option<PathBuf> {
	let path = Path::new(program);
	let candidates: Vec<PathBuf> = if path.components().count() > 1 || path.is_absolute() {
		vec![if path.is_absolute() {
			path.to_path_buf()
		} else {
			root.join(path)
		}]
	} else {
		let beside = std::env::current_exe()
			.ok()
			.and_then(|exe| exe.parent().map(|dir| dir.join(program)));
		std::iter::once(root.join("bin").join(program))
			.chain(beside)
			.chain(std::env::var_os("PATH").into_iter().flat_map(|paths| {
				std::env::split_paths(&paths)
					.map(|dir| dir.join(program))
					.collect::<Vec<_>>()
			}))
			.collect()
	};
	candidates
		.into_iter()
		.find(|p| p.is_file())
		.and_then(|p| p.canonicalize().ok())
}

/// One `literal` clause, escaped the way the profile language needs it: a
/// path is data here, and a quote or backslash inside a declared name must
/// not end the string the profile reads.
fn literal(path: &Path) -> String {
	format!(
		"\"{}\"",
		path.display()
			.to_string()
			.replace('\\', "\\\\")
			.replace('"', "\\\"")
	)
}

/// The profile text for one cartridge: what it declared, plus the plumbing a
/// child needs to exist. Every line the grant does not ask for is absent, and
/// `deny default` is what remains.
pub fn profile(grant: &Grant, root: &Path, binary: &Path) -> String {
	let mut profile = String::from("(version 1)\n(deny default)\n");
	// Paths the OS resolves through symlinks must be named canonically: a
	// grant written through `/tmp` would never match the file it names.
	let binary = binary
		.canonicalize()
		.unwrap_or_else(|_| binary.to_path_buf());
	let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
	// Exec: the child's own entry, its interpreter when it is a script, and
	// what the grant named. Nothing else is executable.
	let mut exec = vec![binary.to_path_buf()];
	exec.extend(interpreters(&binary));
	for program in &grant.exec {
		if let Some(path) = granted_exec(program, &root) {
			exec.push(path);
		}
	}
	exec.sort();
	exec.dedup();
	if !exec.is_empty() {
		let lines: Vec<String> = exec
			.iter()
			.map(|p| format!("(literal {})", literal(p)))
			.collect();
		profile.push_str(&format!("(allow process-exec {})\n", lines.join(" ")));
	}
	// Read: the runtime a child needs to exist, the cartridge folder it was
	// declared in, and the paths it asked to read — a write is also a read.
	// The root itself is a literal, not a subpath: dyld reads the root
	// directory on the way up, and a subpath of `/` would be everything.
	let mut read = vec![
		PathBuf::from("/usr/lib"),
		PathBuf::from("/System/Library"),
		PathBuf::from("/dev"),
		PathBuf::from("/etc"),
	];
	read.push(root.to_path_buf());
	read.push(binary.to_path_buf());
	for path in grant.read.iter().chain(grant.write.iter()) {
		read.extend(granted_paths(path, &root));
	}
	read.sort();
	read.dedup();
	let mut lines = vec![format!("(literal {})", literal(Path::new("/")))];
	lines.extend(read.iter().map(|p| format!("(subpath {})", literal(p))));
	profile.push_str(&format!("(allow file-read* {})\n", lines.join(" ")));
	// Write: only what the grant names, in every spelling the child may open.
	let writes: Vec<String> = grant
		.write
		.iter()
		.flat_map(|path| granted_paths(path, &root))
		.map(|p| format!("(subpath {})", literal(&p)))
		.collect();
	if !writes.is_empty() {
		profile.push_str(&format!("(allow file-write* {})\n", writes.join(" ")));
	}
	// Network is all or nothing on this platform: a nonempty `net` allows
	// outbound connections, an empty one allows none. The hosts named in a
	// nonempty grant are not expressible in a remote filter.
	if !grant.net.is_empty() {
		profile.push_str("(allow network*)\n(allow system-socket)\n");
	}
	// The wire itself: pipes are not files, and a child that talks to the host
	// through stdio needs no write grant for that. What it needs is here.
	profile.push_str("(allow sysctl-read)\n(allow mach-lookup)\n(allow process-fork)\n");
	profile
}

/// Prepare a confined command before any cartridge code executes, including
/// discovery. Callers can attach their pipes and environment to this command;
/// the operating-system policy remains the same for every launch route.
pub fn command(
	cmd: &[String],
	grant: &Grant,
	root: &Path,
) -> std::io::Result<std::process::Command> {
	let binary = cmd.first().ok_or_else(|| {
		std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty cartridge command")
	})?;
	#[cfg(target_os = "macos")]
	{
		let binary = Path::new(binary);
		let text = profile(grant, root, binary);
		// The profile travels on the command line, so no file is written and no
		// file has to outlive the spawn. `ps` shows the policy; that is a
		// property of `sandbox-exec`, not a leak of what the cartridge declared.
		let mut command = std::process::Command::new(SBIN);
		command.arg("-p").arg(&text).args(cmd);
		Ok(command)
	}
	#[cfg(target_os = "linux")]
	{
		let _ = binary;
		linux::command(cmd, grant, root)
	}
	#[cfg(not(any(target_os = "macos", target_os = "linux")))]
	{
		let _ = (binary, cmd, grant, root);
		Err(std::io::Error::new(
			std::io::ErrorKind::Unsupported,
			"no sandbox mechanism on this platform: a cartridge process is not spawned unconfined",
		))
	}
}

/// Spawn a confined child with the stdio wire and ownership used by the host.
pub fn spawn(cmd: &[String], grant: &Grant, root: &Path) -> std::io::Result<tokio::process::Child> {
	let mut command = tokio::process::Command::from(command(cmd, grant, root)?);
	command
		.stdin(std::process::Stdio::piped())
		.stdout(std::process::Stdio::piped())
		.stderr(std::process::Stdio::piped())
		.kill_on_drop(true)
		.spawn()
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;

	#[test]
	fn an_empty_command_is_refused_before_launch() {
		let error = command(&[], &Grant::default(), &root()).unwrap_err();
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
		let output = command(&cmd, &Grant::default(), &root)
			.unwrap()
			.output()
			.unwrap();
		assert!(output.status.success(), "{:?}", output);
		assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "denied");
		assert!(!fixture.path().join("outside").exists());
	}

	#[cfg(target_os = "macos")]
	#[tokio::test]
	async fn the_asynchronous_spawn_enforces_the_same_empty_grant() {
		let (fixture, root, cmd) = wall_fixture();
		let output = spawn(&cmd, &Grant::default(), &root)
			.unwrap()
			.wait_with_output()
			.await
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
		let text = profile(&Grant::default(), &root(), Path::new("/bin/tool"));
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
		let text = profile(&grant, &root, Path::new("/bin/tool"));
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
		let on = profile(&grant, &root(), Path::new("/bin/tool"));
		assert!(on.contains("(allow network*)"), "{on}");
		let off = profile(&Grant::default(), &root(), Path::new("/bin/tool"));
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
		let text = profile(&grant, &root(), Path::new("/bin/tool"));
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
		let text = profile(&Grant::default(), dir.path(), &script);
		assert!(text.contains("(literal \"/bin/sh\")"), "{text}");
		assert!(
			text.contains(&format!(
				"(literal \"{}\")",
				script.canonicalize().unwrap().display()
			)),
			"{text}"
		);
	}
}
