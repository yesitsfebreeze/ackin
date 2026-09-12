//! Read-only facts about this host's committed composition: the raw snapshot the
//! landscape projects. No entry evaluation, process spawning, model calls, or
//! configuration values belong in this view. The git tracked-files join and the
//! paging projection over this snapshot live in the `landscape` crate, behind
//! the memo record's `landscape` operation.
use std::collections::HashMap;
use std::path::Path;

use serde_json::{json, Value};

use crate::lua::Host;
use crate::runtime::Registry;

fn existing(root: &Path, name: &str) -> Option<std::path::PathBuf> {
	let path = root.join(name);
	path.exists().then_some(path)
}

/// The identifier a generation is known by: its profile entry's id when it is
/// one, otherwise the fiber's composition name.
fn names(reg: &Registry, loaded: &[crate::loader::Loaded]) -> HashMap<crate::runtime::Uid, String> {
	let mut map: HashMap<_, String> = reg
		.fibers
		.iter()
		.map(|(uid, fiber)| (*uid, fiber.name.clone()))
		.collect();
	for slot in loaded {
		if let Some(handle) = &slot.fiber {
			map.insert(handle.uid(), slot.entry.id.clone());
		}
	}
	map
}

impl Host {
	pub fn landscape(&self) -> Value {
		let loaded = self.loaded.lock();
		let reg = self.rt.reg.lock();
		let labels = names(&reg, &loaded);
		let owned: Vec<_> = loaded
			.iter()
			.filter_map(|slot| slot.fiber.as_ref())
			.collect();
		let entries: Vec<_> = loaded.iter().map(|slot| {
			let file = slot.entry.file(&self.dir);
			let root = file.parent().unwrap_or(&self.dir);
			let fiber = slot.fiber.as_ref().and_then(|handle| reg.fibers.get(&handle.uid()));
			let generation = slot.fiber.as_ref().map(|handle| handle.uid());
			let state = if slot.entry.disabled {
				"Disabled".to_owned()
			} else if let Some(fiber) = fiber {
				format!("{:?}", fiber.state)
			} else if !file.exists() {
				"Missing".to_owned()
			} else {
				"Failed".to_owned()
			};
			let dependencies = Self::dependencies(fiber, &slot.entry.inject, &reg, &labels);
			let sources: Vec<_> = slot.sources.iter().map(|source| {
				let stamp = source.stamp.map(|(bytes, modified)| json!({
					"bytes":bytes,
					"modified_ns":modified.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_nanos().to_string())
				}));
				json!({"path":source.path,"loaded_stamp":stamp,"changed":source.changed()})
			}).collect();
			let source = std::fs::read_to_string(root.join("cartridge.json")).ok()
				.and_then(|text| serde_json::from_str::<Value>(&text).ok())
				.and_then(|manifest| manifest["source"].as_str().map(str::to_owned));
			let implementation_source = ["Cargo.toml", "package.json"].iter().any(|name| root.join(name).is_file());
			json!({
				"id":slot.entry.id,"generation":generation,"state":state,
				"declarations_available":fiber.is_some(),
				"error":fiber.and_then(|fiber| fiber.error.as_ref()).or(slot.error.as_ref()),
				"path":file,"dir":root,
				"inject":fiber.map(|fiber| fiber.inject.as_slice()).unwrap_or(&slot.entry.inject),
				"provide":fiber.map(|fiber| fiber.provide.as_slice()).unwrap_or(&[]),
				"dependencies":dependencies,"sources":sources,
				"context":{
					"readme":existing(root,"README.md"),"memos":existing(root,".cartridge/memos"),
					"tests":existing(root,"tests"),"source":root,"implementation_source":implementation_source.then_some(root),
					"source_reference":source,
				}
			})
		}).collect();
		let entries = self.children(&reg, &labels, owned, entries);
		json!({"host_pid":std::process::id(),"profile":self.profile,"cartridge_root":self.dir,"entries":entries})
	}

	fn dependencies(
		fiber: Option<&crate::runtime::Fiber>,
		inject: &[String],
		reg: &Registry,
		labels: &HashMap<crate::runtime::Uid, String>,
	) -> Value {
		let inject = fiber.map(|fiber| fiber.inject.as_slice()).unwrap_or(inject);
		let dependencies: Vec<_> = inject
			.iter()
			.map(|key| {
				let provider = fiber
					.and_then(|fiber| reg.realm(&fiber.isolate, key))
					.and_then(|realm| reg.active_provider(realm));
				let owner = provider.as_ref().and_then(|provider| labels.get(provider));
				json!({"key":key, "provider":owner, "generation":provider})
			})
			.collect();
		json!(dependencies)
	}

	/// Fibers the profile did not load itself: children a cartridge composed at
	/// runtime. They get rows so a dependency that resolves to one names it.
	fn children(
		&self,
		reg: &Registry,
		labels: &HashMap<crate::runtime::Uid, String>,
		owned: Vec<&crate::runtime::FiberHandle>,
		mut entries: Vec<Value>,
	) -> Vec<Value> {
		for (uid, fiber) in reg.fibers.iter() {
			if owned.iter().any(|handle| handle.uid() == *uid) {
				continue;
			}
			let state = format!("{:?}", fiber.state);
			let parent = fiber.parent.as_ref().and_then(|parent| labels.get(parent));
			entries.push(json!({
				"id":fiber.name,"generation":Some(uid),"state":state,"dynamic":true,
				"parent":parent,"declarations_available":fiber.state == crate::runtime::State::Active,
				"error":fiber.error,
				"inject":fiber.inject,"provide":fiber.provide,
				"dependencies":Self::dependencies(Some(fiber), &[], reg, labels),
				"sources":[],"tracked":[],"context":null,
			}));
		}
		entries
	}
}
