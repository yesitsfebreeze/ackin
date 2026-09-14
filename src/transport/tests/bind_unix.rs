use super::*;

#[tokio::test]
async fn a_live_owner_reports_already_running() {
	let dir = tempfile::tempdir().unwrap();
	let ep = Endpoint::Unix(dir.path().join("test.sock"));
	let first = bind(&ep).await.unwrap();
	let BindOutcome::Bound(_listener) = first else {
		panic!("first bind should own the socket")
	};
	let second = bind(&ep).await.unwrap();
	assert!(
		matches!(second, BindOutcome::AlreadyRunning),
		"a live owner -> AlreadyRunning"
	);
}

#[tokio::test]
async fn a_bound_socket_is_owner_only() {
	use std::os::unix::fs::PermissionsExt;
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("test.sock");
	let ep = Endpoint::Unix(path.clone());
	let BindOutcome::Bound(_listener) = bind(&ep).await.unwrap() else {
		panic!("first bind should own the socket")
	};
	let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
	assert_eq!(mode, 0o600, "socket must be owner-only, got {mode:o}");
}

#[tokio::test]
async fn a_socket_is_owner_only_even_under_a_permissive_umask() {
	use std::os::unix::fs::PermissionsExt;
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("test.sock");
	// Under the same lock as `bind_owner_only`, so the restore cannot interleave.
	// SAFETY: `umask` cannot fail and touches no memory.
	let previous = unsafe {
		let _guard = UMASK.lock();
		libc::umask(0)
	};
	let bound = bind(&Endpoint::Unix(path.clone())).await;
	// SAFETY: restoring the value `umask` returned above.
	unsafe {
		let _guard = UMASK.lock();
		libc::umask(previous);
	}
	let BindOutcome::Bound(_listener) = bound.unwrap() else {
		panic!("first bind should own the socket")
	};
	let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
	assert_eq!(mode, 0o600, "owner-only whatever the umask, got {mode:o}");
}

#[tokio::test]
async fn a_rebound_stale_socket_is_also_owner_only() {
	use std::os::unix::fs::PermissionsExt;
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("test.sock");
	{
		let _l = tokio::net::UnixListener::bind(&path).unwrap();
	}
	let ep = Endpoint::Unix(path.clone());
	let BindOutcome::Bound(_listener) = bind(&ep).await.unwrap() else {
		panic!("stale file should be removed and rebound")
	};
	let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
	assert_eq!(
		mode, 0o600,
		"the stale-rebind path hardens too, got {mode:o}"
	);
}

// A symlink we own pointing at a foreign target must refuse the bind.
#[tokio::test]
async fn a_symlink_to_a_foreign_target_refuses_the_bind() {
	let Some(foreign) = super::owner_tests_unix::foreign_path() else {
		return; // running as root: nothing here is foreign
	};
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("test.sock");
	std::os::unix::fs::symlink(&foreign, &path).unwrap();
	let ep = Endpoint::Unix(path.clone());
	let Err(err) = bind(&ep).await else {
		panic!("a foreign-owned endpoint must refuse the bind, not bind over it")
	};
	assert!(
		matches!(err, BindError::Untrusted(_)),
		"a squat is not i/o and is not AlreadyRunning: {err}"
	);
	assert!(
		err.to_string().contains("owned by uid 0"),
		"the refusal names the foreign uid: {err}"
	);
	assert!(
		path.symlink_metadata().is_ok(),
		"a path we refused is a path we must not have unlinked"
	);
}

// A live socket served by another uid (injected) must refuse the bind.
#[tokio::test]
async fn a_live_endpoint_served_by_another_uid_refuses_the_bind() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("test.sock");
	// Bound, listening and answering — the shape a squatter presents. Only the
	// uid the arm compares against is a fiction.
	let _squatter = tokio::net::UnixListener::bind(&path).unwrap();
	// SAFETY: `geteuid` cannot fail and touches no memory the caller owns.
	let euid = unsafe { libc::geteuid() };
	let Err(err) = bind_unix(&path, euid.wrapping_add(1)).await else {
		panic!(
			"a live endpoint served by a foreign uid must refuse the bind, not stand down for it"
		)
	};
	assert!(
		matches!(err, BindError::Untrusted(_)),
		"a squat is not i/o and is not AlreadyRunning: {err}"
	);
	assert!(
		err.to_string().contains(&format!("served by uid {euid}")),
		"the refusal names who is actually serving: {err}"
	);
	assert!(
		path.symlink_metadata().is_ok(),
		"a name we refused is a name we must not have unlinked"
	);
}

#[tokio::test]
async fn a_stale_socket_file_is_removed_and_rebound() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("test.sock");
	{
		let _l = tokio::net::UnixListener::bind(&path).unwrap();
	}
	assert!(
		path.exists(),
		"stale socket file remains after the listener drops"
	);
	let ep = Endpoint::Unix(path);
	let outcome = bind(&ep).await.unwrap();
	assert!(
		matches!(outcome, BindOutcome::Bound(_)),
		"stale file removed, endpoint rebound"
	);
}

#[tokio::test]
async fn a_regular_file_squatting_the_name_is_refused_not_removed() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("test.sock");
	std::fs::write(&path, b"not a socket").unwrap();
	let Err(err) = bind(&Endpoint::Unix(path.clone())).await else {
		panic!("a file that is not a socket must refuse the bind, not be deleted")
	};
	assert!(
		matches!(err, BindError::Untrusted(_)),
		"a squat is not i/o and is not AlreadyRunning: {err}"
	);
	assert!(
		err.to_string().contains("not a socket"),
		"the refusal says what it found: {err}"
	);
	assert_eq!(
		std::fs::read(&path).unwrap(),
		b"not a socket",
		"a path we refused is a path we must not have unlinked"
	);
}

#[tokio::test]
async fn a_caller_of_another_uid_is_refused_and_the_listener_keeps_serving() {
	use std::time::Duration;
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("test.sock");
	let BindOutcome::Bound(mut listener) = bind(&Endpoint::Unix(path.clone())).await.unwrap()
	else {
		panic!("first bind should own the socket")
	};
	// SAFETY: `geteuid` cannot fail and touches no memory the caller owns.
	let euid = unsafe { libc::geteuid() };
	// A knock the listener must refuse; only the uid it compares against is a fiction.
	let _stranger = UnixStreamAdapter::connect(&path).await.unwrap();
	let refused = tokio::time::timeout(
		Duration::from_millis(250),
		listener.accept_from(euid.wrapping_add(1)),
	)
	.await;
	assert!(
		refused.is_err(),
		"a connection from another uid must never become an adapter"
	);
	let _ours = UnixStreamAdapter::connect(&path).await.unwrap();
	let accepted = tokio::time::timeout(Duration::from_secs(2), listener.accept_from(euid)).await;
	assert!(
		accepted.is_ok_and(|r| r.is_ok()),
		"a refusal must not stop the listener"
	);
}
