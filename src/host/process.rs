use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::oneshot;

use crate::error::{Error, Result};
use crate::transport::cartridge::{Directory, CONNECT_TIMEOUT_ENV, NODE_TOKEN_ENV, SOCKET_ENV};

use super::{Host, Plan, Running};

pub const NODE_BIN_ENV: &str = "CARTRIDGE_NODE_BIN";
/// The fd of the listener the host bound for this node (its manifest's
/// `listener`), inherited across the spawn.
pub const LISTENER_FD_ENV: &str = "CARTRIDGE_LISTENER_FD";

/// Trim with care: without `HOME`/`CARTRIDGE_HOME` a node reads another
/// machine's trust store and global `config.lua`.
#[cfg(unix)]
const PASSTHROUGH: &[&str] = &[
	"PATH",
	"HOME",
	"CARTRIDGE_HOME",
	"TMPDIR",
	"TZ",
	"LANG",
	"LC_ALL",
	"USER",
	"LOGNAME",
	"SHELL",
	"XDG_RUNTIME_DIR",
	"CARTRIDGE_PROXY_KEY",
];

/// A child created without `SystemRoot`/`SystemDrive` fails in
/// `CreateProcessW` itself, before anything it runs. Trim with care.
#[cfg(windows)]
const PASSTHROUGH: &[&str] = &[
	"PATH",
	"PATHEXT",
	"SystemRoot",
	"SystemDrive",
	"windir",
	"ComSpec",
	"LOCALAPPDATA",
	"APPDATA",
	"USERPROFILE",
	"TEMP",
	"TMP",
	"TZ",
	"LANG",
	"USERNAME",
	"NUMBER_OF_PROCESSORS",
	"PROCESSOR_ARCHITECTURE",
];

fn granted_env(grant: &[String]) -> Vec<(String, std::ffi::OsString)> {
	let allows = |name: &str| {
		grant.iter().any(|pattern| match pattern.strip_suffix('*') {
			Some("") => false,
			Some(prefix) => name.starts_with(prefix),
			None => name == pattern,
		})
	};
	std::env::vars_os()
		.filter_map(|(key, value)| {
			let name = key.to_str()?.to_owned();
			allows(&name).then_some((name, value))
		})
		.collect()
}

pub(super) async fn start(
	host: &Arc<Host>,
	plan: &Plan,
	directory: &Directory,
	generation: u64,
) -> Result<Running> {
	let settings = crate::settings::host();
	let token = crate::transport::token();
	host.node_tokens
		.lock()
		.insert(plan.id.clone(), token.clone());
	let socket = host.socket(&plan.id);
	let _ = std::fs::remove_file(&socket);
	let sockets = Some(host.sockets.clone());
	// Settled here, not in the node: the sandbox hides the person's config.lua
	// files from the node, so a re-settle there would read declared defaults
	// over the person's actual choices.
	let lua_memory = settings.lua_memory_bytes.to_string();
	let lua_budget = settings.lua_instruction_budget.to_string();
	#[cfg(windows)]
	let handed = {
		let sid = crate::sandbox::container_sid_for(&plan.root)
			.map_err(|e| Error::process(&plan.id, e))?;
		let endpoint = crate::transport::typed::Endpoint::local(&socket);
		crate::transport::typed::broker(&endpoint, &sid)
			.map_err(|e| Error::process(&plan.id, e))?
			.into_iter()
			.map(|handle| handle.to_string())
			.collect::<Vec<_>>()
			.join(",")
	};
	let exe = match std::env::var_os(NODE_BIN_ENV) {
		Some(exe) => std::path::PathBuf::from(exe),
		None => std::env::current_exe().map_err(|e| Error::file("cartridge", e))?,
	};
	let command = vec![exe.to_string_lossy().into_owned(), "node".to_owned()];
	let mut spawner = tokio::process::Command::from(
		crate::sandbox::command(&command, &plan.grant, &plan.root, sockets.as_deref())
			.map_err(|e| Error::process(&plan.id, e))?,
	);
	spawner
		.env_clear()
		.envs(
			PASSTHROUGH
				.iter()
				.filter_map(|key| std::env::var_os(key).map(|value| (key, value))),
		)
		.envs(granted_env(&plan.grant.env))
		.env(SOCKET_ENV, &socket)
		.env(NODE_TOKEN_ENV, &token)
		.env(
			CONNECT_TIMEOUT_ENV,
			settings.startup_timeout_secs.to_string(),
		)
		.env(crate::node::LUA_MEMORY_ENV, &lua_memory)
		.env(crate::node::LUA_BUDGET_ENV, &lua_budget)
		.env(crate::node::ENTRY_ENV, &plan.entry)
		.env(crate::node::ENTRY_SHA256_ENV, &plan.entry_sha256)
		.env(crate::node::ROOT_ENV, &plan.root)
		.env("CARTRIDGE_BIN", &exe)
		.env(
			crate::node::LISTEN_ENV,
			serde_json::to_string(&plan.listen)?,
		)
		.stdin(Stdio::piped())
		.stdout(Stdio::null())
		.stderr(Stdio::piped())
		.kill_on_drop(true);
	#[cfg(windows)]
	spawner.env(crate::transport::typed::PIPE_HANDLES_ENV, &handed);
	#[cfg(unix)]
	spawner.process_group(0);
	#[cfg(unix)]
	if let Some(address) = &plan.listener {
		let fd = host.listener_for(&plan.id, address)?;
		spawner.env(LISTENER_FD_ENV, fd.to_string());
		// SAFETY: runs in the child between fork and exec, and only clears
		// close-on-exec on one fd this process owns; nothing allocates.
		unsafe {
			spawner.pre_exec(move || {
				if libc::fcntl(fd, libc::F_SETFD, 0) == -1 {
					return Err(std::io::Error::last_os_error());
				}
				Ok(())
			});
		}
	}
	let mut child = spawner.spawn().map_err(|e| Error::process(&plan.id, e))?;
	let group = Group::of(&child).map_err(|e| Error::process(&plan.id, e))?;
	let stdin = child.stdin.take();
	let tail = Arc::new(std::sync::Mutex::new(
		std::collections::VecDeque::<String>::new(),
	));
	if let Some(stderr) = child.stderr.take() {
		let (id, tail) = (plan.id.clone(), tail.clone());
		let recorder = host.trace_recorder().clone();
		tokio::spawn(async move {
			let mut lines = BufReader::new(stderr).lines();
			while let Ok(Some(line)) = lines.next_line().await {
				crate::trace::diagnostic_line(&id, &line);
				if !line.starts_with("cartridge trace delivery:") {
					let activity = json!({"kind":"diagnostic", "origin":id, "diagnostic":
							serde_json::from_str::<serde_json::Value>(&line).unwrap_or_else(|_| json!(line))});
					recorder.record(activity);
				}
				let mut tail = tail.lock().expect("stderr tail");
				tail.push_back(line);
				if tail.len() > 20 {
					tail.pop_front();
				}
			}
		});
	}
	let said = move || {
		let tail = tail.lock().expect("stderr tail");
		match tail.is_empty() {
			true => String::new(),
			false => format!(": {}", tail.iter().cloned().collect::<Vec<_>>().join(" | ")),
		}
	};
	let deadline = tokio::time::Instant::now() + settings.startup_timeout();
	let (peer, _incoming) = loop {
		if let Some(status) = child.try_wait()? {
			tokio::time::sleep(Duration::from_millis(50)).await;
			return Err(Error::process(
				&plan.id,
				format!("exited before serving: {status}{}", said()),
			));
		}
		if let Ok(connected) = super::connect(&socket, &token).await {
			break connected;
		}
		if tokio::time::Instant::now() >= deadline {
			let _ = child.kill().await;
			return Err(Error::Timeout(plan.id.clone()));
		}
		tokio::time::sleep(Duration::from_millis(20)).await;
	};
	let apply = peer.call(
		"apply",
		json!({ "name": plan.name, "config": plan.config, "directory": directory }),
	);
	match tokio::time::timeout(settings.startup_timeout(), apply).await {
		Ok(Ok(_)) => {}
		Ok(Err(error)) => {
			let _ = child.kill().await;
			return Err(Error::process(
				&plan.id,
				format!("{}{}", error.message, said()),
			));
		}
		Err(_) => {
			let _ = child.kill().await;
			return Err(Error::Timeout(plan.id.clone()));
		}
	}
	let (stop_tx, stop_rx) = oneshot::channel::<()>();
	let (id, weak) = (plan.id.clone(), Arc::downgrade(host));
	let monitor = tokio::spawn(async move {
		let _group = group;
		let exited = tokio::select! {
			status = child.wait() => Some(status),
			_ = stop_rx => None,
		};
		match exited {
			Some(status) => {
				if let Some(host) = weak.upgrade() {
					let why = match status {
						Ok(status) => format!("exited: {status}"),
						Err(error) => format!("lost: {error}"),
					};
					host.exited(&id, generation, why);
				}
			}
			None => {
				drop(stdin);
				let shutdown = crate::settings::host().shutdown_timeout();
				if tokio::time::timeout(shutdown, child.wait()).await.is_err() {
					let _ = child.kill().await;
				}
			}
		}
		let _ = std::fs::remove_file(&socket);
	});
	Ok(Running {
		peer,
		stop: Box::new(move |peer| {
			Box::pin(async move {
				let shutdown = crate::settings::host().shutdown_timeout();
				let _ = tokio::time::timeout(shutdown, peer.call("dispose", json!({}))).await;
				let _ = stop_tx.send(());
				let _ = monitor.await;
			})
		}),
	})
}

#[cfg(unix)]
struct Group(libc::pid_t);

#[cfg(unix)]
impl Group {
	fn of(child: &tokio::process::Child) -> std::io::Result<Self> {
		let pid = child
			.id()
			.ok_or_else(|| std::io::Error::other("exited as it was spawned"))?;
		Ok(Group(pid as libc::pid_t))
	}
}

#[cfg(unix)]
impl Drop for Group {
	fn drop(&mut self) {
		// SAFETY: killpg only sends a signal. The id names no other group while a
		// member lives, and the guard drops as soon as its leader is reaped.
		unsafe { libc::killpg(self.0, libc::SIGKILL) };
	}
}

/// Assigned after the child is spawned, not created into the job: `tokio`
/// offers no `CREATE_SUSPENDED`. A grandchild started in that window would
/// escape, but the node's first act is to connect to its socket, before any
/// cartridge code runs.
#[cfg(windows)]
struct Group(windows_sys::Win32::Foundation::HANDLE);

// SAFETY: a job object handle is a kernel handle, valid in any thread of this
// process. The guard is moved into the task that waits on the node.
#[cfg(windows)]
unsafe impl Send for Group {}

#[cfg(windows)]
impl Group {
	fn of(child: &tokio::process::Child) -> std::io::Result<Self> {
		use windows_sys::Win32::System::JobObjects::{
			AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
			SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
			JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
		};
		let handle = child
			.raw_handle()
			.ok_or_else(|| std::io::Error::other("exited as it was spawned"))?;
		let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
		if job.is_null() {
			return Err(std::io::Error::last_os_error());
		}
		let group = Group(job);
		let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
		limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
		let set = unsafe {
			SetInformationJobObject(
				job,
				JobObjectExtendedLimitInformation,
				&mut limits as *mut _ as *mut core::ffi::c_void,
				std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
			)
		};
		if set == 0 {
			return Err(std::io::Error::last_os_error());
		}
		if unsafe { AssignProcessToJobObject(job, handle as _) } == 0 {
			return Err(std::io::Error::last_os_error());
		}
		Ok(group)
	}
}

#[cfg(windows)]
impl Drop for Group {
	fn drop(&mut self) {
		// SAFETY: the handle came from `CreateJobObjectW` and nothing else holds
		// it. Closing the last handle to a kill-on-close job kills its members.
		unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
	}
}
