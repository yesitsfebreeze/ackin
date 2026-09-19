//! The host's `asp` service: one world, composed on request from every loaded
//! cartridge that declares an `asp` block. ASP keeps no store. It asks the
//! live providers each time, so a cartridge that leaves the composition takes
//! its types and its facts with it, and nothing has to be retracted.

pub mod protocol;
pub mod rank;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::host::{Host, Plan};

use protocol::{
	scheme_of, Asserted, Assertion, Availability, Bound, Declaration, Edge, Linked, Node, Source,
};

/// The key the host answers ASP on: `cartridge call asp`, and the `asp` host
/// method a cartridge reaches through `cartridge.host`.
pub const SERVICE: &str = "asp";

/// The event the base owns and listens to, so ASP is one of the agent's tools.
pub const TOOL: &str = "tool.asp";

const DEPTH: u64 = 1;
const MAX_DEPTH: u64 = 4;
const LIMIT: usize = 256;

struct Provider {
	id: String,
	event: String,
	declared: Declaration,
}

/// The types the loaded cartridges declare. Rebuilt per request from the
/// active plans, so it never outlives the composition it describes.
struct Registry {
	providers: Vec<Provider>,
}

impl Registry {
	fn of(plans: &[Arc<Plan>]) -> Self {
		let providers = plans
			.iter()
			.filter(|plan| !plan.asp.is_empty())
			.filter_map(|plan| {
				let event = plan.listen.iter().find(|k| k.starts_with("asp."))?;
				Some(Provider {
					id: plan.id.clone(),
					event: event.clone(),
					declared: plan.asp.clone(),
				})
			})
			.collect();
		Self { providers }
	}

	/// The first loaded cartridge that declares itself the scheme's owner.
	fn owner(&self, scheme: &str) -> Option<&str> {
		self.providers
			.iter()
			.find(|p| p.declared.schemes.get(scheme).is_some_and(|s| s.owner))
			.map(|p| p.id.as_str())
	}

	fn expanding(&self, scheme: &str) -> Vec<&Provider> {
		self.providers
			.iter()
			.filter(|p| p.declared.schemes.contains_key(scheme))
			.collect()
	}

	fn types(&self) -> Value {
		let mut schemes: BTreeMap<&str, Value> = BTreeMap::new();
		let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
		let mut attributes: BTreeMap<&str, &str> = BTreeMap::new();
		let mut actions = Vec::new();
		let mut clashes = Vec::new();
		for provider in &self.providers {
			for (name, scheme) in &provider.declared.schemes {
				let owner = self.owner(name);
				if scheme.owner && owner != Some(provider.id.as_str()) {
					clashes.push(format!(
						"`{}` declares itself owner of `{name}`, which `{}` already owns",
						provider.id,
						owner.unwrap_or_default()
					));
				}
				let entry = schemes.entry(name).or_insert_with(
					|| json!({ "owner": owner, "description": null, "contributors": [] }),
				);
				if entry["description"].is_null() || owner == Some(provider.id.as_str()) {
					if let Some(description) = &scheme.description {
						entry["description"] = json!(description);
					}
				}
				entry["contributors"]
					.as_array_mut()
					.expect("contributors")
					.push(json!(provider.id));
			}
			for kind in provider.declared.edges.keys() {
				edges.entry(kind).or_default().push(&provider.id);
			}
			for attribute in provider.declared.attributes.keys() {
				attributes.insert(attribute, &provider.id);
			}
			for action in &provider.declared.actions {
				let mut action = json!(action);
				action["contributor"] = json!(provider.id);
				actions.push(action);
			}
		}
		json!({
			"schemes": schemes,
			"edges": edges,
			"attributes": attributes,
			"actions": actions,
			"search": self.providers.iter().filter(|p| p.declared.search).map(|p| &p.id).collect::<Vec<_>>(),
			"clashes": clashes,
		})
	}

	fn actions(&self, entity: &str) -> Vec<Bound> {
		let Some((scheme, key)) = entity.split_once(':') else {
			return Vec::new();
		};
		let mut bound = Vec::new();
		for provider in &self.providers {
			for action in provider
				.declared
				.actions
				.iter()
				.filter(|a| a.applies_to == scheme)
			{
				bound.push(Bound {
					name: action.name.clone(),
					contributor: provider.id.clone(),
					effect: action.effect,
					tool: action.tool.clone(),
					args: bind(&action.args, entity, key),
					description: action.description.clone(),
				});
			}
		}
		bound
	}
}

fn bind(args: &Value, entity: &str, key: &str) -> Value {
	match args {
		Value::String(text) if text == "${entity}" => json!(entity),
		Value::String(text) if text == "${key}" => json!(key),
		Value::Array(items) => items.iter().map(|v| bind(v, entity, key)).collect(),
		Value::Object(map) => map
			.iter()
			.map(|(k, v)| (k.clone(), bind(v, entity, key)))
			.collect::<serde_json::Map<_, _>>()
			.into(),
		other => other.clone(),
	}
}

/// What one provider's answer holds once its declaration has been held
/// against it. The first fact the declaration does not cover refuses the
/// whole answer: a provider that asserts past its declaration is wrong about
/// what it is, and the rest of what it says is not trusted either.
fn admitted(provider: &Provider, answer: &Value) -> Result<(Vec<Asserted>, Vec<Linked>), String> {
	let rows = |field: &str| answer[field].as_array().cloned().unwrap_or_default();
	let mut nodes = Vec::new();
	for row in rows("nodes") {
		let node: Asserted = serde_json::from_value(row)
			.map_err(|e| format!("`{}` answered a malformed node: {e}", provider.id))?;
		let scheme = scheme_of(&node.id).ok_or_else(|| {
			format!(
				"`{}` asserted `{}`, which is not `scheme:key`",
				provider.id, node.id
			)
		})?;
		if !provider.declared.schemes.contains_key(scheme) {
			return Err(format!(
				"`{}` asserted a `{scheme}` node without declaring the scheme",
				provider.id
			));
		}
		if let Some(attribute) = node
			.attributes
			.keys()
			.find(|a| !provider.declared.attributes.contains_key(*a))
		{
			return Err(format!(
				"`{}` set attribute `{attribute}` without declaring it",
				provider.id
			));
		}
		nodes.push(node);
	}
	let mut edges = Vec::new();
	for row in rows("edges") {
		let edge: Linked = serde_json::from_value(row)
			.map_err(|e| format!("`{}` answered a malformed edge: {e}", provider.id))?;
		if !provider.declared.edges.contains_key(&edge.kind) {
			return Err(format!(
				"`{}` asserted a `{}` edge without declaring the edge kind",
				provider.id, edge.kind
			));
		}
		if scheme_of(&edge.from).is_none() || scheme_of(&edge.to).is_none() {
			return Err(format!(
				"`{}` asserted an edge whose ends are not `scheme:key`",
				provider.id
			));
		}
		edges.push(edge);
	}
	Ok((nodes, edges))
}

/// A provider's answer, or why it contributes nothing: it could not be
/// reached, or it answered past its declaration.
fn taken(
	provider: &Provider,
	answer: Result<Value>,
) -> Result<(Vec<Asserted>, Vec<Linked>), (Availability, String)> {
	let answer = answer.map_err(|e| (Availability::Unavailable, e.to_string()))?;
	admitted(provider, &answer).map_err(|e| (Availability::Refused, e))
}

#[derive(Default)]
struct World {
	nodes: BTreeMap<String, Node>,
	edges: Vec<Edge>,
	sources: BTreeMap<String, Source>,
}

impl World {
	fn source(&mut self, contributor: &str, state: Availability, error: Option<String>) {
		let known = self.sources.get(contributor).map(|s| s.state);
		if known.is_none() || known == Some(Availability::Available) {
			self.sources.insert(
				contributor.to_owned(),
				Source {
					contributor: contributor.to_owned(),
					state,
					error,
				},
			);
		}
	}

	fn assert(&mut self, owns: bool, asserted: Asserted, assertion: Assertion) {
		let node = self
			.nodes
			.entry(asserted.id.clone())
			.or_insert_with(|| Node {
				id: asserted.id.clone(),
				..Node::default()
			});
		// The scheme's owner describes the entity; anyone else only fills in
		// what nobody has said yet.
		let fill = |place: &mut String, given: String| {
			if !given.is_empty() && (owns || place.is_empty()) {
				*place = given;
			}
		};
		fill(&mut node.name, asserted.name);
		fill(&mut node.description, asserted.description);
		for word in asserted.when {
			if !node.when.contains(&word) {
				node.when.push(word);
			}
		}
		for tag in asserted.tags {
			if !node.tags.contains(&tag) {
				node.tags.push(tag);
			}
		}
		node.attributes.extend(asserted.attributes);
		if !node.contributors.contains(&assertion) {
			node.contributors
				.retain(|a| a.contributor != assertion.contributor);
			node.contributors.push(assertion);
		}
	}

	fn link(&mut self, linked: Linked, assertion: Assertion) {
		let same = |e: &Edge| {
			e.from == linked.from
				&& e.to == linked.to
				&& e.kind == linked.kind
				&& e.assertion.contributor == assertion.contributor
		};
		if !self.edges.iter().any(same) {
			self.edges.push(Edge {
				from: linked.from,
				to: linked.to,
				kind: linked.kind,
				assertion,
			});
		}
	}
}

impl Host {
	/// The `asp` service: `{op: types | expand | search | actions | act}`.
	pub async fn asp(&self, request: Value) -> Result<Value> {
		let registry = Registry::of(&self.active_plans());
		let entity = || {
			let id = request["entity"].as_str().unwrap_or_default();
			match scheme_of(id) {
				Some(_) => Ok(id),
				None => Err(Error::Argument(format!(
					"`entity` must be `scheme:key`, got `{id}`"
				))),
			}
		};
		match request["op"].as_str() {
			Some("types") => Ok(registry.types()),
			Some("expand") => {
				let depth = request["depth"]
					.as_u64()
					.unwrap_or(DEPTH)
					.clamp(1, MAX_DEPTH);
				let limit = request["limit"]
					.as_u64()
					.map_or(LIMIT, |n| (n as usize).clamp(1, LIMIT));
				self.asp_expand(&registry, entity()?, depth, limit).await
			}
			Some("search") => {
				let query = request["query"].as_str().unwrap_or_default();
				if query.trim().is_empty() {
					return Err(Error::Argument("`query` must not be empty".into()));
				}
				let limit = request["limit"]
					.as_u64()
					.map_or(LIMIT, |n| (n as usize).clamp(1, LIMIT));
				self.asp_search(&registry, query, limit).await
			}
			Some("actions") => Ok(json!({ "actions": registry.actions(entity()?) })),
			Some("act") => {
				let entity = entity()?;
				let name = request["action"].as_str().unwrap_or_default();
				let action = registry
					.actions(entity)
					.into_iter()
					.find(|a| a.name == name)
					.ok_or_else(|| {
						Error::Argument(format!(
							"no loaded cartridge offers `{name}` on `{entity}`"
						))
					})?;
				// The tool event is the whole dispatch: whatever answers or
				// refuses it, policy included, reaches the caller as it is.
				Ok(Box::pin(self.bail(&action.tool, action.args))
					.await?
					.unwrap_or(Value::Null))
			}
			other => Err(Error::Argument(format!(
				"unknown asp op {other:?}; one of types, expand, search, actions, act"
			))),
		}
	}

	/// The tool envelope every harness speaks: `describe`, `call`, `cancel`.
	/// It offers no `act`: an action names a tool, and the agent runs that
	/// tool through its own dispatch, where policy is asked.
	pub async fn asp_tool(&self, args: Value) -> Result<Value> {
		match args["op"].as_str() {
			Some("describe") => Ok(json!({
				"name": "asp",
				"description": "The one interface to the project's world. `search` finds entities (files, symbols, memos, ...) by words; `expand` returns everything every cartridge knows about one entity `scheme:key` (nodes, edges, attributes, staleness) and the actions you can run on it, each naming a tool and its arguments; `types` lists the schemes and edge kinds that exist.",
				"reads": true,
				"input_schema": {
					"type": "object",
					"properties": {
						"op": { "type": "string", "enum": ["types", "expand", "search", "actions"] },
						"entity": { "type": "string", "description": "scheme:key, for example file:src/a.rs" },
						"query": { "type": "string" },
						"depth": { "type": "integer", "minimum": 1, "maximum": MAX_DEPTH },
						"limit": { "type": "integer", "minimum": 1, "maximum": LIMIT }
					},
					"required": ["op"],
					"additionalProperties": false
				}
			})),
			Some("cancel") => Ok(json!({ "content": "nothing to cancel", "error": false })),
			Some("call") if args["input"]["op"] == "act" => Ok(json!({
				"content": "asp does not run actions; call the tool the action names",
				"error": true,
			})),
			Some("call") => Ok(match self.asp(args["input"].clone()).await {
				Ok(answer) => json!({ "content": answer.to_string(), "error": false }),
				Err(error) => json!({ "content": error.to_string(), "error": true }),
			}),
			other => Err(Error::Argument(format!("unknown tool.asp op: {other:?}"))),
		}
	}

	async fn asp_ask(&self, providers: &[&Provider], request: &Value) -> Vec<Result<Value>> {
		futures::future::join_all(
			providers
				.iter()
				.map(|p| self.send_to(&p.id, &p.event, request.clone())),
		)
		.await
	}

	async fn asp_expand(
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
			for subject in std::mem::take(&mut frontier) {
				if !seen.insert(subject.clone()) || world.nodes.len() >= limit {
					continue;
				}
				let scheme = scheme_of(&subject).expect("admitted ids are scheme:key");
				let providers = registry.expanding(scheme);
				let request = json!({ "op": "expand", "entity": subject });
				let answers = self.asp_ask(&providers, &request).await;
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

	async fn asp_search(&self, registry: &Registry, query: &str, limit: usize) -> Result<Value> {
		let providers: Vec<&Provider> = registry
			.providers
			.iter()
			.filter(|p| p.declared.search)
			.collect();
		let request = json!({ "op": "search", "query": query, "limit": limit });
		let answers = self.asp_ask(&providers, &request).await;
		let mut world = World::default();
		let mut order = Vec::new();
		for (provider, answer) in providers.iter().zip(answers) {
			match taken(provider, answer) {
				Ok((nodes, _)) => {
					world.source(&provider.id, Availability::Available, None);
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
		Ok(json!({
			"query": query,
			"hits": hits,
			"sources": world.sources.into_values().collect::<Vec<_>>(),
		}))
	}
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/asp.rs"]
mod tests;
