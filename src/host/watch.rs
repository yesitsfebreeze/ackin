use std::path::{Path, PathBuf};
use std::sync::Arc;

use notify::{RecursiveMode, Watcher};

use crate::error::Result;
use crate::loader::normalize;

use super::Host;

impl Host {
	pub fn watch(self: &Arc<Self>) -> Result<tokio::task::JoinHandle<()>> {
		self.stop_when_project_gone();
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
		if self.descriptor != self.dir {
			watcher.watch(&self.descriptor, RecursiveMode::NonRecursive)?;
		}
		let mut watched = std::collections::HashSet::from([self.descriptor.clone()]);
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
				let descriptor = changed.iter().any(|path| {
					path.parent() == Some(host.descriptor.as_path())
						&& matches!(
							path.file_name().and_then(|n| n.to_str()),
							Some("init.lua" | "config.lua")
						)
				});
				if descriptor {
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

	/// A host outlives no project: its directory or descriptor directory
	/// missing on two consecutive ticks stops it. Directories only, so an
	/// editor's save of `init.lua` never counts, and an unreadable path is
	/// present. Its own task, so a long reconcile neither starves the tick nor
	/// bursts two misses at once.
	fn stop_when_project_gone(&self) {
		let (dir, descriptor, stop) = (
			self.dir.clone(),
			self.descriptor.clone(),
			self.stop_signal(),
		);
		tokio::spawn(async move {
			let mut tick = tokio::time::interval(std::time::Duration::from_secs(2));
			tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
			let mut misses = 0;
			loop {
				tokio::select! {
					() = stop.cancelled() => return,
					_ = tick.tick() => {}
				}
				let gone = |path: &Path| matches!(path.try_exists(), Ok(false));
				misses = if gone(&dir) || gone(&descriptor) {
					misses + 1
				} else {
					0
				};
				if misses >= 2 {
					tracing::warn!(target: "cartridge", dir = %dir.display(), "project gone, stopping");
					stop.cancel();
					return;
				}
			}
		});
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
