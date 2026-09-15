//! A cartridge's `grant` is compiled into an operating-system policy and the
//! child is spawned inside it, so a cartridge reaching outside what it
//! declared is stopped by the OS rather than by review. A child that cannot
//! be confined on the running platform is refused, never spawned unconfined —
//! and a kernel that enforces less than the policy asks for says so on the
//! child's stderr.
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
//! installations — a Homebrew python reaching `/opt/homebrew/opt` — is the user's
//! package tree, granted only where `grant.read` names it.
//!
//! **The empty grant is the tightest policy.** A cartridge that declares
//! nothing can run, serve and reach sockets in its host's socket directory,
//! and reach nothing else.
//!
//! **What the OS on this platform cannot express.** `grant.net` names hosts,
//! and `sandbox-exec` accepts only `*` and `localhost` in a remote filter, so
//! a grant naming specific hosts cannot be confined host by host: a nonempty
//! `net` allows outbound network and an empty one allows none. On Linux the
//! same all-or-nothing rule is a seccomp filter on the socket call's domain.

use std::path::{Path, PathBuf};

use crate::loader::Grant;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
const SBIN: &str = "/usr/bin/sandbox-exec";

/// A clause written through a symlinked prefix (`/tmp`, `/var`) matches
/// nothing: only the canonical spelling opens. The spelling a child
/// constructs is its own choice, so both are rendered — resolved when the
/// path exists, through its deepest existing ancestor when it does not yet.
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
	// The deepest existing ancestor resolved, the rest appended: the file a
	// cartridge creates is still `/private/...`.
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

fn interpreter(binary: &Path) -> Option<PathBuf> {
	use std::io::Read;
	// Only the shebang matters, so only the first `sandbox_error_chars` bytes
	// are read: the whole binary is tens of megabytes on every node start.
	let limit = crate::settings::host().sandbox_error_chars as u64;
	let mut first = Vec::new();
	std::fs::File::open(binary)
		.ok()?
		.take(limit)
		.read_to_end(&mut first)
		.ok()?;
	if !first.starts_with(b"#!") {
		return None;
	}
	let line = String::from_utf8_lossy(&first);
	let line = line.lines().next().unwrap_or("").trim_start_matches("#!");
	let program = line.split_whitespace().next()?;
	let path = PathBuf::from(program);
	if path.file_name().is_some_and(|name| name == "env") {
		// Allowed as itself: resolving to what it would run would mean
		// allowing every directory on PATH.
		return path.canonicalize().ok();
	}
	path.canonicalize().ok()
}

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

fn interpreters(binary: &Path) -> Vec<PathBuf> {
	interpreter(binary).map(with_variants).unwrap_or_default()
}

/// `%USERPROFILE%` leads on Windows: the `PASSTHROUGH` list in `host::process`
/// does not carry `HOME` to a node at all, and a shell like Git Bash sets it
/// to an MSYS path naming somewhere else.
pub(crate) fn home_var_names() -> &'static [&'static str] {
	if cfg!(target_os = "windows") {
		&["USERPROFILE", "HOME"]
	} else {
		&["HOME", "USERPROFILE"]
	}
}

/// The one resolver every caller shares. Not `crate::trust::home()`, which is
/// `$CARTRIDGE_HOME` — a store under this directory, not this directory.
pub(crate) fn home() -> Option<PathBuf> {
	home_var_names()
		.iter()
		.find_map(std::env::var_os)
		.map(PathBuf::from)
}

/// Every home that is set is tested, since a Windows shell can set `HOME`
/// beside the `USERPROFILE` it ignores, and both sides are canonical so a
/// verbatim `\\?\` spelling still compares equal to a plain one.
pub(crate) fn homes() -> Vec<PathBuf> {
	home_var_names()
		.iter()
		.filter_map(std::env::var_os)
		.map(PathBuf::from)
		.map(|home| home.canonicalize().unwrap_or(home))
		.collect()
}

pub(crate) fn installation_outside(program: &Path, homes: &[PathBuf]) -> Option<PathBuf> {
	let dir = program.parent()?;
	if dir.file_name()? != "bin" {
		return None;
	}
	let prefix = dir.parent()?;
	// A prefix directly under the root (`/bin/sh` → `/`, `/usr/bin/env` →
	// `/usr`) is shared, not one program's.
	if prefix.components().count() <= 2 {
		return None;
	}
	let prefix = prefix
		.canonicalize()
		.unwrap_or_else(|_| prefix.to_path_buf());
	match homes.iter().any(|home| home.starts_with(&prefix)) {
		true => None,
		false => Some(prefix),
	}
}

/// Named as written and canonically: `/etc` and `/var` are symlinks, and a
/// child that cannot read `/etc/localtime` is killed before `main`.
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

/// Resolved the way the host resolves a cartridge's own binary. Unresolvable
/// entries build no line — the OS denies what was never allowed.
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
	let extensions = pathext();
	candidates
		.into_iter()
		.flat_map(|candidate| spellings(candidate, &extensions))
		.find(|p| p.is_file())
		.and_then(|p| p.canonicalize().ok())
}

/// The extensions a bare program name may be spelled with on Windows, where
/// `git` on PATH is the file `git.exe` and `is_file()` on the bare name is
/// false. `PATHEXT` is only guaranteed for a process started from cmd or
/// PowerShell, and the host is normally launched by an MCP client, so an unset
/// one falls back to Windows's own defaults rather than skipping the probe.
fn pathext() -> Vec<String> {
	if !cfg!(target_os = "windows") {
		return Vec::new();
	}
	let list = std::env::var_os("PATHEXT").unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into());
	std::env::split_paths(&list)
		.map(|ext| ext.to_string_lossy().trim_start_matches('.').to_owned())
		.collect()
}

fn spellings(candidate: PathBuf, extensions: &[String]) -> Vec<PathBuf> {
	// A name already spelled with one of them is tried as itself only.
	// `extension().is_some()` cannot tell that from a dotted name like
	// `python3.12`, whose "extension" is `12`, and which still needs every
	// extension probed to find `python3.12.exe`.
	let spelled = candidate.extension().is_some_and(|ext| {
		extensions
			.iter()
			.any(|pathext| pathext.eq_ignore_ascii_case(&ext.to_string_lossy()))
	});
	if extensions.is_empty() || spelled {
		return vec![candidate];
	}
	let name = candidate
		.file_name()
		.unwrap_or_default()
		.to_string_lossy()
		.into_owned();
	let mut spellings: Vec<PathBuf> = extensions
		.iter()
		.map(|ext| candidate.with_file_name(format!("{name}.{ext}")))
		.collect();
	spellings.push(candidate);
	spellings
}

/// Compared path component by path component rather than as a string prefix:
/// `/devices` and `/development` start with the four characters `/dev` but are
/// not under it, and a grant naming them must not pick up device access
/// nothing asked for.
fn is_dev(path: &str) -> bool {
	Path::new(path).starts_with("/dev")
}

/// A path is data here, and a quote or backslash inside a declared name must
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

/// Every line the grant does not ask for is absent, and `deny default` is
/// what remains.
pub fn profile(grant: &Grant, root: &Path, binary: &Path, sockets: Option<&Path>) -> String {
	let mut profile = String::from("(version 1)\n(deny default)\n");
	// Paths the OS resolves through symlinks must be named canonically: a
	// grant written through `/tmp` would never match the file it names.
	let binary = binary
		.canonicalize()
		.unwrap_or_else(|_| binary.to_path_buf());
	let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
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
	let homes = homes();
	let mut installations: Vec<PathBuf> = exec
		.iter()
		.filter_map(|p| installation_outside(p, &homes))
		.collect();
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
	// A write is also a read. The root itself is a literal, not a subpath:
	// dyld reads the root directory on the way up, and a subpath of `/`
	// would be everything.
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
	// `/dev/null` is plumbing, not a capability: git, shells and runtimes
	// discard output to it without any grant naming it, so a program allowed
	// to run at all may open it.
	let writes: Vec<String> =
		std::iter::once(format!("(literal {})", literal(Path::new("/dev/null"))))
			.chain(
				grant
					.write
					.iter()
					.flat_map(|path| granted_paths(path, &root))
					.map(|p| format!("(subpath {})", literal(&p))),
			)
			.collect();
	profile.push_str(&format!("(allow file-write* {})\n", writes.join(" ")));
	// Terminals: a cartridge that may write devices may open and drive a pseudo-terminal.
	if grant.write.iter().any(|path| path == "/" || is_dev(path)) {
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

#[cfg(target_os = "windows")]
pub fn container_sid_for(root: &Path) -> crate::Result<String> {
	windows::container_sid_for(root)
}

/// Confines before any cartridge code executes, including discovery.
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
		let _ = binary;
		linux::command(cmd, grant, root, sockets)
	}
	#[cfg(target_os = "windows")]
	{
		let _ = binary;
		windows::command(cmd, grant, root, sockets)
	}
	#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
	{
		let _ = (binary, cmd, grant, root, sockets);
		Err(std::io::Error::new(
			std::io::ErrorKind::Unsupported,
			"no sandbox mechanism on this platform: a cartridge process is not spawned unconfined",
		))
	}
}

pub fn confine(policy: &str, cmd: &[String]) -> crate::Result<std::convert::Infallible> {
	#[cfg(target_os = "linux")]
	{
		linux::confine(policy, cmd)
	}
	#[cfg(target_os = "windows")]
	{
		windows::confine(policy, cmd)
	}
	#[cfg(not(any(target_os = "linux", target_os = "windows")))]
	{
		let _ = (policy, cmd);
		Err(crate::Error::Argument(
			"__confine is the Linux and Windows trampoline; this platform confines at spawn".into(),
		))
	}
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/sandbox/tests.rs"]
mod tests;
