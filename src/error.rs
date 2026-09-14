//! The host's error type. Errors cross the wire as text and come back as [`Error::Remote`].

use std::path::{Path, PathBuf};

/// What went wrong, by kind.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// A file the host had to read, write or watch, named.
	#[error("{path}: {source}")]
	File {
		path: PathBuf,
		#[source]
		source: std::io::Error,
	},
	#[error(transparent)]
	Io(#[from] std::io::Error),
	#[error(transparent)]
	Json(#[from] serde_json::Error),
	#[error(transparent)]
	Lua(#[from] mlua::Error),
	#[error(transparent)]
	Watch(#[from] notify::Error),
	/// A cartridge document that will not read, or declares what the format
	/// refuses. The path names the document, not the tree around it.
	#[error("{path}: {reason}")]
	Document { path: PathBuf, reason: String },
	/// A configuration value a declaration refuses.
	#[error("{0}")]
	Settings(String),
	/// The profile as composed: an entry that will not validate, or one the
	/// ledger cannot place.
	#[error("{0}")]
	Profile(String),
	/// A cartridge process that could not be started or did not come up.
	#[error("{program}: {reason}")]
	Process { program: String, reason: String },
	/// A cartridge that did not announce itself within its startup budget.
	#[error("{0} timed out before ready")]
	Timeout(String),
	/// The text a peer answered a request with.
	#[error("{0}")]
	Remote(String),
	#[error("`{0}` is not provided")]
	NotProvided(String),
	/// A service a foreground run needed never became active.
	#[error("service `{key}` unavailable: {why}")]
	Unavailable { key: String, why: String },
	/// A reload that could not be done.
	#[error("{0}")]
	Reload(String),
	/// A contract the caller broke: too many contributors, a budget of nothing.
	#[error("{0}")]
	Invalid(&'static str),
	/// Work the host was asked to stop before it finished: an interrupt, a
	/// terminate, or `cartridge stop`.
	#[error("stopped")]
	Stopped,
	/// A project file this machine was never told to trust, or that changed
	/// since. The refusal names what to read and what to run afterwards.
	#[error("{file}: {why}; review it, then run `cartridge trust {project}`")]
	Untrusted {
		project: PathBuf,
		file: PathBuf,
		why: &'static str,
	},
	/// A command-line argument that does not parse.
	#[error("invalid argument: {0}")]
	Argument(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
	/// An I/O failure at a named path.
	pub fn file(path: impl AsRef<Path>, source: std::io::Error) -> Self {
		Self::File {
			path: path.as_ref().to_path_buf(),
			source,
		}
	}

	/// A document that refuses, named.
	pub fn document(path: impl AsRef<Path>, reason: impl Into<String>) -> Self {
		Self::Document {
			path: path.as_ref().to_path_buf(),
			reason: reason.into(),
		}
	}

	/// A process that could not be run, named.
	pub fn process(program: impl Into<String>, reason: impl std::fmt::Display) -> Self {
		Self::Process {
			program: program.into(),
			reason: reason.to_string(),
		}
	}

	/// The variant a wire reply carries: the peer's text, as it sent it.
	pub fn remote(text: impl Into<String>) -> Self {
		Self::Remote(text.into())
	}

	/// Whether the sentence this error renders contains `needle`. What a test
	/// or a log matcher asks of an error it reads as text; a caller that wants
	/// the kind matches the variant instead.
	pub fn contains(&self, needle: &str) -> bool {
		self.to_string().contains(needle)
	}
}

/// A host error as a handler's text answer.
impl From<Error> for String {
	fn from(error: Error) -> Self {
		error.to_string()
	}
}

/// A host error raised inside Lua: the Lua surface reports it as an external
/// error whose text is the host's own.
impl From<Error> for mlua::Error {
	fn from(error: Error) -> Self {
		match error {
			Error::Lua(inner) => inner,
			other => mlua::Error::external(other),
		}
	}
}
