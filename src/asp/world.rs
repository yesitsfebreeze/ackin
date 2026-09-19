//! The merge: one world out of what every contributor asserted.

use std::collections::BTreeMap;

use super::protocol::{Asserted, Assertion, Availability, Edge, Linked, Node, Source};

#[derive(Default)]
pub(super) struct World {
	pub(super) nodes: BTreeMap<String, Node>,
	pub(super) edges: Vec<Edge>,
	pub(super) sources: BTreeMap<String, Source>,
}

impl World {
	pub(super) fn source(&mut self, contributor: &str, state: Availability, error: Option<String>) {
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

	pub(super) fn assert(&mut self, owns: bool, asserted: Asserted, assertion: Assertion) {
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

	pub(super) fn link(&mut self, linked: Linked, assertion: Assertion) {
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
