//! What the base itself contributes: every loaded cartridge and every tool
//! event, which only the base knows in full.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{json, Value};

use crate::host::Plan;

use super::protocol::{self, Declaration};
use super::rank;

/// What the base itself adds to the world: every loaded cartridge and every
/// tool event, which only the base knows in full.
pub(crate) fn own_types() -> Declaration {
	let scheme = |description: &str| protocol::Scheme {
		description: Some(description.to_owned()),
		owner: true,
	};
	Declaration {
		schemes: BTreeMap::from([
			(
				"cartridge".to_owned(),
				scheme("a loaded cartridge, by its id"),
			),
			(
				"tool".to_owned(),
				scheme("a tool event, by its name without the `tool.` prefix"),
			),
		]),
		edges: BTreeMap::from([(
			"provides".to_owned(),
			protocol::Described {
				description: Some("the cartridge owns the tool event".to_owned()),
			},
		)]),
		search: true,
		..Declaration::default()
	}
}

/// The base's own answer to an expand or a search.
pub(super) fn own_answer(plans: &[Arc<Plan>], request: &Value) -> Value {
	let tools = plans.iter().flat_map(|plan| {
		plan.events.iter().filter_map(move |(name, event)| {
			let key = name.strip_prefix("tool.")?;
			Some((
				plan.id.as_str(),
				key,
				event.description.clone().unwrap_or_default(),
			))
		})
	});
	let subject = request["entity"].as_str().unwrap_or_default();
	let query = request["query"].as_str().unwrap_or_default();
	let searching = request["op"] == "search";
	let mut nodes = Vec::new();
	let mut edges = Vec::new();
	for (owner, key, description) in tools {
		let (tool, cartridge) = (format!("tool:{key}"), format!("cartridge:{owner}"));
		let node = json!({ "id": tool, "name": key, "description": description, "tags": ["tool"] });
		if searching {
			if rank::matches(query, &format!("{key} {description}")) {
				nodes.push(node);
			}
		} else if subject == tool || subject == cartridge {
			nodes.push(node);
			nodes.push(json!({ "id": cartridge, "name": owner, "tags": ["cartridge"] }));
			edges.push(json!({ "from": cartridge, "to": tool, "kind": "provides" }));
		}
	}
	if !searching && nodes.is_empty() {
		if let Some(plan) = plans
			.iter()
			.find(|p| format!("cartridge:{}", p.id) == subject)
		{
			nodes.push(json!({ "id": subject, "name": plan.id, "tags": ["cartridge"] }));
		}
	}
	json!({ "nodes": nodes, "edges": edges })
}
