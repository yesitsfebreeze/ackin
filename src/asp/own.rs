//! What the base itself contributes: every cartridge in the profile with its
//! lifecycle, and every tool event, which only the base knows in full.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{json, Value};

use crate::host::Plan;

use super::protocol::{self, Declaration};
use super::rank;

/// One cartridge of the profile, as the base sees it.
pub(crate) struct Member {
	pub(crate) id: String,
	pub(crate) state: Value,
	pub(crate) generation: u64,
	pub(crate) error: Option<String>,
}

/// What the base itself adds to the world: every cartridge in the profile and
/// every tool event, which only the base knows in full.
pub(crate) fn own_types() -> Declaration {
	let scheme = |description: &str| protocol::Scheme {
		schema: None,
		description: Some(description.to_owned()),
		owner: true,
	};
	Declaration {
		schemes: BTreeMap::from([
			(
				"asp".to_owned(),
				scheme("the extensible composition tree, starting at asp:root"),
			),
			(
				"event".to_owned(),
				scheme("a declared event type, by its full name"),
			),
			(
				"cartridge".to_owned(),
				scheme("a cartridge of the profile, by its id; its revision is its start count"),
			),
			(
				"tool".to_owned(),
				scheme("a tool event, by its name without the `tool.` prefix"),
			),
		]),
		edges: BTreeMap::from([
			(
				"contains".to_owned(),
				protocol::Described {
					schema: None,
					description: Some("an ASP tree entry contains another entry".to_owned()),
				},
			),
			(
				"provides".to_owned(),
				protocol::Described {
					schema: None,
					description: Some("the cartridge owns the tool event".to_owned()),
				},
			),
		]),
		attributes: [
			(
				"host.state",
				"the lifecycle state: disabled, waiting, starting, active, stopping or failed",
			),
			("host.error", "why the cartridge failed or waits"),
			(
				"host.event",
				"the event declaration, including its payload schema and frame",
			),
		]
		.into_iter()
		.map(|(name, description)| {
			let described = protocol::Described {
				schema: None,
				description: Some(description.to_owned()),
			};
			(name.to_owned(), described)
		})
		.collect(),
		search: true,
		..Declaration::default()
	}
}

/// A `cartridge:` node. Its revision is its start count, so a fact another
/// cartridge asserted about it before a restart reads as stale.
fn member_node(member: &Member) -> Value {
	let mut attributes = json!({ "host.state": member.state });
	if let Some(error) = &member.error {
		attributes["host.error"] = json!(error);
	}
	json!({
		"id": format!("cartridge:{}", member.id),
		"name": member.id,
		"revision": member.generation.to_string(),
		"tags": ["cartridge"],
		"attributes": attributes,
	})
}

/// The base's own answer to an expand or a search.
pub(super) fn own_answer(plans: &[Arc<Plan>], roster: &[Member], request: &Value) -> Value {
	let subject = request["entity"].as_str().unwrap_or_default();
	let query = request["query"].as_str().unwrap_or_default();
	let searching = request["op"] == "search";
	let mut tree = super::tree::answer(plans, request);
	let mut nodes = tree["nodes"]
		.as_array_mut()
		.map(std::mem::take)
		.unwrap_or_default();
	let mut edges = tree["edges"]
		.as_array_mut()
		.map(std::mem::take)
		.unwrap_or_default();
	let host = Member {
		id: "host".to_owned(),
		state: json!("active"),
		generation: u64::from(std::process::id()),
		error: None,
	};
	for member in roster.iter().chain(std::iter::once(&host)) {
		let named = match searching {
			true => rank::matches(query, &member.id),
			false => subject == format!("cartridge:{}", member.id),
		};
		if named {
			nodes.push(member_node(member));
		}
	}
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
	for (owner, key, description) in tools {
		let (tool, cartridge) = (format!("tool:{key}"), format!("cartridge:{owner}"));
		let node = json!({ "id": tool, "name": key, "description": description, "tags": ["tool"] });
		if searching {
			if rank::matches(query, &format!("{key} {description}")) {
				nodes.push(node);
			}
		} else if subject == tool || subject == cartridge {
			if subject == tool {
				nodes.extend(
					roster
						.iter()
						.chain(std::iter::once(&host))
						.filter(|m| m.id == owner)
						.map(member_node),
				);
			}
			nodes.push(node);
			edges.push(json!({ "from": cartridge, "to": tool, "kind": "provides" }));
		}
	}
	json!({ "nodes": nodes, "edges": edges })
}
