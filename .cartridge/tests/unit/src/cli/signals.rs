use std::time::Duration;

fn signal(sig: i32) {
	unsafe { libc::kill(std::process::id() as libc::pid_t, sig) };
}

/// One test per process (see the justfile): a signal handler registered here
/// stays registered for whatever runs after it.
#[tokio::test(flavor = "multi_thread")]
async fn a_terminate_stops_the_host() {
	let stop = tokio_util::sync::CancellationToken::new();
	super::stop_on_signals(stop.clone()).unwrap();
	tokio::time::sleep(Duration::from_millis(100)).await;
	signal(libc::SIGTERM);
	tokio::select! {
		_ = stop.cancelled() => {}
		_ = tokio::time::sleep(Duration::from_secs(5)) => panic!("a terminate did not stop the host"),
	}
}

/// `launch` holds the terminal for its agent, which takes the interrupt
/// itself; without this the default action would end this process here.
#[tokio::test(flavor = "multi_thread")]
async fn an_interrupt_is_ignored_while_a_program_holds_the_terminal() {
	super::ignore_interrupt().unwrap();
	tokio::time::sleep(Duration::from_millis(100)).await;
	signal(libc::SIGINT);
	tokio::time::sleep(Duration::from_millis(200)).await;
}
