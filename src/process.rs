//! Bounded startup and owned subprocess resources. RPC calls have no deadline.
use std::ops::{Deref, DerefMut};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};

/// How long a starting cartridge has to announce itself, and how much a
/// discovery run may print. Both are settings — `host.startup_timeout_secs` and
/// `host.discovery_bytes` — because a cold cartridge on a loaded machine is
/// slow, not broken, and a large declaration is large, not hostile.
pub(crate) fn startup_timeout() -> Duration {
	crate::settings::host().startup_timeout()
}

fn discovery_bytes() -> usize {
	crate::settings::host().discovery_bytes
}

/// Dropping startup or its owner stops reader/writer tasks, kills the child,
/// and transfers wait ownership to a reaper on the same runtime.
pub(crate) struct Child {
	process: Option<tokio::process::Child>,
	pub tasks: tokio::task::JoinSet<()>,
}
impl Child {
	pub fn new(process: tokio::process::Child) -> Self {
		Self {
			process: Some(process),
			tasks: tokio::task::JoinSet::new(),
		}
	}
}
impl Deref for Child {
	type Target = tokio::process::Child;
	fn deref(&self) -> &Self::Target {
		self.process.as_ref().expect("owned child")
	}
}
impl DerefMut for Child {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.process.as_mut().expect("owned child")
	}
}
impl Drop for Child {
	fn drop(&mut self) {
		self.tasks.abort_all();
		if let Some(mut child) = self.process.take() {
			if child.try_wait().ok().flatten().is_none() {
				let _ = child.start_kill();
				if let Ok(runtime) = tokio::runtime::Handle::try_current() {
					runtime.spawn(async move {
						let _ = child.wait().await;
					});
				}
			}
		}
	}
}

async fn capture(read: impl AsyncRead + Unpin, limit: usize) -> Result<Vec<u8>, String> {
	let mut bytes = Vec::new();
	read.take(limit as u64 + 1)
		.read_to_end(&mut bytes)
		.await
		.map_err(|e| e.to_string())?;
	if bytes.len() > limit {
		return Err(format!("discovery output exceeds {limit} bytes"));
	}
	Ok(bytes)
}

pub(crate) async fn discover(
	cmd: &[String],
	timeout: Duration,
) -> Result<std::process::Output, String> {
	let program = cmd.first().ok_or("empty cmd")?;
	let mut child = Child::new(
		tokio::process::Command::new(program)
			.args(&cmd[1..])
			.arg("hello")
			.stdin(Stdio::null())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.kill_on_drop(true)
			.spawn()
			.map_err(|e| format!("{program}: {e}"))?,
	);
	let stdout = child.stdout.take().expect("piped stdout");
	let stderr = child.stderr.take().expect("piped stderr");
	let result = tokio::time::timeout(timeout, async {
		let (stdout, stderr, status) = tokio::try_join!(
			capture(stdout, discovery_bytes()),
			capture(stderr, discovery_bytes()),
			async { child.wait().await.map_err(|e| e.to_string()) },
		)?;
		Ok(std::process::Output {
			status,
			stdout,
			stderr,
		})
	})
	.await
	.unwrap_or_else(|_| Err(format!("{program} hello timed out")));
	if result.is_err() {
		let _ = child.kill().await;
	}
	result
}

/// Startup frames share one byte budget; even an unterminated frame is bounded.
pub(crate) async fn startup_line<R: tokio::io::AsyncBufRead + Unpin>(
	reader: &mut R,
	remaining: &mut usize,
) -> std::io::Result<Option<String>> {
	use tokio::io::AsyncBufReadExt;
	let mut line = Vec::new();
	loop {
		let bytes = reader.fill_buf().await?;
		if bytes.is_empty() {
			if line.is_empty() {
				return Ok(None);
			}
			break;
		}
		let count = bytes
			.iter()
			.position(|byte| *byte == b'\n')
			.map_or(bytes.len(), |index| index + 1);
		if count > *remaining {
			return Err(std::io::Error::new(
				std::io::ErrorKind::InvalidData,
				format!(
					"startup output exceeds host.startup_bytes ({} bytes)",
					crate::settings::host().startup_bytes
				),
			));
		}
		let ended = bytes[count - 1] == b'\n';
		line.extend_from_slice(&bytes[..count]);
		reader.consume(count);
		*remaining -= count;
		if ended {
			break;
		}
	}
	String::from_utf8(line)
		.map(Some)
		.map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

#[cfg(test)]
#[path = "../.cartridge/tests/unit/src/process/tests.rs"]
mod tests;
