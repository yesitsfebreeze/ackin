//! The types the loaded cartridges declare: who owns a scheme, who is asked
//! about one, and which actions an entity carries.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{json, Value};

use crate::host::Plan;

use super::protocol::{Bound, Declaration};
use super::HOST;

pub(super) struct Provider {
	pub(super) id: String,
	pub(super) event: String,
	pub(super) declared: Declaration,
}

/// The types the loaded cartridges declare. Rebuilt per request from the
/// active plans, so it never outlives the composition it describes.
pub(super) struct Registry {
	pub(super) providers: Vec<Provider>,
}

impl Registry {
	pub(super) fn of(plans: &[Arc<Plan>]) -> Self {
		let providers = plans
			.iter()
			.filter(|plan| !plan.asp.is_empty())
			.filter_map(|plan| {
				// The base answers in process, so it has no event to be asked on.
				let event = match plan.id == HOST {
					true => "",
					false => plan.listen.iter().find(|k| k.starts_with("asp."))?,
				};
				Some(Provider {
					id: plan.id.clone(),
					event: event.to_owned(),
					declared: plan.asp.clone(),
				})
			})
			.collect();
		Self { providers }
	}

	/// The first loaded cartridge that declares itself the scheme's owner.
	pub(super) fn owner(&self, scheme: &str) -> Option<&str> {
		self.providers
			.iter()
			.find(|p| p.declared.schemes.get(scheme).is_some_and(|s| s.owner))
			.map(|p| p.id.as_str())
	}

	pub(super) fn expanding(&self, scheme: &str) -> Vec<&Provider> {
		self.providers
			.iter()
			.filter(|p| p.declared.schemes.contains_key(scheme))
			.collect()
	}

	/// The registry, and with it every declared event: an event is a type of
	/// the same world, declared in the same manifest.
	pub(super) fn types(&self, plans: &[Arc<Plan>]) -> Value {
		let events: BTreeMap<&str, Value> = plans
			.iter()
			.flat_map(|plan| {
				plan.events.iter().map(move |(name, event)| {
					let mut entry = json!(event);
					entry["owner"] = json!(plan.id);
					(name.as_str(), entry)
				})
			})
			.collect();
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
			"events": events,
			"clashes": clashes,
		})
	}

	pub(super) fn actions(&self, entity: &str) -> Vec<Bound> {
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
