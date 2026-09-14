//! `cartridge.json` and the profile.
//!
//! ```json
//! {
//!   "name": "store",            // required
//!   "entry": "init.lua",        // required: Lua entry, relative to this folder
//!   "description": "Stores data",
//!   "commands": { "test": {"argv": ["just", "test", "store"], "cwd": "."} },
//!   "binary": "store-bin",      // executable basename, when it differs from the folder
//!   "ui": "ui/index.ts",
//!   "contracts": ["store.check"], // events it listens to that prove it
//!   "setup": "store.setup",     // an event it listens to, sent by `cartridge setup`
//!   "doctor": "store.doctor",   // an event it listens to, sent by `cartridge doctor`
//!   "source": "https://…",
//!   "settings": { "max_bytes": {"type": "integer", "default": 67108864, "min": 1024, "doc": "Read budget."} },
//!   "events": { "store.changed": {"description": "…", "schema": {"type": "object"}} },
//!   "needs": ["log.write"],     // events that must have a listener
//!   "listen": ["store.flush"],  // events this cartridge listens to
//!   "grant": { "read": ["data"], "write": ["cache"], "net": ["api.host"], "exec": ["rg"] }
//! }
//! ```
//!
//! Every declared setting holds a value by the time the cartridge starts. An
//! absent `grant` grants nothing. The document is the whole declaration; the
//! Lua entry registers what it declared.

mod document;
pub(crate) mod entries;

use std::path::{Path, PathBuf};

use document::classify;
pub(crate) use document::{document, resolve, Declared};
pub use document::{Cartridge, Command, Event, Grant, MANIFEST};

/// A profile entry: a cartridge folder, or a bare Lua file.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
	pub id: String,
	pub path: String,
	#[serde(default)]
	pub config: serde_json::Value,
	#[serde(default)]
	pub disabled: bool,
}

impl Entry {
	pub fn file(&self, dir: &Path) -> PathBuf {
		normalize(&classify(&dir.join(&self.path)))
	}
}

/// One profile entry as its files describe it.
pub struct CartridgeInfo {
	pub entry: Entry,
	pub needs: Vec<String>,
	pub events: Vec<String>,
	pub listen: Vec<String>,
	/// `None` when the document could not be read.
	pub grant: Option<Grant>,
	/// Why the document would not read.
	pub unread: Option<String>,
	/// Why the entry would not load.
	pub error: Option<String>,
}

/// One entry's configurable surface.
pub struct SettingsInfo {
	pub id: String,
	pub path: String,
	pub disabled: bool,
	pub specs: crate::settings::Specs,
	pub settled: serde_json::Value,
	/// Configured keys no declaration mentions.
	pub undeclared: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct Source {
	pub(crate) path: PathBuf,
	pub(crate) stamp: Option<(u64, std::time::SystemTime)>,
}

impl Source {
	pub(crate) fn new(path: PathBuf) -> Self {
		let stamp = std::fs::metadata(&path)
			.ok()
			.and_then(|m| Some((m.len(), m.modified().ok()?)));
		Self { path, stamp }
	}

	pub(crate) fn changed(&self) -> bool {
		self.stamp != Self::new(self.path.clone()).stamp
	}
}

pub(crate) fn normalize(path: &Path) -> PathBuf {
	path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// The profile directory: `.cartridge`, holding `init.lua` and `config.lua`.
pub fn profile() -> PathBuf {
	PathBuf::from(".cartridge")
}

/// The nearest directory at or above the working directory whose `.cartridge`
/// holds an `init.lua`, or the working directory itself.
pub fn root() -> PathBuf {
	let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
	cwd.ancestors()
		.find(|dir| dir.join(".cartridge").join("init.lua").is_file())
		.map_or(cwd.clone(), Path::to_path_buf)
}
