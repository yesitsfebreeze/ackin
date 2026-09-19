//! `search`: every search provider's nodes, merged and ranked.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

use crate::error::Result;
use crate::host::Host;

use super::admit::taken;
use super::protocol::{scheme_of, Assertion, Availability};
use super::rank;
use super::registry::{Provider, Registry};
use super::world::World;

impl Host {
	pub(super) async fn asp_search(
		&self,
		registry: &Registry,
		query: &str,
		limit: usize,
		deadline: Option<tokio::time::Instant>,
	) -> Result<Value> {
		let providers: Vec<&Provider> = registry
			.providers
			.iter()
			.filter(|p| p.declared.search)
			.collect();
		let request = json!({ "op": "search", "query": query, "limit": limit });
		let answers = self.asp_ask(&providers, &request, deadline).await;
		let mut world = World::default();
		let mut order = Vec::new();
		for (provider, answer) in providers.iter().zip(answers) {
			match taken(provider, answer) {
				Ok((nodes, edges)) => {
					world.source(&provider.id, Availability::Available, None);
					for edge in edges {
						let assertion = Assertion {
							contributor: provider.id.clone(),
							revision: edge.revision.clone(),
							stale: false,
						};
						world.link(edge, assertion);
					}
					for node in nodes {
						let owns = scheme_of(&node.id).and_then(|s| registry.owner(s))
							== Some(provider.id.as_str());
						let assertion = Assertion {
							contributor: provider.id.clone(),
							revision: node.revision.clone(),
							stale: false,
						};
						if !order.contains(&node.id) {
							order.push(node.id.clone());
						}
						world.assert(owns, node, assertion);
					}
				}
				Err((state, error)) => world.source(&provider.id, state, Some(error)),
			}
		}
		let nodes = order
			.iter()
			.filter_map(|id| world.nodes.remove(id))
			.collect();
		// ponytail: no use counts yet, so standing is 1 for every node; the
		// event ring supplies them once it indexes events by entity id.
		let mut hits = rank::rank(query, nodes, &BTreeMap::new());
		hits.truncate(limit);
		// An edge is kept while both of its ends are among the hits: a match
		// is a range, and the edge is what says which file holds it.
		let kept: BTreeSet<&str> = hits.iter().map(|hit| hit.node.id.as_str()).collect();
		world
			.edges
			.retain(|e| kept.contains(e.from.as_str()) && kept.contains(e.to.as_str()));
		Ok(json!({
			"query": query,
			"hits": hits,
			"edges": world.edges,
			"sources": world.sources.into_values().collect::<Vec<_>>(),
		}))
	}
}
