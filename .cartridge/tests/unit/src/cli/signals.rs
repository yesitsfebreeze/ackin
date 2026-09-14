use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// An interrupt stops the host unless a program holds the terminal; a
/// terminate stops it regardless.
#[tokio::test(flavor = "multi_thread")]
async fn an_interrupt_stops_the_host_unless_a_program_holds_the_terminal() {
	// SAFETY: signals only this process, which the test owns.
	let signal = |sig: i32| unsafe { libc::kill(std::process::id() as libc::pid_t, sig) };
	let held = Arc::new(AtomicBool::new(true));
	let stop = tokio_util::sync::CancellationToken::new();
	super::stop_on_signals(stop.clone(), Some(held.clone())).unwrap();
	tokio::time::sleep(Duration::from_millis(100)).await;
	signal(libc::SIGINT);
	tokio::time::sleep(Duration::from_millis(200)).await;
	assert!(!stop.is_cancelled(), "an interrupt stopped a held run");
	held.store(false, Ordering::SeqCst);
	signal(libc::SIGTERM);
	tokio::select! {
		_ = stop.cancelled() => {}
		_ = tokio::time::sleep(Duration::from_secs(5)) => panic!("a terminate did not stop the host"),
	}
}
