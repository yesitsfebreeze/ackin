//! Admission: a provider's answer held against its own declaration.

use serde_json::Value;

use crate::error::Result;

use super::protocol::{scheme_of, Asserted, Availability, Linked};
use super::registry::Provider;

/// What one provider's answer holds once its declaration has been held
/// against it. The first fact the declaration does not cover refuses the
/// whole answer: a provider that asserts past its declaration is wrong about
/// what it is, and the rest of what it says is not trusted either.
fn admitted(provider: &Provider, answer: &Value) -> Result<(Vec<Asserted>, Vec<Linked>), String> {
	let validate = |name: &str, schema: &Option<Value>, value: &Value| -> Result<(), String> {
		if let Some(schema) = schema {
			let validator = jsonschema::validator_for(schema)
				.map_err(|e| format!("`{}` schema `{name}`: {e}", provider.id))?;
			validator
				.validate(value)
				.map_err(|e| format!("`{}` violated schema `{name}`: {e}", provider.id))?;
		}
		Ok(())
	};
	let rows = |field: &str| answer[field].as_array().cloned().unwrap_or_default();
	let mut nodes = Vec::new();
	for row in rows("nodes") {
		let node: Asserted = serde_json::from_value(row.clone())
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
		validate(scheme, &provider.declared.schemes[scheme].schema, &row)?;
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
		for (name, value) in &node.attributes {
			validate(name, &provider.declared.attributes[name].schema, value)?;
		}
		nodes.push(node);
	}
	let mut edges = Vec::new();
	for row in rows("edges") {
		let edge: Linked = serde_json::from_value(row.clone())
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
		validate(
			&edge.kind,
			&provider.declared.edges[&edge.kind].schema,
			&row,
		)?;
		edges.push(edge);
	}
	Ok((nodes, edges))
}

/// A provider's answer, or why it contributes nothing: it could not be
/// reached, or it answered past its declaration.
pub(super) fn taken(
	provider: &Provider,
	answer: Result<Value>,
) -> Result<(Vec<Asserted>, Vec<Linked>), (Availability, String)> {
	let answer = answer.map_err(|e| (Availability::Unavailable, e.to_string()))?;
	admitted(provider, &answer).map_err(|e| (Availability::Refused, e))
}
