use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::loader::{Cartridge, MANIFEST};

pub struct Installed {
	pub path: String,
	pub dir: PathBuf,
	pub name: String,
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
	pub fn scan(root: &Path) -> Self {
		let mut entries = BTreeMap::new();
		descend(root, "", &mut entries);
		Self { entries }
	}

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
