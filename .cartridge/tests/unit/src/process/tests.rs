use super::*;
use std::os::unix::fs::PermissionsExt;

async fn refuses(body: &str, expected: &str) {
	let dir = tempfile::tempdir().unwrap();
	let script = dir.path().join("hello.sh");
	let pid = dir.path().join("pid");
	std::fs::write(
		&script,
		format!("#!/bin/sh\necho $$ > '{}'\n{body}\n", pid.display()),
	)
	.unwrap();
	std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
	let error = discover(&[script.display().to_string()], Duration::from_secs(3))
		.await
		.unwrap_err();
	assert!(error.contains(expected), "{error}");
	let pid = std::fs::read_to_string(pid).unwrap();
	let alive = std::process::Command::new("/bin/kill")
		.args(["-0", pid.trim()])
		.stderr(Stdio::null())
		.status()
		.unwrap();
	assert!(!alive.success(), "failed discovery child still alive");
}

#[tokio::test]
async fn hung_discovery_is_killed_and_reaped() {
	refuses("while :; do :; done", "timed out").await;
}
#[tokio::test]
async fn excessive_discovery_output_is_killed_and_reaped() {
	refuses(
		"while :; do echo 1234567890123456789012345678901234567890; done",
		"exceeds",
	)
	.await;
}
#[tokio::test]
async fn excessive_discovery_stderr_is_also_bounded() {
	refuses(
		"while :; do echo 1234567890123456789012345678901234567890 >&2; done",
		"exceeds",
	)
	.await;
}
#[tokio::test]
async fn an_unterminated_startup_frame_cannot_exceed_its_budget() {
	let bytes = vec![b'x'; 70 * 1024];
	let mut reader = tokio::io::BufReader::new(bytes.as_slice());
	let mut remaining = 64 * 1024;
	let error = startup_line(&mut reader, &mut remaining).await.unwrap_err();
	assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
	assert!(error.to_string().contains("exceeds"));
}
