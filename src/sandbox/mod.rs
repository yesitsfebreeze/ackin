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

pub(crate) fn home_var_names() -> &'static [&'static str] {
	if cfg!(target_os = "windows") {
		&["USERPROFILE", "HOME"]
	} else {
		&["HOME", "USERPROFILE"]
	}
}

pub(crate) fn home() -> Option<PathBuf> {
	home_var_names()
		.iter()
		.find_map(std::env::var_os)
		.map(PathBuf::from)
}

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
	"/System/Volumes/Preboot/Cryptexes",
	"/usr/share/icu",
	"/var/db/timezone",
	"/dev",
	"/etc",
];

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

fn is_dev(path: &str) -> bool {
	Path::new(path).starts_with("/dev")
}

fn literal(path: &Path) -> String {
	format!(
		"\"{}\"",
		path.display()
			.to_string()
			.replace('\\', "\\\\")
			.replace('"', "\\\"")
	)
}

pub fn profile(grant: &Grant, root: &Path, binary: &Path, sockets: Option<&Path>) -> String {
	let mut profile = String::from("(version 1)\n(deny default)\n");
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
	let homes = homes();
	let mut installations: Vec<PathBuf> = exec
		.iter()
		.filter_map(|p| installation_outside(p, &homes))
		.collect();
	installations.sort();
	installations.dedup();
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
	// The root is a literal, not a subpath: dyld reads `/` on the way up, and a
	// subpath of `/` would be everything.
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
	let cwd = std::env::current_dir().unwrap_or_else(|_| root.to_path_buf());
	let mut walked: Vec<PathBuf> = root.ancestors().skip(1).map(Path::to_path_buf).collect();
	walked.extend(cwd.ancestors().map(Path::to_path_buf));
	walked.sort();
	walked.dedup();
	lines.extend(walked.iter().map(|p| format!("(literal {})", literal(p))));
	lines.extend(read.iter().map(|p| format!("(subpath {})", literal(p))));
	profile.push_str(&format!("(allow file-read* {})\n", lines.join(" ")));
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
	if grant.write.iter().any(|path| path == "/" || is_dev(path)) {
		profile.push_str("(allow pseudo-tty)\n(allow file-ioctl)\n");
	}
	if !grant.net.is_empty() {
		profile.push_str("(allow network*)\n(allow system-socket)\n");
	}
	if grant.audio {
		profile.push_str("(allow device-microphone)\n");
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
		let canonical = format!(
			"(subpath {})",
			literal(spellings.last().expect("a spelling"))
		);
		profile.push_str(&format!(
			"(allow file-read* {})\n(allow file-read* file-write* {within})\n(allow system-socket)\n(allow network-bind (local unix-socket {canonical}))\n(allow network-outbound (remote unix-socket {canonical}))\n",
			ancestors.join(" ")
		));
	}
	profile.push_str("(allow file-read-metadata)\n(allow ipc-posix-sem)\n(allow ipc-sysv-sem)\n(allow sysctl-read)\n(allow mach-lookup)\n(allow process-fork)\n(allow signal (target children))\n");
	profile
}

#[cfg(target_os = "windows")]
pub fn container_sid_for(root: &Path) -> crate::Result<String> {
	windows::container_sid_for(root)
}

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
