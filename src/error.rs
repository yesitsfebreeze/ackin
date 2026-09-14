//! The one error type the host speaks.
//!
//! Every fallible host operation answers with [`Error`], so a caller can match
//! on *what* failed — a document that will not read, a peer that went away, a
//! service that is not provided — instead of parsing a sentence. The text a
//! variant renders is still the sentence a person reads, on the CLI and on the
//! wire: the wire carries error text, not variants, so an error that crosses a
//! process boundary comes back as [`Error::Remote`] on the other side.
//!
//! The SDK keeps its own `Result<T, String>`: a cartridge author's handler
//! answers with text that travels the wire, and `From<Error> for String` lets
//! `?` carry a host error into that answer unchanged.

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
	Runtime(#[from] crate::runtime::Error),
	#[error(transparent)]
	Watch(#[from] notify::Error),
	#[error(transparent)]
	Refusal(#[from] crate::resolver::Refusal),
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
	/// The peer a request was for is no longer there.
	#[error("{0}")]
	Gone(&'static str),
	#[error("`{0}` is not provided")]
	NotProvided(String),
	/// A service a foreground run needed never became active.
	#[error("service `{key}` unavailable: {why}")]
	Unavailable { key: String, why: String },
	/// A replacement the reload transaction refused.
	#[error("{0}")]
	Reload(String),
	/// A bridge call the profile's grant or the owner's state refuses.
	#[error("{0}")]
	Bridge(&'static str),
	/// A contract the caller broke: too many contributors, a budget of nothing.
	#[error("{0}")]
	Invalid(&'static str),
	/// A command-line argument that does not parse.
	#[error("invalid argument: {0}")]
	Argument(String),
	/// A wire frame neither end recognises.
	#[error("unknown message {0}")]
	UnknownMessage(serde_json::Value),
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

/// A host error as a handler's answer: the SDK speaks text, and `?` on a host
/// call inside a handler carries the error text unchanged.
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
