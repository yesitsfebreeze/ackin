//! Windows implementation: an AppContainer, entered by the `__confine`
//! trampoline `sandbox::command` spawns in front of the node.
//!
//! The policy rides on argv, like the macOS profile and the Linux one:
//! nothing is written, and unlike an environment variable it does not survive
//! into the node's environment.
//!
//! **Why a trampoline here too.** An AppContainer is a property of the token a
//! process is created with, not something a running process can enter, so it
//! cannot be applied the way Landlock restricts the caller. It has to be
//! passed to `CreateProcessW` through a proc-thread attribute list, which
//! `std::process::Command` cannot express on stable Rust. The trampoline is
//! the base binary: it grants, creates the confined child, waits for it, and
//! exits with the child's code. Unlike the Linux trampoline it does not
//! disappear into the node — it stays as the node's parent for the child's
//! whole life, because the grants below have to be taken back.
//!
//! **What this platform cannot express.** `grant.net` names hosts, and an
//! AppContainer capability is all-or-nothing per capability, so a non-empty
//! `grant.net` means the `internetClient` and `privateNetworkClientServer`
//! capabilities and an empty one means neither. That is the same gap the
//! other two platforms have, in the same place.
//!
//! **What this platform does that the others do not.** Landlock and
//! `sandbox-exec` are pure: the policy lives in the process and nothing on
//! disk changes. An AppContainer grants by writing an ACE for the container's
//! SID onto each granted path, so confining here *mutates the filesystem*.
//! The trampoline takes every ACE back when the child exits. A trampoline
//! that is killed outright leaks its ACEs, so the container name is derived
//! from the cartridge root and is therefore stable: the next start of the
//! same cartridge re-grants the same ACEs to the same SID rather than
//! accumulating new ones, and the leak is bounded by the number of granted
//! paths, not by the number of starts.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, HLOCAL, WAIT_FAILED};
use windows_sys::Win32::Security::Authorization::{
	GetNamedSecurityInfoW, SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W,
	GRANT_ACCESS, NO_MULTIPLE_TRUSTEE, REVOKE_ACCESS, SE_FILE_OBJECT, TRUSTEE_IS_GROUP,
	TRUSTEE_IS_SID, TRUSTEE_W,
};
use windows_sys::Win32::Security::Isolation::{
	CreateAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
};
use windows_sys::Win32::Security::{
	DeriveCapabilitySidsFromName, FreeSid, ACL, DACL_SECURITY_INFORMATION, PSID,
	SECURITY_CAPABILITIES, SID_AND_ATTRIBUTES, SUB_CONTAINERS_AND_OBJECTS_INHERIT,
};
use windows_sys::Win32::Storage::FileSystem::{
	FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
};
use windows_sys::Win32::System::Console::{
	GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
use windows_sys::Win32::System::SystemServices::SE_GROUP_ENABLED;
use windows_sys::Win32::System::Threading::{
	CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess,
	InitializeProcThreadAttributeList, UpdateProcThreadAttribute, WaitForSingleObject,
	EXTENDED_STARTUPINFO_PRESENT, INFINITE, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION,
	PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, STARTF_USESTDHANDLES, STARTUPINFOEXW,
};

use crate::loader::Grant;

/// A grant compiled to what this platform takes. On argv, like the macOS
/// profile: nothing on disk, and unlike env it does not survive into the
/// node's environment.
#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct Policy {
	read: Vec<PathBuf>,
	write: Vec<PathBuf>,
	exec: Vec<PathBuf>,
	net: bool,
	/// The AppContainer moniker, derived from the cartridge root so that the
	/// same cartridge always confines into the same container.
	container: String,
}

/// The capabilities a non-empty `grant.net` buys. Outbound to the internet and
/// to the local network; an AppContainer has no network at all without them.
const NET_CAPABILITIES: [&str; 2] = ["internetClient", "privateNetworkClientServer"];

/// A nul-terminated UTF-16 string, as every `W` entry point wants it.
fn wide(text: impl AsRef<OsStr>) -> Vec<u16> {
	text.as_ref().encode_wide().chain(Some(0)).collect()
}

pub(super) fn command(
	cmd: &[String],
	grant: &Grant,
	root: &Path,
	sockets: Option<&Path>,
) -> std::io::Result<std::process::Command> {
	let binary = Path::new(&cmd[0]).canonicalize()?;
	let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
	// Exec: the node's own binary, what the grant named, and each one's
	// installation. There is no loader directory to add: the system image is
	// already readable to every AppContainer through ALL APPLICATION PACKAGES.
	let mut exec: Vec<PathBuf> = vec![binary.clone()];
	exec.extend(super::interpreters(&binary));
	for program in &grant.exec {
		if program != "*" {
			if let Some(path) = super::granted_exec(program, &root) {
				exec.extend(super::with_variants(path));
			}
		}
	}
	let mut installations: Vec<PathBuf> =
		exec.iter().filter_map(|p| super::installation(p)).collect();
	installations.sort();
	installations.dedup();
	exec.extend(installations.iter().cloned());
	exec.sort();
	exec.dedup();
	// Read: the cartridge folder, what the grant names — a write is also a
	// read — the socket directory and the installations.
	let mut read: Vec<PathBuf> = vec![root.clone()];
	for path in grant.read.iter().chain(grant.write.iter()) {
		read.extend(super::granted_paths(path, &root));
	}
	read.extend(installations);
	let mut write: Vec<PathBuf> = grant
		.write
		.iter()
		.flat_map(|path| super::granted_paths(path, &root))
		.collect();
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
		container: container_name(&root),
	};
	let text = serde_json::to_string(&policy).map_err(std::io::Error::other)?;
	// A command line is capped at 32767 characters; above it `CreateProcessW`
	// would answer an opaque failure, so the refusal names the real reason.
	if text.len() > 30_000 {
		return Err(std::io::Error::new(
			std::io::ErrorKind::InvalidInput,
			"the compiled policy does not fit on the command line; split the grant",
		));
	}
	let mut command = std::process::Command::new(&cmd[0]);
	command.arg("__confine").arg(&text).arg("--").args(cmd);
	Ok(command)
}

/// The container moniker for a cartridge root. Stable across starts, so the
/// ACEs a leaked trampoline left behind are re-granted rather than added to.
/// An AppContainer name is capped at 64 characters; this is 26.
fn container_name(root: &Path) -> String {
	format!("cartridge-{}", crate::transport::typed::path_tag(root))
}

/// A SID that is freed when it goes out of scope. Both of the derive entry
/// points hand back a SID the caller owns.
struct Sid(PSID);

impl Drop for Sid {
	fn drop(&mut self) {
		if !self.0.is_null() {
			// SAFETY: the SID came from a derive call that transfers ownership,
			// and nothing else frees it.
			unsafe { FreeSid(self.0) };
		}
	}
}

/// The container's SID, creating the profile the first time this cartridge is
/// confined. An existing profile is not an error: the moniker is deliberately
/// stable, so every start after the first lands here.
fn container_sid(name: &str) -> crate::Result<Sid> {
	let name = wide(name);
	let mut psid: PSID = std::ptr::null_mut();
	// SAFETY: `name` outlives the call and `psid` is a valid out parameter.
	let created = unsafe {
		CreateAppContainerProfile(
			name.as_ptr(),
			name.as_ptr(),
			name.as_ptr(),
			std::ptr::null(),
			0,
			&mut psid,
		)
	};
	if created >= 0 {
		return Ok(Sid(psid));
	}
	// SAFETY: same contract; the profile exists, so derive its SID instead.
	let derived = unsafe { DeriveAppContainerSidFromAppContainerName(name.as_ptr(), &mut psid) };
	if derived < 0 {
		return Err(crate::Error::process(
			"appcontainer",
			format!("cannot create or derive the container profile: 0x{derived:08x}"),
		));
	}
	Ok(Sid(psid))
}

/// The SIDs for one named capability. `DeriveCapabilitySidsFromName` hands back
/// two `LocalAlloc`ed arrays; only the capability SIDs are wanted, and both
/// arrays and every SID in them belong to the caller.
fn capability_sids(name: &str) -> Vec<PSID> {
	let name = wide(name);
	let mut group_sids: *mut PSID = std::ptr::null_mut();
	let mut group_count: u32 = 0;
	let mut cap_sids: *mut PSID = std::ptr::null_mut();
	let mut cap_count: u32 = 0;
	// SAFETY: every out parameter is a valid place and `name` outlives the call.
	let ok = unsafe {
		DeriveCapabilitySidsFromName(
			name.as_ptr(),
			&mut group_sids,
			&mut group_count,
			&mut cap_sids,
			&mut cap_count,
		)
	};
	let mut out = Vec::new();
	if ok != 0 {
		for index in 0..cap_count as isize {
			// SAFETY: the call reported `cap_count` entries in `cap_sids`.
			out.push(unsafe { *cap_sids.offset(index) });
		}
		// The group array is not used here, so its SIDs and the array go back now.
		// SAFETY: both arrays were `LocalAlloc`ed by the call above.
		unsafe {
			for index in 0..group_count as isize {
				FreeSid(*group_sids.offset(index));
			}
			if !group_sids.is_null() {
				LocalFree(group_sids as HLOCAL);
			}
			if !cap_sids.is_null() {
				LocalFree(cap_sids as HLOCAL);
			}
		}
	}
	out
}

/// Grant or revoke one access mask for `sid` on `path`. Windows has no way to
/// say "this path and nothing under it" for a directory a child must traverse,
/// so a directory grant inherits: the ACE is written with
/// `SUB_CONTAINERS_AND_OBJECTS_INHERIT`, which is the same shape as Landlock's
/// path-beneath rule and the seatbelt's subpath clause.
fn ace(path: &Path, sid: PSID, rights: u32, grant: bool) -> crate::Result<()> {
	let name = wide(path);
	let mut dacl: *mut ACL = std::ptr::null_mut();
	let mut descriptor = std::ptr::null_mut();
	// SAFETY: `name` outlives the call; the rest are valid out parameters. The
	// descriptor is `LocalAlloc`ed and freed below.
	let read = unsafe {
		GetNamedSecurityInfoW(
			name.as_ptr(),
			SE_FILE_OBJECT,
			DACL_SECURITY_INFORMATION,
			std::ptr::null_mut(),
			std::ptr::null_mut(),
			&mut dacl,
			std::ptr::null_mut(),
			&mut descriptor,
		)
	};
	if read != 0 {
		return Err(crate::Error::file(
			path,
			std::io::Error::from_raw_os_error(read as i32),
		));
	}
	let entry = EXPLICIT_ACCESS_W {
		grfAccessPermissions: rights,
		grfAccessMode: if grant { GRANT_ACCESS } else { REVOKE_ACCESS },
		grfInheritance: SUB_CONTAINERS_AND_OBJECTS_INHERIT,
		Trustee: TRUSTEE_W {
			pMultipleTrustee: std::ptr::null_mut(),
			MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
			TrusteeForm: TRUSTEE_IS_SID,
			TrusteeType: TRUSTEE_IS_GROUP,
			ptstrName: sid as *mut u16,
		},
	};
	let mut merged: *mut ACL = std::ptr::null_mut();
	// SAFETY: one entry is described, the old DACL came from the read above,
	// and `merged` is `LocalAlloc`ed for the caller to free.
	let built = unsafe { SetEntriesInAclW(1, &entry, dacl, &mut merged) };
	if built != 0 {
		// SAFETY: the descriptor was allocated by `GetNamedSecurityInfoW`.
		unsafe { LocalFree(descriptor as HLOCAL) };
		return Err(crate::Error::file(
			path,
			std::io::Error::from_raw_os_error(built as i32),
		));
	}
	// SAFETY: `merged` is a well-formed DACL and `name` outlives the call.
	let written = unsafe {
		SetNamedSecurityInfoW(
			name.as_ptr(),
			SE_FILE_OBJECT,
			DACL_SECURITY_INFORMATION,
			std::ptr::null_mut(),
			std::ptr::null_mut(),
			merged,
			std::ptr::null_mut(),
		)
	};
	// SAFETY: both blocks were `LocalAlloc`ed by the calls above.
	unsafe {
		LocalFree(merged as HLOCAL);
		LocalFree(descriptor as HLOCAL);
	}
	if written != 0 {
		return Err(crate::Error::file(
			path,
			std::io::Error::from_raw_os_error(written as i32),
		));
	}
	Ok(())
}

/// Every (path, rights) pair a policy grants. Read carries no execute, so
/// `grant.read` does not become `grant.exec`; a write is also a read, matching
/// what the document promises and what the other two platforms do.
fn grants(policy: &Policy) -> Vec<(&Path, u32)> {
	let mut out = Vec::new();
	for path in &policy.read {
		out.push((path.as_path(), FILE_GENERIC_READ));
	}
	for path in &policy.write {
		out.push((path.as_path(), FILE_GENERIC_READ | FILE_GENERIC_WRITE));
	}
	for path in &policy.exec {
		out.push((path.as_path(), FILE_GENERIC_READ | FILE_GENERIC_EXECUTE));
	}
	out
}

/// Restrict `cmd` to `policy` and become its parent. Unlike the Linux
/// trampoline this one returns only on failure *to start*: once the child is
/// running it waits, takes its grants back and exits with the child's code.
pub(super) fn confine(policy: &str, cmd: &[String]) -> crate::Result<std::convert::Infallible> {
	let policy: Policy = serde_json::from_str(policy)?;
	let sid = container_sid(&policy.container)?;
	// A path a grant names must exist: an ACE cannot be written to something
	// that is not there, and silently dropping the rule would be a hole.
	let wanted = grants(&policy);
	let mut granted: Vec<(&Path, u32)> = Vec::new();
	let started = (|| -> crate::Result<PROCESS_INFORMATION> {
		for (path, rights) in &wanted {
			ace(path, sid.0, *rights, true)?;
			granted.push((path, *rights));
		}
		spawn(&policy, sid.0, cmd)
	})();
	// Whatever happened, the ACEs written above come back off. A path that was
	// never granted is not revoked, so a failure part way through leaves the
	// filesystem as it was found.
	let process = match started {
		Ok(process) => process,
		Err(error) => {
			revoke(&granted, sid.0);
			return Err(error);
		}
	};
	// SAFETY: the child started, so both handles are open; the thread handle
	// is not used and goes back immediately.
	unsafe { CloseHandle(process.hThread) };
	// SAFETY: `hProcess` is open until closed below.
	let waited = unsafe { WaitForSingleObject(process.hProcess, INFINITE) };
	let mut code: u32 = 1;
	if waited != WAIT_FAILED {
		// SAFETY: the process has exited, so its code is final.
		unsafe { GetExitCodeProcess(process.hProcess, &mut code) };
	}
	// SAFETY: nothing uses the handle after this.
	unsafe { CloseHandle(process.hProcess) };
	revoke(&granted, sid.0);
	std::process::exit(code as i32)
}

/// Take back every ACE this trampoline wrote. A revoke that fails is reported
/// and does not stop the others: one path left granted must not leave the rest
/// granted too.
fn revoke(granted: &[(&Path, u32)], sid: PSID) {
	for (path, rights) in granted {
		if let Err(error) = ace(path, sid, *rights, false) {
			eprintln!("cartridge: {} stayed granted: {error}", path.display());
		}
	}
}

/// Create the confined child. The trampoline's own standard handles are the
/// child's, so the host's pipes reach the node through this process without
/// it reading a byte of them.
fn spawn(policy: &Policy, sid: PSID, cmd: &[String]) -> crate::Result<PROCESS_INFORMATION> {
	let fail = |what: &str| {
		crate::Error::process(
			"appcontainer",
			format!("{what}: {}", std::io::Error::last_os_error()),
		)
	};
	// The capability SIDs outlive the call: `SECURITY_CAPABILITIES` borrows them.
	let mut capabilities: Vec<SID_AND_ATTRIBUTES> = Vec::new();
	let mut owned: Vec<PSID> = Vec::new();
	if policy.net {
		for name in NET_CAPABILITIES {
			owned.extend(capability_sids(name));
		}
		capabilities.extend(owned.iter().map(|sid| SID_AND_ATTRIBUTES {
			Sid: *sid,
			Attributes: SE_GROUP_ENABLED as u32,
		}));
	}
	let mut security = SECURITY_CAPABILITIES {
		AppContainerSid: sid,
		Capabilities: if capabilities.is_empty() {
			std::ptr::null_mut()
		} else {
			capabilities.as_mut_ptr()
		},
		CapabilityCount: capabilities.len() as u32,
		Reserved: 0,
	};
	// The attribute list is sized by asking, then allocated as bytes.
	let mut size: usize = 0;
	// SAFETY: the first call is documented to fail and report the size it needs.
	unsafe { InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut size) };
	if size == 0 {
		return Err(fail("cannot size the attribute list"));
	}
	let mut buffer = vec![0u8; size];
	let list = buffer.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;
	// SAFETY: `buffer` is exactly the size the call above asked for.
	if unsafe { InitializeProcThreadAttributeList(list, 1, 0, &mut size) } == 0 {
		return Err(fail("cannot initialise the attribute list"));
	}
	// SAFETY: `security` outlives the `CreateProcessW` call below, which is
	// what the attribute list borrows it for.
	let updated = unsafe {
		UpdateProcThreadAttribute(
			list,
			0,
			PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
			&mut security as *mut _ as *const core::ffi::c_void,
			std::mem::size_of::<SECURITY_CAPABILITIES>(),
			std::ptr::null_mut(),
			std::ptr::null(),
		)
	};
	if updated == 0 {
		// SAFETY: the list was initialised above.
		unsafe { DeleteProcThreadAttributeList(list) };
		return Err(fail("cannot set the container on the attribute list"));
	}
	let mut startup = STARTUPINFOEXW::default();
	startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
	startup.lpAttributeList = list;
	startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
	// SAFETY: the standard handles belong to this process and are inherited,
	// not owned, by the child.
	unsafe {
		startup.StartupInfo.hStdInput = GetStdHandle(STD_INPUT_HANDLE);
		startup.StartupInfo.hStdOutput = GetStdHandle(STD_OUTPUT_HANDLE);
		startup.StartupInfo.hStdError = GetStdHandle(STD_ERROR_HANDLE);
	}
	let mut line = wide(command_line(cmd));
	let mut process = PROCESS_INFORMATION::default();
	// SAFETY: `line` is mutable and nul-terminated as `CreateProcessW` requires,
	// and every other pointer is either null or outlives the call.
	let created = unsafe {
		CreateProcessW(
			std::ptr::null(),
			line.as_mut_ptr(),
			std::ptr::null(),
			std::ptr::null(),
			1,
			EXTENDED_STARTUPINFO_PRESENT,
			std::ptr::null(),
			std::ptr::null(),
			&startup as *const _ as *const _,
			&mut process,
		)
	};
	// SAFETY: the list has done its work whether or not the child started.
	unsafe { DeleteProcThreadAttributeList(list) };
	for sid in owned {
		// SAFETY: each came from `DeriveCapabilitySidsFromName`.
		unsafe { FreeSid(sid) };
	}
	if created == 0 {
		return Err(fail("cannot create the confined process"));
	}
	Ok(process)
}

/// `cmd` as one command line. `CreateProcessW` takes a string, not a vector,
/// so each argument is quoted the way the C runtime parses it back: backslashes
/// double only where they run into the closing quote.
fn command_line(cmd: &[String]) -> String {
	let mut out = String::new();
	for (index, argument) in cmd.iter().enumerate() {
		if index > 0 {
			out.push(' ');
		}
		if !argument.is_empty() && !argument.contains([' ', '\t', '"']) {
			out.push_str(argument);
			continue;
		}
		out.push('"');
		let mut backslashes = 0usize;
		for c in argument.chars() {
			match c {
				'\\' => {
					backslashes += 1;
					out.push(c);
				}
				'"' => {
					// The run before a quote is doubled, then the quote escaped.
					out.extend(std::iter::repeat_n('\\', backslashes + 1));
					backslashes = 0;
					out.push('"');
				}
				_ => {
					backslashes = 0;
					out.push(c);
				}
			}
		}
		// A run before the closing quote is doubled too, or it would escape it.
		out.extend(std::iter::repeat_n('\\', backslashes));
		out.push('"');
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_command_line_quotes_what_the_runtime_parses_back() {
		assert_eq!(command_line(&["a".into(), "b".into()]), "a b");
		assert_eq!(
			command_line(&["c:\\program files\\x.exe".into()]),
			"\"c:\\program files\\x.exe\""
		);
		// A trailing backslash inside quotes doubles, or it escapes the quote.
		assert_eq!(command_line(&["a b\\".into()]), "\"a b\\\\\"");
		// An embedded quote is escaped, and the run before it doubled.
		assert_eq!(command_line(&["a\"b".into()]), "\"a\\\"b\"");
		// An empty argument still has to occupy a place on the line.
		assert_eq!(command_line(&["a".into(), String::new()]), "a \"\"");
	}

	#[test]
	fn a_container_name_is_stable_and_fits_the_moniker_limit() {
		let name = container_name(Path::new("c:\\projects\\demo"));
		assert_eq!(name, container_name(Path::new("c:\\projects\\demo")));
		assert!(name.len() <= 64, "an AppContainer name is capped at 64");
		assert!(name.starts_with("cartridge-"));
	}

	#[test]
	fn a_write_grant_carries_read_and_never_execute() {
		let policy = Policy {
			read: vec![PathBuf::from("c:\\r")],
			write: vec![PathBuf::from("c:\\w")],
			exec: vec![PathBuf::from("c:\\x")],
			net: false,
			container: "cartridge-test".into(),
		};
		let grants = grants(&policy);
		let of = |p: &str| {
			grants
				.iter()
				.find(|(path, _)| path == &Path::new(p))
				.map(|(_, rights)| *rights)
				.expect("every policy path is granted")
		};
		assert_eq!(of("c:\\r") & FILE_GENERIC_EXECUTE, 0);
		assert_ne!(of("c:\\w") & FILE_GENERIC_READ, 0);
		assert_ne!(of("c:\\x") & FILE_GENERIC_EXECUTE, 0);
	}
}
