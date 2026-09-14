//! The sources observed: every loaded entry's files, the profile beside
//! them, and the host's own tree, so a change is a replacement.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use notify::{RecursiveMode, Watcher};

use crate::error::Result;
use crate::lua::Host;

use super::normalize;

impl Host {
	pub(super) fn watch_sources(
		&self,
		watcher: &mut impl Watcher,
		watched: &mut std::collections::HashSet<PathBuf>,
	) -> notify::Result<()> {
		let dirs: Vec<_> = self
			.loaded
			.lock()
			.iter()
			.flat_map(|l| l.sources.iter())
			.filter_map(|source| source.path.parent().map(Path::to_path_buf))
			.collect();
		for dir in dirs {
			if !watched.contains(&dir) && !dir.starts_with(&self.dir) {
				watcher.watch(&dir, RecursiveMode::NonRecursive)?;
				watched.insert(dir);
			}
		}
		Ok(())
	}

	pub fn watch(self: &Arc<Self>) -> Result<()> {
		self.watch_mode().map(|_| ())
	}

	pub(super) fn watch_mode(self: &Arc<Self>) -> Result<tokio::task::JoinHandle<()>> {
		let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<PathBuf>();
		let mut watcher =
			notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
				if let Ok(event) = event {
					if matches!(event.kind, notify::EventKind::Access(_)) {
						return;
					}
					for path in event.paths {
						let _ = tx.send(normalize(&path));
					}
				}
			})?;
		watcher.watch(&self.dir, RecursiveMode::Recursive)?;
		// A source tree beside the host rebuilds on edit; a bundle has none.
		for path in [
			std::path::Path::new("src"),
			std::path::Path::new("Cargo.toml"),
		] {
			if path.exists() {
				watcher.watch(path, RecursiveMode::Recursive)?;
			}
		}
		if self.profile != self.dir {
			watcher.watch(&self.profile, RecursiveMode::NonRecursive)?;
		}
		let mut watched = std::collections::HashSet::from([self.profile.clone()]);
		self.watch_sources(&mut watcher, &mut watched)?;
		let host = self.clone();
		let task = tokio::spawn(async move {
			while let Some(first) = rx.recv().await {
				let mut changed = vec![first];
				tokio::time::sleep(crate::settings::host().watch_debounce()).await;
				while let Ok(p) = rx.try_recv() {
					changed.push(p);
				}
				changed.sort();
				changed.dedup();
				if changed.iter().any(|path| {
					path.parent() == Some(host.profile.as_path())
						&& matches!(
							path.file_name().and_then(|n| n.to_str()),
							Some("init.lua" | "config.lua")
						)
				}) {
					if let Err(e) = host.reconcile().await {
						host.report("init.lua", e);
					}
				}
				host.replace_changed(&changed).await;
				if let Err(e) = host.watch_sources(&mut watcher, &mut watched) {
					host.report("watch", e);
				}
			}
		});
		Ok(task)
	}
}
