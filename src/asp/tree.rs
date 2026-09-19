//! The composition root and declared event types in the same provider graph.
use std::sync::Arc;

use serde_json::{json, Value};

use crate::host::Plan;

pub(super) fn answer(plans: &[Arc<Plan>], request: &Value) -> Value {
	let subject = request["entity"].as_str().unwrap_or_default();
	let searching = request["op"] == "search";
	let query = request["query"].as_str().unwrap_or_default();
	let mut nodes = Vec::new();
	let mut edges = Vec::new();
	if subject == "asp:root" {
		nodes.push(
			json!({"id":"asp:root", "name":"ASP", "description":"The running composition and its trace"}),
		);
		for plan in plans {
			edges.push(
				json!({"from":"asp:root", "to":format!("cartridge:{}", plan.id), "kind":"contains"}),
			);
			for root in &plan.asp.roots {
				edges.push(json!({"from":"asp:root", "to":root, "kind":"contains"}));
			}
		}
	}
	for plan in plans {
		let cartridge = format!("cartridge:{}", plan.id);
		for (name, event) in &plan.events {
			let id = format!("event:{name}");
			let description = event.description.as_deref().unwrap_or_default();
			if subject == id
				|| subject == cartridge
				|| (searching && super::rank::matches(query, &format!("{name} {description}")))
			{
				nodes.push(json!({"id":id, "name":name, "description":description,
					"tags":["event"], "attributes":{"host.event":event}}));
				edges.push(json!({"from":cartridge, "to":id, "kind":"provides"}));
			}
		}
	}
	json!({"nodes":nodes, "edges":edges})
}
