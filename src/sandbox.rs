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
//! its interpreter, that interpreter's installation and the machine's runtime,
//! so every profile allows exactly that: the executed binary, the interpreter
//! its shebang names and the launcher variants the platform execs behind them,
//! the prefix each was installed under, the cartridge folder, the directories
//! on the way to it and to the working directory, and the system runtime —
//! loader, system libraries, ICU, timezone database — in both spellings,
//! written and canonical. That is the machine's own plumbing, not a
//! capability. Everything else — writes, network, other programs — is allowed
//! only where the grant names it.
//!
//! **What it does not cover.** An interpreter linked against other
//! installations — a Homebrew node reaching `/opt/homebrew/opt` — is the user's
//! package tree, granted only where `grant.read` names it.
//!
//! **The empty grant is the tightest policy.** A cartridge that declares
//! nothing can run, serve and reach sockets in its host's socket directory,
//! and reach nothing else.
//!
//! **What the OS on this platform cannot express.** `grant.net` names hosts,
//! and `sandbox-exec` accepts only `*` and `localhost` in a remote filter, so
//! a grant naming specific hosts cannot be confined host by host: a nonempty
//! `net` allows outbound network and an empty one allows none. The gap is the
//! platform's, and it is stated here rather than hidden.

use std::path::{Path, PathBuf};

use crate::loader::Grant;

#[cfg(target_os = "linux")]
#[path = "sandbox_linux.rs"]
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
	let line = String::from_utf8_lossy(
		&first[..first.len().min(crate::settings::host().sandbox_error_chars)],
	);
	let line = line.lines().next().unwrap_or("").trim_start_matches("#!");
	let program = line.split_whitespace().next()?;
	let path = PathBuf::from(program);
	if path.file_name().is_some_and(|name| name == "env") {
		return path.canonicalize().ok();
	}
	path.canonicalize().ok()
}

/// A program and the launcher variants the platform execs behind it:
/// `/bin/sh` on macOS is a launcher for `/bin/bash` or `/bin/zsh`.
fn with_variants(program: PathBuf) -> Vec<PathBuf> {
	let mut all = vec![program.clone()];
	if program == Path::new("/bin/sh") {
		for variant in ["/bin/bash", "/bin/zsh"] {
			let variant = PathBuf::from(variant);
			if variant.is_file() {
				all.push(variant);
			}
		}
	}
	all
}

/// The interpreters one shebang resolves into, with their launcher variants.
fn interpreters(binary: &Path) -> Vec<PathBuf> {
	interpreter(binary).map(with_variants).unwrap_or_default()
}

/// The installation an executable belongs to: the prefix `@executable_path/..`
/// names, when the program sits in a `bin/`. A prefix directly under the root
/// (`/bin/sh` → `/`, `/usr/bin/env` → `/usr`) is shared, not one program's.
fn installation(program: &Path) -> Option<PathBuf> {
	let dir = program.parent()?;
	if dir.file_name()? != "bin" {
		return None;
	}
	let prefix = dir.parent()?;
	(prefix.components().count() > 2).then(|| prefix.to_path_buf())
}

/// The machine's own runtime. Named as written and canonically: `/etc` and
/// `/var` are symlinks, and a child that cannot read its timezone database
/// (`/etc/localtime` → `/private/var/db/timezone`) is killed before `main`.
const RUNTIME: [&str; 7] = [
	"/usr/lib",
	"/System/Library",
	// The dyld shared cache lives in the OS cryptex on macOS 13 and later.
	"/System/Volumes/Preboot/Cryptexes",
	"/usr/share/icu",
	"/var/db/timezone",
	"/dev",
	"/etc",
];

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
		std::iter::once(root.join(".cartridge/bin").join(program))
			.chain(std::iter::once(root.join("bin").join(program)))
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
pub fn profile(grant: &Grant, root: &Path, binary: &Path, sockets: Option<&Path>) -> String {
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
			exec.extend(with_variants(path));
		}
	}
	exec.sort();
	exec.dedup();
	// A program's own installation is plumbing, not a capability.
	let mut installations: Vec<PathBuf> = exec.iter().filter_map(|p| installation(p)).collect();
	installations.sort();
	installations.dedup();
	// `*` is every program: a shell or a version-control client runs what it is told.
	if grant.exec.iter().any(|program| program == "*") {
		profile.push_str("(allow process-exec)\n");
	} else if !exec.is_empty() {
		let mut lines: Vec<String> = exec
			.iter()
			.map(|p| format!("(literal {})", literal(p)))
			.collect();
		lines.extend(
			installations
				.iter()
				.map(|p| format!("(subpath {})", literal(p))),
		);
		profile.push_str(&format!("(allow process-exec {})\n", lines.join(" ")));
	}
	// Read: the runtime a child needs to exist, the cartridge folder it was
	// declared in, and the paths it asked to read — a write is also a read.
	// The root itself is a literal, not a subpath: dyld reads the root
	// directory on the way up, and a subpath of `/` would be everything.
	let mut read: Vec<PathBuf> = RUNTIME
		.iter()
		.flat_map(|path| granted_paths(path, &root))
		.collect();
	read.push(root.to_path_buf());
	read.push(binary.to_path_buf());
	for path in grant.read.iter().chain(grant.write.iter()) {
		read.extend(granted_paths(path, &root));
	}
	read.extend(exec.iter().cloned());
	read.extend(installations.iter().cloned());
	read.sort();
	read.dedup();
	let mut lines = vec![format!("(literal {})", literal(Path::new("/")))];
	// A runtime resolves its working directory by reading every directory on
	// the way up; a literal names the directory, never its contents.
	let cwd = std::env::current_dir().unwrap_or_else(|_| root.to_path_buf());
	let mut walked: Vec<PathBuf> = root.ancestors().skip(1).map(Path::to_path_buf).collect();
	walked.extend(cwd.ancestors().map(Path::to_path_buf));
	walked.sort();
	walked.dedup();
	lines.extend(walked.iter().map(|p| format!("(literal {})", literal(p))));
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
	// Terminals: a cartridge that may write devices may open and drive a pseudo-terminal.
	if grant
		.write
		.iter()
		.any(|path| path == "/" || path.starts_with("/dev"))
	{
		profile.push_str("(allow pseudo-tty)\n(allow file-ioctl)\n");
	}
	// Network is all or nothing on this platform: a nonempty `net` allows
	// outbound connections, an empty one allows none. The hosts named in a
	// nonempty grant are not expressible in a remote filter.
	if !grant.net.is_empty() {
		profile.push_str("(allow network*)\n(allow system-socket)\n");
	}
	if let Some(sockets) = sockets {
		let mut spellings = vec![sockets.to_path_buf()];
		spellings.extend(sockets.canonicalize());
		spellings.dedup();
		let mut ancestors: Vec<String> = spellings
			.iter()
			.flat_map(|path| path.ancestors().skip(1))
			.map(|path| format!("(literal {})", literal(path)))
			.collect();
		ancestors.sort();
		ancestors.dedup();
		let within: Vec<String> = spellings
			.iter()
			.map(|path| format!("(subpath {})", literal(path)))
			.collect();
		let within = within.join(" ");
		// Socket filters only match the canonical spelling; a symlinked one voids the rule.
		let canonical = format!(
			"(subpath {})",
			literal(spellings.last().expect("a spelling"))
		);
		profile.push_str(&format!(
			"(allow file-read* {})\n(allow file-read* file-write* {within})\n(allow system-socket)\n(allow network-bind (local unix-socket {canonical}))\n(allow network-outbound (remote unix-socket {canonical}))\n",
			ancestors.join(" ")
		));
	}
	// Metadata of any path, so creating a granted directory can walk its parents;
	// POSIX and System V semaphores, which embedded stores (LMDB) use for their locks.
	profile.push_str("(allow file-read-metadata)\n(allow ipc-posix-sem)\n(allow ipc-sysv-sem)\n(allow sysctl-read)\n(allow mach-lookup)\n(allow process-fork)\n(allow signal (target children))\n");
	profile
}

/// Prepare a confined command before any cartridge code executes, including
/// discovery. Callers can attach their pipes and environment to this command;
/// the operating-system policy remains the same for every launch route.
pub fn command(
	cmd: &[String],
	grant: &Grant,
	root: &Path,
	sockets: Option<&Path>,
) -> std::io::Result<std::process::Command> {
	let binary = cmd.first().ok_or_else(|| {
		std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty cartridge command")
	})?;
	#[cfg(target_os = "macos")]
	{
		let binary = Path::new(binary);
		let text = profile(grant, root, binary, sockets);
		// The profile travels on the command line, so no file is written and no
		// file has to outlive the spawn. `ps` shows the policy; that is a
		// property of `sandbox-exec`, not a leak of what the cartridge declared.
		let mut command = std::process::Command::new(SBIN);
		command.arg("-p").arg(&text).args(cmd);
		Ok(command)
	}
	#[cfg(target_os = "linux")]
	{
		let _ = (binary, sockets);
		linux::command(cmd, grant, root)
	}
	#[cfg(not(any(target_os = "macos", target_os = "linux")))]
	{
		let _ = (binary, cmd, grant, root, sockets);
		Err(std::io::Error::new(
			std::io::ErrorKind::Unsupported,
			"no sandbox mechanism on this platform: a cartridge process is not spawned unconfined",
		))
	}
}

#[cfg(test)]
#[path = "../.cartridge/tests/unit/src/sandbox/tests.rs"]
mod tests;
