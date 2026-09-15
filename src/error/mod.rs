use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
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
	/// The path names the document, not the tree around it.
	#[error("{path}: {reason}")]
	Document { path: PathBuf, reason: String },
	#[error("{0}")]
	Settings(String),
	#[error("{0}")]
	Descriptor(String),
	#[error("{program}: {reason}")]
	Process { program: String, reason: String },
	#[error("{0} timed out before ready")]
	Timeout(String),
	#[error("{0}")]
	Remote(String),
	#[error("`{0}` is not provided")]
	NotProvided(String),
	#[error("service `{key}` unavailable: {why}")]
	Unavailable { key: String, why: String },
	#[error("{0}")]
	Reload(String),
	#[error("{0}")]
	Invalid(&'static str),
	#[error("stopped")]
	Stopped,
	#[error("{file}: {why}; review it, then run `cartridge trust {project}`")]
	Untrusted {
		project: PathBuf,
		file: PathBuf,
		why: &'static str,
	},
	#[error("invalid argument: {0}")]
	Argument(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
	pub fn file(path: impl AsRef<Path>, source: std::io::Error) -> Self {
		Self::File {
			path: path.as_ref().to_path_buf(),
			source,
		}
	}

	pub fn document(path: impl AsRef<Path>, reason: impl Into<String>) -> Self {
		Self::Document {
			path: path.as_ref().to_path_buf(),
			reason: reason.into(),
		}
	}

	pub fn process(program: impl Into<String>, reason: impl std::fmt::Display) -> Self {
		Self::Process {
			program: program.into(),
			reason: reason.to_string(),
		}
	}

	pub fn remote(text: impl Into<String>) -> Self {
		Self::Remote(text.into())
	}

	/// A test or a log matcher asks of an error it reads as text; a caller
	/// that wants the kind matches the variant instead.
	pub fn contains(&self, needle: &str) -> bool {
		self.to_string().contains(needle)
	}
}

impl From<Error> for String {
	fn from(error: Error) -> Self {
		error.to_string()
	}
}

impl From<Error> for mlua::Error {
	fn from(error: Error) -> Self {
		match error {
			Error::Lua(inner) => inner,
			other => mlua::Error::external(other),
		}
	}
}
