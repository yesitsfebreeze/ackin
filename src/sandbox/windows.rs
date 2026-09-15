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
#[cfg(test)]
use windows_sys::Win32::Storage::FileSystem::{FILE_EXECUTE, FILE_WRITE_DATA};
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

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct Policy {
	read: Vec<PathBuf>,
	write: Vec<PathBuf>,
	exec: Vec<PathBuf>,
	net: bool,
	container: String,
}

const NET_CAPABILITIES: [&str; 2] = ["internetClient", "privateNetworkClientServer"];

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
	let mut exec: Vec<PathBuf> = vec![binary.clone()];
	exec.extend(super::interpreters(&binary));
	for program in &grant.exec {
		if program != "*" {
			if let Some(path) = super::granted_exec(program, &root) {
				exec.extend(super::with_variants(path));
			}
		}
	}
	let homes = super::homes();
	let mut installations: Vec<PathBuf> = exec
		.iter()
		.filter_map(|p| super::installation_outside(p, &homes))
		.collect();
	installations.sort();
	installations.dedup();
	exec.extend(installations.iter().cloned());
	exec.sort();
	exec.dedup();
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

fn container_name(root: &Path) -> String {
	format!("cartridge-{}", crate::transport::typed::path_tag(root))
}

pub(crate) fn container_sid_for(root: &Path) -> crate::Result<String> {
	use windows_sys::Win32::Foundation::{LocalFree, HLOCAL};
	use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
	let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
	let sid = container_sid(&container_name(&root))?;
	let mut text: *mut u16 = std::ptr::null_mut();
	let ok = unsafe { ConvertSidToStringSidW(sid.0, &mut text) };
	if ok == 0 || text.is_null() {
		return Err(crate::Error::process(
			"appcontainer",
			std::io::Error::last_os_error(),
		));
	}
	let out = unsafe {
		let mut len = 0;
		while *text.add(len) != 0 {
			len += 1;
		}
		let out = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
		LocalFree(text as HLOCAL);
		out
	};
	Ok(out)
}

struct Sid(PSID);

impl Drop for Sid {
	fn drop(&mut self) {
		if !self.0.is_null() {
			unsafe { FreeSid(self.0) };
		}
	}
}

fn container_sid(name: &str) -> crate::Result<Sid> {
	let name = wide(name);
	let mut psid: PSID = std::ptr::null_mut();
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
	let derived = unsafe { DeriveAppContainerSidFromAppContainerName(name.as_ptr(), &mut psid) };
	if derived < 0 {
		return Err(crate::Error::process(
			"appcontainer",
			format!("cannot create or derive the container profile: 0x{derived:08x}"),
		));
	}
	Ok(Sid(psid))
}

fn capability_sids(name: &str) -> Vec<PSID> {
	let name = wide(name);
	let mut group_sids: *mut PSID = std::ptr::null_mut();
	let mut group_count: u32 = 0;
	let mut cap_sids: *mut PSID = std::ptr::null_mut();
	let mut cap_count: u32 = 0;
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
			out.push(unsafe { *cap_sids.offset(index) });
		}
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

fn ace(path: &Path, sid: PSID, rights: u32, grant: bool) -> crate::Result<()> {
	let name = wide(path);
	let mut dacl: *mut ACL = std::ptr::null_mut();
	let mut descriptor = std::ptr::null_mut();
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
	let built = unsafe { SetEntriesInAclW(1, &entry, dacl, &mut merged) };
	if built != 0 {
		unsafe { LocalFree(descriptor as HLOCAL) };
		return Err(crate::Error::file(
			path,
			std::io::Error::from_raw_os_error(built as i32),
		));
	}
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

pub(super) fn confine(policy: &str, cmd: &[String]) -> crate::Result<std::convert::Infallible> {
	let policy: Policy = serde_json::from_str(policy)?;
	let sid = container_sid(&policy.container)?;
	let wanted = grants(&policy);
	let mut granted: Vec<(&Path, u32)> = Vec::new();
	let started = (|| -> crate::Result<PROCESS_INFORMATION> {
		for (path, rights) in &wanted {
			ace(path, sid.0, *rights, true)?;
			granted.push((path, *rights));
		}
		spawn(&policy, sid.0, cmd)
	})();
	let process = match started {
		Ok(process) => process,
		Err(error) => {
			revoke(&granted, sid.0);
			return Err(error);
		}
	};
	unsafe { CloseHandle(process.hThread) };
	let waited = unsafe { WaitForSingleObject(process.hProcess, INFINITE) };
	let mut code: u32 = 1;
	if waited != WAIT_FAILED {
		unsafe { GetExitCodeProcess(process.hProcess, &mut code) };
	}
	unsafe { CloseHandle(process.hProcess) };
	revoke(&granted, sid.0);
	std::process::exit(code as i32)
}

fn revoke(granted: &[(&Path, u32)], sid: PSID) {
	for (path, rights) in granted {
		if let Err(error) = ace(path, sid, *rights, false) {
			eprintln!("cartridge: {} stayed granted: {error}", path.display());
		}
	}
}

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
	let mut size: usize = 0;
	unsafe { InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut size) };
	if size == 0 {
		return Err(fail("cannot size the attribute list"));
	}
	let mut buffer = vec![0u8; size];
	let list = buffer.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;
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
	unsafe { DeleteProcThreadAttributeList(list) };
	for sid in owned {
		unsafe { FreeSid(sid) };
	}
	if created == 0 {
		return Err(fail("cannot create the confined process"));
	}
	Ok(process)
}

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
		assert_eq!(command_line(&["a b\\".into()]), "\"a b\\\\\"");
		assert_eq!(command_line(&["a\"b".into()]), "\"a\\\"b\"");
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
		assert_eq!(
			of("c:\\r") & FILE_EXECUTE,
			0,
			"a read grant is not an exec grant"
		);
		assert_eq!(
			of("c:\\r") & FILE_WRITE_DATA,
			0,
			"a read grant is not a write grant"
		);
		assert_ne!(of("c:\\w") & FILE_WRITE_DATA, 0, "a write grant writes");
		assert_ne!(of("c:\\w") & FILE_GENERIC_READ, 0, "a write is also a read");
		assert_eq!(
			of("c:\\w") & FILE_EXECUTE,
			0,
			"a write grant is not an exec grant"
		);
		assert_ne!(of("c:\\x") & FILE_EXECUTE, 0, "an exec grant executes");
	}
}
