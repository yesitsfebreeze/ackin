use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::loader::{Cartridge, MANIFEST};

pub struct Installed {
	/// The identity: never the bare `name`, which is not unique across the tree.
	pub path: String,
	pub dir: PathBuf,
	/// Empty when the document would not read.
	pub name: String,
	/// Seen by every lookup made from inside its parent's subtree, and by
	/// nothing outside the parent unless the parent passes it on.
	pub listen: Vec<String>,
	pub needs: Vec<String>,
	pub unread: Option<String>,
}

impl Installed {
	pub fn parent(&self) -> Option<&str> {
		self.path.rsplit_once('/').map(|(head, _)| head)
	}

	pub fn offers(&self) -> impl Iterator<Item = &String> {
		self.listen.iter()
	}
}

pub enum Bound<'a> {
	One(&'a Installed),
	Clashed(Vec<&'a Installed>),
	None,
}

impl Bound<'_> {
	pub fn paths(&self) -> Vec<&str> {
		match self {
			Bound::One(e) => vec![e.path.as_str()],
			Bound::Clashed(offered) => offered.iter().map(|e| e.path.as_str()).collect(),
			Bound::None => Vec::new(),
		}
	}

	pub fn is_clashed(&self) -> bool {
		matches!(self, Bound::Clashed(_))
	}
}

pub struct Ledger {
	entries: BTreeMap<String, Installed>,
}

impl Ledger {
	/// A root that does not exist is an empty ledger, not an error: nothing
	/// installed is a state the host runs in.
	pub fn scan(root: &Path) -> Self {
		let mut entries = BTreeMap::new();
		descend(root, "", &mut entries);
		Self { entries }
	}

	/// In path order, not filesystem order: backed by a `BTreeMap`.
	pub fn entries(&self) -> impl Iterator<Item = &Installed> {
		self.entries.values()
	}

	pub fn get(&self, path: &str) -> Option<&Installed> {
		self.entries.get(path)
	}

	pub fn len(&self) -> usize {
		self.entries.len()
	}

	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}

	/// Excludes `from` itself.
	pub fn resolve(&self, from: &str, key: &str) -> Bound<'_> {
		let offered: Vec<&Installed> = self
			.entries()
			.filter(|e| e.path != from && e.offers().any(|k| k == key))
			.collect();
		match offered.len() {
			0 => Bound::None,
			1 => Bound::One(offered[0]),
			_ => Bound::Clashed(offered),
		}
	}

	pub fn bindings(&self) -> Vec<(&Installed, &String, Bound<'_>)> {
		self.entries()
			.flat_map(|e| {
				e.needs
					.iter()
					.map(move |key| (e, key, self.resolve(&e.path, key)))
			})
			.collect()
	}
}

/// A directory holding no document is not descended into: a nested cartridge
/// it hid would be reachable by nobody, but that subtree was never a
/// cartridge's to expose.
fn descend(dir: &Path, scope: &str, into: &mut BTreeMap<String, Installed>) {
	let Ok(read) = std::fs::read_dir(dir) else {
		return;
	};
	let mut folders: Vec<PathBuf> = read
		.flatten()
		.map(|e| e.path())
		.filter(|p| p.join(MANIFEST).is_file())
		.collect();
	folders.sort();
	for folder in folders {
		let Some(name) = folder.file_name().and_then(|n| n.to_str()) else {
			continue;
		};
		let path = if scope.is_empty() {
			name.to_owned()
		} else {
			format!("{scope}/{name}")
		};
		into.insert(path.clone(), read_entry(&folder, path.clone()));
		descend(&folder, &path, into);
	}
}

/// Declarations are recorded from the manifest whether or not the Lua entry
/// it names actually exists.
fn read_entry(folder: &Path, path: String) -> Installed {
	let manifest = folder.join(MANIFEST);
	match Cartridge::document(&manifest) {
		Ok(doc) => Installed {
			path,
			dir: folder.to_path_buf(),
			name: doc.name,
			listen: doc.listen,
			needs: doc.needs,
			unread: None,
		},
		Err(e) => Installed {
			path,
			dir: folder.to_path_buf(),
			name: String::new(),
			listen: Vec::new(),
			needs: Vec::new(),
			unread: Some(e.to_string()),
		},
	}
}
