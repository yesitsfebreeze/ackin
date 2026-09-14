//! Linux implementation boundary. Refuse execution until both required kernel
//! policies can be installed; a partial policy must never become a fallback.

use std::path::Path;

use crate::loader::Grant;

pub(super) fn command(
	_cmd: &[String],
	_grant: &Grant,
	_root: &Path,
) -> std::io::Result<std::process::Command> {
	Err(std::io::Error::new(
		std::io::ErrorKind::Unsupported,
		"Linux Landlock and seccomp policy is not implemented: refusing cartridge execution",
	))
}
