use super::*;

// A path owned by another uid; None when running as root.
pub(super) fn foreign_path() -> Option<PathBuf> {
	// SAFETY: `geteuid` cannot fail and touches no memory the caller owns.
	if unsafe { libc::geteuid() } == 0 {
		return None;
	}
	let p = PathBuf::from("/etc/hosts");
	p.exists().then_some(p)
}

#[test]
fn a_path_this_user_owns_is_accepted() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("test.sock");
	std::fs::write(&path, b"").unwrap();
	assert!(
		require_owned_by_caller(&path).is_ok(),
		"our own path must pass, or nothing connects"
	);
}

#[test]
fn a_path_owned_by_another_uid_is_refused() {
	let Some(path) = foreign_path() else {
		return; // running as root: nothing here is foreign
	};
	let err = require_owned_by_caller(&path).expect_err("a foreign owner must refuse");
	assert!(
		matches!(err, AdapterError::UntrustedEndpoint(_)),
		"refusal must be untrust, not i/o: {err}"
	);
	assert!(
		err.to_string().contains("owned by uid"),
		"the refusal names the owner: {err}"
	);
}

#[test]
fn a_missing_path_reads_as_absence_not_as_a_squat() {
	let dir = tempfile::tempdir().unwrap();
	let err = require_owned_by_caller(&dir.path().join("nothing.sock"))
		.expect_err("nothing to connect to is still an error");
	assert!(
		matches!(err, AdapterError::Io(_)),
		"an empty path is the no-daemon case, not a squat: {err}"
	);
}

#[test]
fn a_dangling_symlink_is_refused() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("test.sock");
	std::os::unix::fs::symlink(dir.path().join("gone"), &path).unwrap();
	let err = require_owned_by_caller(&path).expect_err("a link to nothing must refuse");
	assert!(
		matches!(err, AdapterError::UntrustedEndpoint(_)),
		"a dangling link is a substitution, not an absence: {err}"
	);
}

#[test]
fn a_symlink_to_a_foreign_target_is_refused() {
	let Some(foreign) = foreign_path() else {
		return; // running as root: nothing here is foreign
	};
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("test.sock");
	std::os::unix::fs::symlink(&foreign, &path).unwrap();
	let err = require_owned_by_caller(&path).expect_err("the target's owner is what counts");
	assert!(
		matches!(err, AdapterError::UntrustedEndpoint(_)),
		"a link we own to a socket we do not is still a squat: {err}"
	);
	assert!(
		err.to_string()
			.contains("resolves to a path owned by uid 0"),
		"the refusal names the target's owner, not the link's: {err}"
	);
}

#[tokio::test]
async fn the_peer_check_reads_the_server_uid_and_decides_both_ways() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("test.sock");
	let _listener = tokio::net::UnixListener::bind(&path).unwrap();
	let adapter = UnixStreamAdapter::connect(&path)
		.await
		.expect("our own listener accepts");

	// SAFETY: `geteuid` cannot fail and touches no memory the caller owns.
	let euid = unsafe { libc::geteuid() };
	assert!(
		require_peer_uid(&adapter, &path, euid).is_ok(),
		"our own daemon must pass, or nothing connects"
	);

	let err = require_peer_uid(&adapter, &path, euid.wrapping_add(1))
		.expect_err("a server that is not who we expect must be refused");
	assert!(
		matches!(err, AdapterError::UntrustedEndpoint(_)),
		"the peer verdict is untrust, not i/o: {err}"
	);
	assert!(
		err.to_string().contains(&format!("served by uid {euid}")),
		"the refusal names who is actually serving: {err}"
	);
}

// The no-regression half in one test: a socket we bound, reached through the
// real entry point, with both checks in the way.
#[tokio::test]
async fn connect_accepts_a_socket_this_user_bound() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("test.sock");
	let _listener = tokio::net::UnixListener::bind(&path).unwrap();
	assert!(
		connect(&Endpoint::Unix(path)).await.is_ok(),
		"a socket this user bound is accepted"
	);
}

#[tokio::test]
async fn connect_refuses_a_foreign_endpoint_before_it_connects() {
	let Some(path) = foreign_path() else {
		return; // running as root: nothing here is foreign
	};
	let err = connect(&Endpoint::Unix(path))
		.await
		.err()
		.expect("a foreign endpoint never becomes an adapter");
	assert!(
		matches!(err, AdapterError::UntrustedEndpoint(_)),
		"refused by the owner check, not by the connect: {err}"
	);
}

#[tokio::test]
async fn connect_refuses_when_the_peer_uid_differs() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("test.sock");
	let _listener = tokio::net::UnixListener::bind(&path).unwrap();
	// SAFETY: `geteuid` cannot fail and touches no memory the caller owns.
	let euid = unsafe { libc::geteuid() };
	let err = connect_with_peer(&Endpoint::Unix(path), euid.wrapping_add(1))
		.await
		.err()
		.expect("a peer uid that is not the server's is refused");
	assert!(
		matches!(err, AdapterError::UntrustedEndpoint(_)),
		"the peer verdict is untrust, not i/o: {err}"
	);
	assert!(
		err.to_string().contains(&format!("served by uid {euid}")),
		"the refusal names who is actually serving: {err}"
	);
}
