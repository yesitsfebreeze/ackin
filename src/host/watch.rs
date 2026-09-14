//! Changed sources restart their cartridge; a changed `init.lua` or
//! `config.lua` reconciles the profile.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use notify::{RecursiveMode, Watcher};

use crate::error::Result;
use crate::loader::normalize;

use super::Host;

impl Host {
	/// Watch until the returned task is aborted.
	pub fn watch(self: &Arc<Self>) -> Result<tokio::task::JoinHandle<()>> {
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
		if self.profile != self.dir {
			watcher.watch(&self.profile, RecursiveMode::NonRecursive)?;
		}
		let mut watched = std::collections::HashSet::from([self.profile.clone()]);
		self.watch_sources(&mut watcher, &mut watched)?;
		let host = self.clone();
		Ok(tokio::spawn(async move {
			while let Some(first) = rx.recv().await {
				let mut changed = vec![first];
				tokio::time::sleep(crate::settings::host().watch_debounce()).await;
				while let Ok(path) = rx.try_recv() {
					changed.push(path);
				}
				changed.sort();
				changed.dedup();
				let profile = changed.iter().any(|path| {
					path.parent() == Some(host.profile.as_path())
						&& matches!(
							path.file_name().and_then(|n| n.to_str()),
							Some("init.lua" | "config.lua")
						)
				});
				if profile {
					if let Err(error) = host.reconcile().await {
						tracing::error!(target: "cartridge", cartridge = "init.lua", "{error}");
					}
				}
				host.replace_changed(&changed).await;
				if let Err(error) = host.watch_sources(&mut watcher, &mut watched) {
					tracing::error!(target: "cartridge", "watch: {error}");
				}
			}
		}))
	}

	fn watch_sources(
		&self,
		watcher: &mut impl Watcher,
		watched: &mut std::collections::HashSet<PathBuf>,
	) -> notify::Result<()> {
		let dirs: Vec<PathBuf> = self
			.slots
			.lock()
			.iter()
			.flat_map(|slot| slot.sources.iter())
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
}
