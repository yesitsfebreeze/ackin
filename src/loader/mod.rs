mod document;
pub(crate) mod entries;

use std::path::{Path, PathBuf};

use document::classify;
pub(crate) use document::{document, resolve, Declared};
pub use document::{is_bare_name, Cartridge, Command, Event, Grant, MANIFEST};

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

pub struct CartridgeInfo {
	pub entry: Entry,
	pub needs: Vec<String>,
	pub events: Vec<String>,
	pub listen: Vec<String>,
	pub grant: Option<Grant>,
	pub unread: Option<String>,
	pub error: Option<String>,
}

pub struct SettingsInfo {
	pub id: String,
	pub path: String,
	pub disabled: bool,
	pub specs: crate::settings::Specs,
	pub settled: serde_json::Value,
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

/// Every file `cartridge.load(name)` may open for a cartridge rooted at `root`,
/// in the order it tries them.
pub(crate) fn native_candidates(root: &Path, name: &str) -> Vec<PathBuf> {
	let symbol = name.replace(['-', '.'], "_");
	let files: Vec<String> = if cfg!(windows) {
		vec![format!("{symbol}.dll"), format!("lib{symbol}.dll")]
	} else {
		vec![
			format!("lib{symbol}.dylib"),
			format!("{symbol}.dylib"),
			format!("lib{symbol}.so"),
			format!("{symbol}.so"),
		]
	};
	[
		root.to_path_buf(),
		root.join("target/release"),
		root.join("target/debug"),
	]
	.iter()
	.flat_map(|dir| files.iter().map(move |file| dir.join(file)))
	.collect()
}

pub(crate) fn normalize(path: &Path) -> PathBuf {
	path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub fn descriptor() -> PathBuf {
	PathBuf::from(".cartridge")
}

pub fn root() -> PathBuf {
	let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
	cwd.ancestors()
		.find(|dir| dir.join(".cartridge").join("init.lua").is_file())
		.map_or(cwd.clone(), Path::to_path_buf)
}
