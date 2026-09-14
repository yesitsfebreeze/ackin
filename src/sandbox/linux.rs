//! Linux implementation: Landlock plus a seccomp socket filter, installed by
//! the `__confine` trampoline `sandbox::command` spawns in front of the node.
//!
//! The policy rides on argv, like the macOS profile: nothing is written, and
//! unlike an environment variable it does not survive into the node's
//! `environ`. The trampoline restricts itself and then `execve`s the command
//! after `--`.
//!
//! **What this platform cannot express.** Below Landlock ABI V9 (Linux 7.1)
//! a unix socket named by a path is not gated, so a node can connect to any
//! socket on the machine — another project's base socket (still behind its
//! token), the docker socket, an ssh agent. The node names the ABI it was
//! confined at on its stderr at every start below V9. A grant naming a path
//! that does not exist yet is refused: Landlock is inode-based.
//! `grant.exec: ["*"]` is refused unless `grant.read` names `/`, because
//! Landlock needs read access on a program to execute it.

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};

use landlock::{
	Access, AccessFs, CompatLevel, Compatible, LandlockStatus, PathBeneath, PathFd, Ruleset,
	RulesetAttr, RulesetCreatedAttr, RulesetStatus, Scope, ABI,
};
use seccompiler::{
	BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
	SeccompRule, TargetArch,
};

use crate::loader::Grant;

/// A grant compiled to what the kernel takes. On argv, like the macOS profile:
/// nothing on disk, and unlike env it does not survive into the node's `environ`.
#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct Policy {
	read: Vec<PathBuf>,
	write: Vec<PathBuf>,
	exec: Vec<PathBuf>,
	net: bool,
}

/// The loader, system libraries and their data. Executable only where the
/// loader lives — `/usr/bin` is absent, so an ungranted program still cannot run.
const RUNTIME_READ: [&str; 6] = [
	"/lib",
	"/lib64",
	"/usr/lib",
	"/usr/lib64",
	"/usr/share",
	"/etc",
];
const RUNTIME_EXEC: [&str; 4] = ["/lib", "/lib64", "/usr/lib", "/usr/lib64"];

pub(super) fn command(
	cmd: &[String],
	grant: &Grant,
	root: &Path,
	sockets: Option<&Path>,
) -> std::io::Result<std::process::Command> {
	let binary = Path::new(&cmd[0]).canonicalize()?;
	// `*` is every program, and on Linux that also means reading every program:
	// Landlock needs read access to execute. The refusal names the honest grant.
	let every = grant.exec.iter().any(|program| program == "*");
	if every && !grant.read.iter().any(|path| path == "/") {
		return Err(std::io::Error::new(
			std::io::ErrorKind::Unsupported,
			"grant.exec `*` on Linux also grants reading every file: Landlock needs \
			 read access on a program to execute it. Declare `read: [\"/\"]` beside it.",
		));
	}
	let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
	// Exec: the node's own binary, the interpreter its shebang names and the
	// launcher variants the platform execs behind it, what the grant named,
	// each one's installation, and the loader's directories.
	let mut exec: Vec<PathBuf> = vec![binary.clone()];
	exec.extend(super::interpreters(&binary));
	for program in &grant.exec {
		if program != "*" {
			if let Some(path) = super::granted_exec(program, &root) {
				exec.extend(super::with_variants(path));
			}
		}
	}
	// A program's own installation is plumbing, not a capability.
	let mut installations: Vec<PathBuf> =
		exec.iter().filter_map(|p| super::installation(p)).collect();
	installations.sort();
	installations.dedup();
	exec.extend(installations.iter().cloned());
	exec.extend(
		RUNTIME_EXEC
			.iter()
			// Runtime entries this distribution lacks are skipped; a grant entry
			// is never skipped, so an unopenable one refuses.
			.filter(|path| Path::new(*path).exists())
			.map(PathBuf::from),
	);
	if every {
		exec.push(PathBuf::from("/"));
	}
	exec.sort();
	exec.dedup();
	// Read: the machine's runtime, the cartridge folder, the paths the grant
	// names — a write is also a read — the socket directory and the installations.
	let mut read: Vec<PathBuf> = RUNTIME_READ
		.iter()
		.filter(|path| Path::new(*path).exists())
		.map(PathBuf::from)
		.collect();
	read.push("/dev".into());
	read.push("/proc/self".into());
	read.push(root.clone());
	for path in grant.read.iter().chain(grant.write.iter()) {
		read.extend(super::granted_paths(path, &root));
	}
	read.extend(installations);
	// Write: only what the grant names, plus the socket directory and the
	// devices a process cannot exist without.
	let mut write: Vec<PathBuf> = grant
		.write
		.iter()
		.flat_map(|path| super::granted_paths(path, &root))
		.collect();
	write.extend(
		[
			"/dev/null",
			"/dev/zero",
			"/dev/full",
			"/dev/random",
			"/dev/urandom",
			"/dev/tty",
		]
		.into_iter()
		.map(PathBuf::from),
	);
	// Terminals: a cartridge that may write devices may drive a pseudo-terminal.
	if grant
		.write
		.iter()
		.any(|path| path == "/" || path.starts_with("/dev"))
	{
		write.push("/dev/ptmx".into());
		write.push("/dev/pts".into());
	}
	if let Some(sockets) = sockets {
		let sockets = sockets
			.canonicalize()
			.unwrap_or_else(|_| sockets.to_path_buf());
		read.push(sockets.clone());
		write.push(sockets);
	}
	read.sort();
	read.dedup();
	write.sort();
	write.dedup();
	let policy = Policy {
		read,
		write,
		exec,
		net: !grant.net.is_empty(),
	};
	let text = serde_json::to_string(&policy).map_err(std::io::Error::other)?;
	// `MAX_ARG_STRLEN` is 128 KiB; above this the kernel would answer an
	// opaque `E2BIG`, so the refusal names the real reason.
	if text.len() > 120_000 {
		return Err(std::io::Error::new(
			std::io::ErrorKind::InvalidInput,
			"the compiled policy does not fit on the command line; split the grant",
		));
	}
	// The trampoline is the base binary itself: it restricts and `execve`s, and
	// `cmd[0]` — not `current_exe()` — is the one caller's binary even under
	// the test harness.
	let mut command = std::process::Command::new(&cmd[0]);
	command.arg("__confine").arg(&text).arg("--").args(cmd);
	Ok(command)
}

/// The Landlock rules a policy compiles to, opened by hand: `path_beneath_rules`
/// drops a path it cannot open, and a granted path that is not there must be a
/// refusal, not a silently ungranted rule.
fn rules(policy: &Policy) -> crate::Result<Vec<PathBeneath<PathFd>>> {
	let mut rules = Vec::new();
	for (paths, access) in [
		// A Landlock read carries Execute; strip it, or `grant.read` becomes
		// `grant.exec`.
		(
			&policy.read,
			AccessFs::from_read(ABI::V1) & !AccessFs::Execute,
		),
		(
			&policy.write,
			AccessFs::from_all(ABI::V1) & !AccessFs::Execute,
		),
		// Execute alone gets EACCES: the program and the loader must be readable.
		(&policy.exec, AccessFs::Execute | AccessFs::ReadFile),
	] {
		for path in paths {
			let fd = PathFd::new(path)
				.map_err(|e| crate::Error::file(path, std::io::Error::other(e)))?;
			rules.push(PathBeneath::new(fd, access));
		}
	}
	Ok(rules)
}

/// Network is all or nothing, as on macOS: `grant.net` names hosts and no kernel
/// policy filters by host. The socket domain decides; every other domain is
/// `EPERM`.
fn filter(net: bool) -> crate::Result<BpfProgram> {
	let seccomp = |error: &dyn std::fmt::Display| crate::Error::process("seccomp", error);
	let arch = match std::env::consts::ARCH {
		"x86_64" => TargetArch::x86_64,
		"aarch64" => TargetArch::aarch64,
		"riscv64" => TargetArch::riscv64,
		// No filter would be a silent hole.
		other => {
			return Err(crate::Error::process(
				"seccomp",
				format!("unsupported architecture {other}"),
			))
		}
	};
	let allowed: &[i32] = match net {
		false => &[libc::AF_UNIX],
		true => &[libc::AF_UNIX, libc::AF_INET, libc::AF_INET6],
	};
	// Conditions in one rule are ANDed: "none of the allowed domains".
	let conditions = allowed
		.iter()
		.map(|domain| {
			SeccompCondition::new(0, SeccompCmpArgLen::Dword, SeccompCmpOp::Ne, *domain as u64)
		})
		.collect::<Result<Vec<_>, _>>()
		.map_err(|error| seccomp(&error))?;
	let rule = SeccompRule::new(conditions).map_err(|error| seccomp(&error))?;
	let filter = SeccompFilter::new(
		[
			(libc::SYS_socket, vec![rule.clone()]),
			(libc::SYS_socketpair, vec![rule]),
		]
		.into_iter()
		.collect(),
		SeccompAction::Allow, // everything else is Landlock's job
		SeccompAction::Errno(libc::EPERM as u32),
		arch,
	)
	.map_err(|error| seccomp(&error))?;
	filter
		.try_into()
		.map_err(|error| seccomp(&format!("{error:?}")))
}

/// Restrict this process to `policy`, then become `cmd`. Returns only on
/// failure: a partial policy is never a silent fallback.
pub(super) fn confine(policy: &str, cmd: &[String]) -> crate::Result<std::convert::Infallible> {
	let policy: Policy = serde_json::from_str(policy)?;
	let landlock = |error: landlock::RulesetError| crate::Error::process("landlock", error);
	// The floor is ABI V1 as a hard requirement; everything above it is best
	// effort, and what the kernel actually enforced is reported below.
	let mut created = Ruleset::default()
		.set_compatibility(CompatLevel::HardRequirement)
		.handle_access(AccessFs::from_all(ABI::V1))
		.map_err(landlock)?
		.set_compatibility(CompatLevel::BestEffort)
		.handle_access(AccessFs::from_all(ABI::V9))
		.map_err(landlock)?
		.scope(Scope::from_all(ABI::V6))
		.map_err(landlock)?
		.create()
		.map_err(landlock)?;
	for rule in rules(&policy)? {
		created = created.add_rule(rule).map_err(landlock)?;
	}
	let status = created.restrict_self().map_err(landlock)?;
	if status.ruleset == RulesetStatus::NotEnforced || !status.no_new_privs {
		return Err(crate::Error::process(
			"landlock",
			"not enforced on this kernel: refusing to start the cartridge unconfined",
		));
	}
	// The host pumps the child's stderr to diagnostics, so the ABI a node
	// actually runs under is on record at every start below V9.
	if let LandlockStatus::Available { effective_abi, .. } = status.landlock {
		if effective_abi < ABI::V9 {
			eprintln!(
				"cartridge: confined at Landlock ABI {effective_abi:?}; unix sockets named \
				 by a path are ungated below V9 (Linux 7.1)"
			);
		}
	}
	seccompiler::apply_filter_all_threads(&filter(policy.net)?)
		.map_err(|error| crate::Error::process("seccomp", error))?;
	Err(crate::Error::process(
		cmd[0].as_str(),
		std::process::Command::new(&cmd[0]).args(&cmd[1..]).exec(),
	))
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/sandbox/linux.rs"]
mod tests;
