//! A runtime is bound to the project it was started in. Two directories run two
//! runtimes, each owning its own cartridges, its own data and its own socket,
//! and each dying with the terminal that started it — so the way to move one is
//! to end it here and start it there. That only holds if the project is decided
//! by where the runtime was started, and if two projects never derive one socket.

use super::*;
use crate::{loader, socket};

/// A project is a directory whose `.cartridge` composes a runtime.
fn project(dir: &Path) -> PathBuf {
	std::fs::create_dir_all(dir.join(".cartridge")).unwrap();
	write(dir, ".cartridge/init.lua", "return {}");
	dir.to_path_buf()
}

/// `loader::root` and `socket::path` both read the working directory, which is
/// process-wide: these tests take one lock rather than racing each other.
fn guard() -> std::sync::MutexGuard<'static, ()> {
	static CWD: std::sync::Mutex<()> = std::sync::Mutex::new(());
	CWD.lock().unwrap_or_else(|e| e.into_inner())
}

/// Run `body` as if the process had been started in `dir`, restoring the
/// working directory afterwards however it ends.
fn started_in<T>(dir: &Path, body: impl FnOnce() -> T) -> T {
	let _lock = guard();
	let previous = std::env::current_dir().unwrap();
	std::env::set_current_dir(dir).unwrap();
	let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
	std::env::set_current_dir(previous).unwrap();
	result.unwrap()
}

#[test]
fn a_command_typed_inside_a_project_joins_that_project() {
	let dir = tempfile::tempdir().unwrap();
	let root = project(dir.path());
	let deep = root.join("src").join("nested");
	std::fs::create_dir_all(&deep).unwrap();

	// Started at the top and started three directories down are the same
	// runtime: a second one beside it would own a second copy of the data.
	let found = started_in(&deep, loader::root);
	assert_eq!(found.canonicalize().unwrap(), root.canonicalize().unwrap());
}

#[test]
fn a_cartridge_folder_that_composes_nothing_is_not_a_project() {
	// A repository keeps memos, records and data under `.cartridge` without
	// ever being a project. Stopping here would bind the runtime to a directory
	// with no composition to load — and, worse, shadow the real project above it.
	let dir = tempfile::tempdir().unwrap();
	let root = project(dir.path());
	let records = root.join("notes");
	std::fs::create_dir_all(records.join(".cartridge").join("memos")).unwrap();

	let found = started_in(&records, loader::root);
	assert_eq!(found.canonicalize().unwrap(), root.canonicalize().unwrap());
}

#[test]
fn a_directory_under_no_project_is_its_own_root() {
	// The runtime still starts; its relative paths land where it was started,
	// which is the only place they could mean.
	let dir = tempfile::tempdir().unwrap();
	let found = started_in(dir.path(), loader::root);
	assert_eq!(
		found.canonicalize().unwrap(),
		dir.path().canonicalize().unwrap()
	);
}

#[test]
fn two_projects_running_one_profile_do_not_share_a_socket() {
	// The regression: the socket was keyed by the profile alone, so the second
	// project found the first one's socket and quietly reused it instead of
	// serving itself. Every project names its profile `.cartridge`.
	let first = tempfile::tempdir().unwrap();
	let second = tempfile::tempdir().unwrap();
	project(first.path());
	project(second.path());

	let profile = loader::profile();
	let one = started_in(first.path(), || socket::path(&profile));
	let two = started_in(second.path(), || socket::path(&profile));
	assert_ne!(one, two);

	// And the same project derives the same socket from anywhere inside it, or
	// a command typed in a subdirectory could not reach its own runtime.
	let deep = first.path().join("src");
	std::fs::create_dir_all(&deep).unwrap();
	let again = started_in(&deep, || {
		let root = loader::root();
		std::env::set_current_dir(root).unwrap();
		socket::path(&profile)
	});
	assert_eq!(one, again);
}
