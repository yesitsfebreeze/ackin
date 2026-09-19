//! ASP (Agent Server Protocol) is the fabric with a better protocol. The fabric
//! was one graph of everything the composition can reach, ranked by match and
//! observed use. ASP is that same graph, and it adds what the fabric's protocol
//! lacked: a typed identity for every node (`scheme:key`), a registry of the
//! types each cartridge declares in its `cartridge.json`, a contributor and a
//! revision on every fact, staleness against the scheme owner's revision, and
//! actions an agent can run on an entity.
//!
//! This file is the wire: what a cartridge declares, what a provider answers
//! and what ASP gives back.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The `asp` block of a `cartridge.json`. A cartridge that declares one also
/// listens to exactly one `asp.<name>` event, where ASP sends its requests.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
	/// The schemes this cartridge asserts nodes of and is asked to expand.
	#[serde(default)]
	pub schemes: BTreeMap<String, Scheme>,
	#[serde(default)]
	pub edges: BTreeMap<String, Described>,
	/// Attribute names, each prefixed with the cartridge's own name.
	#[serde(default)]
	pub attributes: BTreeMap<String, Described>,
	#[serde(default)]
	pub actions: Vec<Action>,
	/// Whether the provider answers `{op: "search"}`.
	#[serde(default)]
	pub search: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scheme {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	/// The owner defines the scheme's canonical key, and its revision of an
	/// entity is the one every other contributor's facts are held against.
	#[serde(default)]
	pub owner: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Described {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
	Read,
	Mutate,
}

/// A pointer to a tool event the host already serves. `args` is the tool's
/// payload, where the strings `${entity}` and `${key}` stand for the entity.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
	pub name: String,
	pub applies_to: String,
	pub effect: Effect,
	pub tool: String,
	#[serde(default)]
	pub args: Value,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
}

impl Declaration {
	pub fn is_empty(&self) -> bool {
		*self == Self::default()
	}

	/// The reason this declaration cannot load for cartridge `name`.
	pub fn problem(&self, name: &str, listen: &[String]) -> Option<String> {
		if self.is_empty() {
			return None;
		}
		let doors = listen.iter().filter(|k| k.starts_with("asp.")).count();
		if doors != 1 {
			return Some(format!(
				"`asp` needs exactly one `asp.<name>` event in `listen`, found {doors}"
			));
		}
		let bare = |word: &str| {
			!word.is_empty()
				&& word
					.chars()
					.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
		};
		if let Some(scheme) = self.schemes.keys().find(|s| !bare(s)) {
			return Some(format!(
				"`asp.schemes.{scheme}` must be lowercase letters, digits and `-`"
			));
		}
		if let Some(kind) = self.edges.keys().find(|k| !bare(k)) {
			return Some(format!(
				"`asp.edges.{kind}` must be lowercase letters, digits and `-`"
			));
		}
		let prefix = format!("{name}.");
		if let Some(attribute) = self.attributes.keys().find(|a| !a.starts_with(&prefix)) {
			return Some(format!(
				"`asp.attributes.{attribute}` must start with `{prefix}`"
			));
		}
		if let Some(action) = self
			.actions
			.iter()
			.find(|a| a.name.trim().is_empty() || !bare(&a.applies_to) || a.tool.trim().is_empty())
		{
			return Some(format!(
				"`asp.actions` entry `{}` needs a name, a scheme in `applies_to` and a tool",
				action.name
			));
		}
		None
	}
}

/// A node as a provider asserts it. The descriptive fields are the fabric's:
/// they are what search matches and ranks.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Asserted {
	pub id: String,
	#[serde(default)]
	pub revision: Option<String>,
	#[serde(default)]
	pub name: String,
	#[serde(default)]
	pub description: String,
	#[serde(default)]
	pub when: Vec<String>,
	#[serde(default)]
	pub tags: Vec<String>,
	#[serde(default)]
	pub attributes: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Linked {
	pub from: String,
	pub to: String,
	pub kind: String,
	#[serde(default)]
	pub revision: Option<String>,
}

/// Who asserted a fact, against which revision, and whether the scheme
/// owner's revision has moved on since.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Assertion {
	pub contributor: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub revision: Option<String>,
	#[serde(skip_serializing_if = "std::ops::Not::not")]
	pub stale: bool,
}

/// A node of the world: the union of what every contributor asserted.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Node {
	pub id: String,
	#[serde(skip_serializing_if = "String::is_empty")]
	pub name: String,
	#[serde(skip_serializing_if = "String::is_empty")]
	pub description: String,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub when: Vec<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub tags: Vec<String>,
	#[serde(skip_serializing_if = "BTreeMap::is_empty")]
	pub attributes: BTreeMap<String, Value>,
	pub contributors: Vec<Assertion>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Edge {
	pub from: String,
	pub to: String,
	pub kind: String,
	#[serde(flatten)]
	pub assertion: Assertion,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Availability {
	Available,
	Unavailable,
	/// The provider answered with a fact its declaration does not cover, so
	/// its whole answer was refused.
	Refused,
}

#[derive(Clone, Debug, Serialize)]
pub struct Source {
	pub contributor: String,
	pub state: Availability,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
}

/// An action bound to one entity, ready to run.
#[derive(Clone, Debug, Serialize)]
pub struct Bound {
	pub name: String,
	pub contributor: String,
	pub effect: Effect,
	pub tool: String,
	pub args: Value,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
}

/// The scheme of an entity id, when the id is `scheme:key` with both parts.
pub fn scheme_of(id: &str) -> Option<&str> {
	let (scheme, key) = id.split_once(':')?;
	(!scheme.is_empty() && !key.is_empty()).then_some(scheme)
}
