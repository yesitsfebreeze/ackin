//! The fabric: what the composition is, and what it says it holds.
//!
//! Two questions, both answered here. The snapshot is read-only facts about
//! this host's committed composition — no entry evaluation, process spawning,
//! model calls or configuration values belong in it. The graph is what the
//! live composition announces over that snapshot: core states the entries it
//! knows first-hand and asks every active cartridge for the rest.
//!
//! [`graph`] carries the node and edge types every contributor answers in, and
//! the one search over them. [`evidence`] carries the bounded-evidence contract
//! the harness, the proxy and the memo record share. The git tracked-files
//! join and the paging projection over a snapshot belong to the cartridge that
//! serves them, not here.
#[path = "evidence.rs"]
pub mod evidence;
#[path = "graph.rs"]
pub mod graph;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;

use serde_json::{json, Map, Value};

use crate::lua::Host;
use crate::runtime::Registry;

/// The event core fires to ask the composition what it contributes to the
/// graph. A cartridge answers `{"nodes":[…],"edges":[…]}` in the shapes
/// [`graph`] reads, or answers nothing at all. Which cartridges are
/// composed is therefore what the graph is: an entry that is not loaded is not
/// asked, and one that is loaded contributes without core knowing what it is.
pub const ANNOUNCE: &str = "fabric.announce";

/// What core contributes about one composed entry without asking anybody: the
/// entry is a node, and what it provides and injects are its edges. A cartridge
/// has no prose of its own here — the capability keys are its vocabulary until
/// it announces something better keyed to the same id.
fn composed(entry: &Value) -> Value {
	let keys = |field: &str| -> Vec<&str> {
		entry[field]
			.as_array()
			.map(|keys| keys.iter().filter_map(Value::as_str).collect())
			.unwrap_or_default()
	};
	let Some(id) = entry["id"].as_str().filter(|id| !id.is_empty()) else {
		return json!({});
	};
	let mut tags = keys("provide");
	tags.extend(keys("inject"));
	let edges: Vec<Value> = [("provide", "provides"), ("inject", "injects")]
		.into_iter()
		.flat_map(|(field, kind)| {
			keys(field)
				.into_iter()
				.map(move |key| json!({"from":id,"to":key,"kind":kind}))
		})
		.collect();
	json!({
		"nodes": [{"kind":"cartridge","key":id,"name":id,
				   "description":entry["state"].as_str().unwrap_or(""),"tags":tags}],
		"edges": edges,
	})
}

/// Fold one contribution in. A node needs a nonempty key and an edge needs all
/// three of its ends; anything else is dropped rather than failing the graph,
/// because the graph is derived state and a malformed answer is the answering
/// cartridge's defect, not the asker's. `from` is stamped here, from the
/// registry's own name for the answering fiber — never read out of the answer,
/// so a contribution cannot claim to come from somewhere else. Core's own rows
/// carry `from: null`: the host's snapshot said it, nobody announced it.
/// Returns whether anything was actually contributed.
fn fold(
	nodes: &mut BTreeMap<String, Value>,
	edges: &mut BTreeSet<(String, String, String)>,
	from: Option<&str>,
	answer: &Value,
) -> bool {
	let mut contributed = false;
	for node in answer["nodes"].as_array().into_iter().flatten() {
		let Some(key) = node["key"].as_str().filter(|key| !key.is_empty()) else {
			continue;
		};
		let mut row = node.as_object().cloned().unwrap_or_else(Map::new);
		row.insert("from".into(), json!(from));
		nodes.insert(key.to_owned(), Value::Object(row));
		contributed = true;
	}
	for edge in answer["edges"].as_array().into_iter().flatten() {
		let ends = (
			edge["from"].as_str(),
			edge["to"].as_str(),
			edge["kind"].as_str(),
		);
		if let (Some(tail), Some(head), Some(kind)) = ends {
			if tail.is_empty() || head.is_empty() {
				continue;
			}
			edges.insert((tail.to_owned(), head.to_owned(), kind.to_owned()));
			contributed = true;
		}
	}
	contributed
}

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
	/// What each live generation is called, for attributing an answer to the
	/// cartridge that gave it. The same identity the snapshot rows carry, so
	/// a gathered contribution and its entry row name the same thing.
	pub(crate) fn labels(&self) -> HashMap<crate::runtime::Uid, String> {
		// Same order the snapshot takes both locks in; taking them the other
		// way round here would be the one place they could deadlock.
		let loaded = self.loaded.lock();
		let reg = self.rt.reg.lock();
		names(&reg, &loaded)
	}

	/// The graph over the live composition: what core knows first-hand about
	/// the entries, and what every active cartridge announces about itself on
	/// top of it. `scope` is the asking side's context — a session cwd, say —
	/// passed through to the contributors that need it, since core has no
	/// session of its own.
	///
	/// Nodes are keyed, so a cartridge that describes a key core already seeded
	/// replaces that row rather than doubling it; the fold runs in entry-id
	/// order so which of two contributors wins is the same on every call, even
	/// though the answers themselves arrive in whatever order they finish.
	pub async fn graph(&self, scope: Value) -> Value {
		let mut nodes = BTreeMap::new();
		let mut edges = BTreeSet::new();
		let snapshot = self.snapshot();
		for entry in snapshot["entries"].as_array().into_iter().flatten() {
			fold(&mut nodes, &mut edges, None, &composed(entry));
		}
		let payload = json!({
			"event": ANNOUNCE, "scope": scope,
			"profile": self.profile, "cartridge_root": self.dir,
		});
		let answers = self
			.rt
			.ctx()
			.gather(ANNOUNCE, Arc::new(payload) as crate::runtime::Value)
			.await;
		let labels = self.labels();
		let mut answers: Vec<(String, Value)> = answers
			.iter()
			.map(|(uid, answer)| {
				let name = labels.get(uid).cloned().unwrap_or_else(|| uid.to_string());
				(name, self.json_of(answer))
			})
			.collect();
		answers.sort_by(|(a, _), (b, _)| a.cmp(b));
		let mut announced = Vec::new();
		for (from, answer) in &answers {
			if fold(&mut nodes, &mut edges, Some(from), answer) {
				announced.push(from.clone());
			}
		}
		json!({
			"nodes": nodes.into_values().collect::<Vec<_>>(),
			"edges": edges.into_iter().map(|(from, to, kind)| json!({"from":from,"to":to,"kind":kind})).collect::<Vec<_>>(),
			"announced": announced,
		})
	}

	pub fn snapshot(&self) -> Value {
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
