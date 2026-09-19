//! `expand`: everything every provider knows about one entity, followed along
//! its edges, with each fact held against the scheme owner's revision.

use std::collections::BTreeSet;

use futures::StreamExt;
use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::host::Host;

use super::admit::taken;
use super::protocol::{scheme_of, Assertion, Availability};
use super::registry::Registry;
use super::world::World;

/// How many subjects of one level are asked at the same time.
const PARALLEL: usize = 16;

impl Host {
	pub(super) async fn asp_expand(
		&self,
		registry: &Registry,
		entity: &str,
		depth: u64,
		limit: usize,
	) -> Result<Value> {
		let scheme = scheme_of(entity).expect("checked by the caller");
		if registry.expanding(scheme).is_empty() {
			return Err(Error::Argument(format!(
				"no loaded cartridge declares the scheme `{scheme}`"
			)));
		}
		let mut world = World::default();
		let mut seen: BTreeSet<String> = BTreeSet::new();
		let mut frontier = vec![entity.to_owned()];
		for _ in 0..depth {
			let mut next = Vec::new();
			let subjects: Vec<String> = std::mem::take(&mut frontier)
				.into_iter()
				.filter(|subject| seen.insert(subject.clone()))
				.collect();
			if subjects.is_empty() || world.nodes.len() >= limit {
				break;
			}
			// Every subject of one level is asked at once, in bounded parallel,
			// so a level costs its slowest provider, not the sum of them all.
			let asked: Vec<_> = futures::stream::iter(subjects)
				.map(|subject| async move {
					let scheme = scheme_of(&subject).expect("admitted ids are scheme:key");
					let providers = registry.expanding(scheme);
					let request = json!({ "op": "expand", "entity": subject });
					let answers = self.asp_ask(&providers, &request).await;
					(subject, providers, answers)
				})
				.buffered(PARALLEL)
				.collect()
				.await;
			for (subject, providers, answers) in asked {
				let scheme = scheme_of(&subject).expect("admitted ids are scheme:key");
				let owner = registry.owner(scheme);
				let mut facts = Vec::new();
				for (provider, answer) in providers.iter().zip(answers) {
					match taken(provider, answer) {
						Ok(admitted) => {
							world.source(&provider.id, Availability::Available, None);
							facts.push((provider.id.as_str(), admitted));
						}
						Err((state, error)) => world.source(&provider.id, state, Some(error)),
					}
				}
				// The owner's revision of the subject is the one every other
				// fact about it is held against.
				let canonical = facts
					.iter()
					.filter(|(id, _)| Some(*id) == owner)
					.flat_map(|(_, (nodes, _))| nodes)
					.find(|n| n.id == subject)
					.and_then(|n| n.revision.clone());
				for (contributor, (nodes, edges)) in facts {
					let assertion = |revision: Option<String>| Assertion {
						contributor: contributor.to_owned(),
						stale: canonical.is_some() && revision.is_some() && revision != canonical,
						revision,
					};
					for node in nodes {
						let owns = scheme_of(&node.id).and_then(|s| registry.owner(s))
							== Some(contributor);
						let given = assertion(node.revision.clone());
						world.assert(owns, node, given);
					}
					for edge in edges {
						for end in [&edge.from, &edge.to] {
							if !seen.contains(end) {
								next.push(end.clone());
							}
						}
						let given = assertion(edge.revision.clone());
						world.link(edge, given);
					}
				}
			}
			frontier = next;
		}
		Ok(json!({
			"entity": entity,
			"nodes": world.nodes.into_values().collect::<Vec<_>>(),
			"edges": world.edges,
			"actions": registry.actions(entity),
			"sources": world.sources.into_values().collect::<Vec<_>>(),
		}))
	}
}
