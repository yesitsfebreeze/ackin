//! The fabric's ranking, over ASP nodes: how well a node's words match the
//! query, times a standing raised by observed use.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::protocol::Node;

const STOP: &[&str] = &[
	"a", "an", "the", "to", "of", "for", "in", "on", "is", "are", "and", "or", "how", "what", "i",
	"we", "do", "best", "way",
];

fn tokens(text: &str) -> BTreeSet<String> {
	text.to_lowercase()
		.split(|c: char| !c.is_alphanumeric())
		.filter(|word| !word.is_empty() && !STOP.contains(word))
		.map(str::to_owned)
		.collect()
}

#[derive(Clone, Debug, Serialize)]
pub struct Hit {
	pub node: Node,
	pub score: f64,
	pub why: String,
}

/// Every node, best first. A node none of whose words match keeps the order
/// its provider gave it, below every node that matched: the provider found it
/// for this query by means the words do not show, such as file content.
pub fn rank(query: &str, nodes: Vec<Node>, uses: &BTreeMap<String, f64>) -> Vec<Hit> {
	let terms = tokens(query);
	let mut hits: Vec<Hit> = nodes
		.into_iter()
		.map(|node| {
			let fields = [
				("name", format!("{} {}", node.name, node.id), 3.0),
				("when", node.when.join(" "), 3.0),
				("tags", node.tags.join(" "), 2.0),
				("description", node.description.clone(), 1.0),
			];
			let mut score = 0.0;
			let mut matched = Vec::new();
			for (field, text, weight) in fields {
				let found: Vec<String> = terms.intersection(&tokens(&text)).cloned().collect();
				if !found.is_empty() {
					score += found.len() as f64 * weight;
					matched.push(format!("{field}: {}", found.join(", ")));
				}
			}
			let standing = 1.0 + uses.get(&node.id).copied().unwrap_or(0.0).ln_1p();
			let why = match matched.is_empty() {
				true => "provider match".to_owned(),
				false => matched.join("; "),
			};
			Hit {
				node,
				score: score * standing,
				why,
			}
		})
		.collect();
	// Stable, so equal scores keep the order the providers answered in.
	hits.sort_by(|a, b| b.score.total_cmp(&a.score));
	hits
}
