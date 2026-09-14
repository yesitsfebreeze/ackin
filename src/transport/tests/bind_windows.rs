use super::*;

#[tokio::test]
async fn a_second_bind_of_the_same_pipe_reports_already_running() {
	let ep = Endpoint::NamedPipe(format!(
		r"\\.\pipe\transport-bindtest-{}",
		std::process::id()
	));
	let first = bind(&ep).await.unwrap();
	assert!(
		matches!(first, BindOutcome::Bound(_)),
		"first bind owns the pipe"
	);
	let second = bind(&ep).await.unwrap();
	assert!(
		matches!(second, BindOutcome::AlreadyRunning),
		"second bind sees AlreadyRunning"
	);
	drop(first); // keep the first instance alive until the assertion above
}
