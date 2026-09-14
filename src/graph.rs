//! The fabric's graph: every reachable thing the composition announced, and the
//! one search over it. Nodes and edges are derived state — the record, the
//! tools and the observation journal stay the sources of truth — so the graph
//! is rebuilt from each announce and keeps no durable copy. What is durable is
//! the use it ranks by, and that lives in the memo resolver's observation
//! journal, whose bounded retained window is also the standing decay.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// One reachable thing. `kind` is `tool`, `memo` or `routine`; `key` is the
/// stable reference a caller uses to reach it (tool id, memo path).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
	pub kind: String,
	pub key: String,
	#[serde(default)]
	pub description: String,
	#[serde(default)]
	pub when: Vec<String>,
	#[serde(default)]
	pub name: String,
	#[serde(default)]
	pub tags: Vec<String>,
	/// The cartridge that announced this node, as core named it — absent for
	/// the rows core states itself from the composition snapshot. Stamped by
	/// the host, never by the contributor.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub from: Option<String>,
}

/// A ranked entry: the node, why it ranked, and how to reach it.
#[derive(Clone, Serialize)]
pub struct Hit {
	pub node: Node,
	pub score: f64,
	pub uses: usize,
	pub why: String,
}

/// A connection between two nodes, named by their keys: a memo's wiki links,
/// the tool a dispatch ran, the usage a routine serves.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
	pub from: String,
	pub to: String,
	pub kind: String,
}

/// The engine's graph: the node set grown by `upsert` as tools, memos and
/// routines appear, and the edges between them. Nodes and edges are derived
/// state — the record, the tool inventories and the observation journal stay
/// the sources of truth, so the graph is rebuilt per composition and keeps no
/// durable copy of them. What is durable is the use the graph counts, and that
/// lives in the resolver's observation journal (`counts_from_journal`), whose
/// bounded retained window is also the standing decay: observations that fall
/// out of the window stop counting.
#[derive(Default)]
pub struct Graph {
	nodes: BTreeMap<String, Node>,
	edges: BTreeSet<Edge>,
}

impl Graph {
	pub fn new() -> Self {
		Self::default()
	}

	/// Grow the node set. A key already present is replaced in place, so a
	/// tool whose description changed or a memo that was edited updates
	/// instead of doubling. Returns whether it replaced an existing node.
	pub fn upsert(&mut self, node: Node) -> bool {
		self.nodes.insert(node.key.clone(), node).is_some()
	}

	/// Connect two nodes. A dispatch may name a node that has not been
	/// upserted yet, so endpoints are not checked here; `neighbours` only
	/// ever reports nodes that are present. A repeated edge is one edge.
	pub fn link(&mut self, edge: Edge) {
		self.edges.insert(edge);
	}

	pub fn node(&self, key: &str) -> Option<&Node> {
		self.nodes.get(key)
	}

	pub fn len(&self) -> usize {
		self.nodes.len()
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.nodes.is_empty()
	}

	/// Stable node rows for read-only fabric clients.
	pub fn nodes(&self) -> impl Iterator<Item = &Node> {
		self.nodes.values()
	}

	pub fn edges(&self) -> impl Iterator<Item = &Edge> {
		self.edges.iter()
	}

	/// The edges touching `key`, in either direction.
	pub fn edges_of(&self, key: &str) -> Vec<&Edge> {
		self.edges
			.iter()
			.filter(|e| e.from == key || e.to == key)
			.collect()
	}

	/// The present nodes at the far end of `key`'s edges. A node linked only
	/// from edges whose other end is absent never appears here.
	pub fn neighbours(&self, key: &str) -> Vec<&Node> {
		self.edges_of(key)
			.into_iter()
			.filter_map(|e| {
				let other = if e.from == key { &e.to } else { &e.from };
				self.nodes.get(other)
			})
			.collect()
	}

	/// The one search, over the grown node set: match times observed standing.
	pub fn search(&self, query: &str, counts: &BTreeMap<String, f64>) -> Vec<Hit> {
		let nodes: Vec<Node> = self.nodes.values().cloned().collect();
		search(query, &nodes, counts)
	}
}

const STOP: &[&str] = &[
	"a", "an", "the", "to", "of", "for", "in", "on", "is", "are", "and", "or", "how", "what", "i",
	"we", "do", "best", "way",
];

pub fn normalized(text: &str) -> String {
	text.to_lowercase()
		.split(|c: char| !c.is_alphanumeric())
		.filter(|s| !s.is_empty())
		.collect::<Vec<_>>()
		.join(" ")
}

pub fn tokens(text: &str) -> Vec<String> {
	normalized(text)
		.split_whitespace()
		.filter(|s| !STOP.contains(s))
		.map(str::to_owned)
		.collect()
}

/// Observation stages weight differently: a use is the act the ranking exists
/// for, a successful outcome confirms it, a surface alone counts least.
pub fn stage_weight(stage: &str, outcome: Option<&str>) -> f64 {
	match (stage, outcome) {
		("used", _) => 1.0,
		("outcome", Some("success")) => 1.0,
		("outcome", Some("failure")) => 0.25,
		("outcome", _) => 0.5,
		("surfaced", _) => 0.0,
		_ => 0.0,
	}
}

/// Use counts from an observation journal's JSON lines (header line skipped).
/// Each event nests its fields under `observation`.
pub fn counts_from_journal(journal: &str) -> BTreeMap<String, f64> {
	let mut counts: BTreeMap<String, f64> = BTreeMap::new();
	for line in journal.lines().skip(1) {
		let Ok(event) = serde_json::from_str::<Value>(line) else {
			continue;
		};
		let Some(path) = event["observation"]["path"].as_str() else {
			continue;
		};
		let weight = stage_weight(
			event["observation"]["stage"].as_str().unwrap_or(""),
			event["observation"]["outcome"].as_str(),
		);
		*counts.entry(path.to_owned()).or_default() += weight;
	}
	counts
}

/// Rank `nodes` for `query`: match score times a standing raised by observed
/// use. An equally matched but never-used entry loses to one the record shows
/// being used.
pub fn search(query: &str, nodes: &[Node], counts: &BTreeMap<String, f64>) -> Vec<Hit> {
	let terms: std::collections::BTreeSet<String> = tokens(query).into_iter().collect();
	if terms.is_empty() {
		return Vec::new();
	}
	let mut hits = Vec::new();
	for node in nodes {
		let fields = [
			("name", node.name.clone(), 3.0),
			("when", node.when.join(" "), 3.0),
			("tags", node.tags.join(" "), 2.0),
			("description", node.description.clone(), 1.0),
		];
		let mut score = 0.0;
		let mut matched = Vec::new();
		for (field, text, weight) in fields {
			let field_tokens: std::collections::BTreeSet<String> =
				tokens(&text).into_iter().collect();
			let found: Vec<_> = terms.intersection(&field_tokens).cloned().collect();
			if !found.is_empty() {
				score += found.len() as f64 * weight;
				matched.push(format!("{field}: {}", found.join(", ")));
			}
		}
		if score == 0.0 {
			continue;
		}
		let uses = counts.get(&node.key).copied().unwrap_or(0.0);
		let standing = 1.0 + uses.ln_1p();
		hits.push(Hit {
			node: node.clone(),
			score: score * standing,
			uses: uses as usize,
			why: if matched.is_empty() {
				"candidate".into()
			} else {
				matched.join("; ")
			},
		});
	}
	hits.sort_by(|a, b| {
		b.score
			.total_cmp(&a.score)
			.then_with(|| a.node.key.cmp(&b.node.key))
	});
	hits
}

/// The announced graph, read back: every node and edge core gathered, and the
/// uses the journal counts. A row that does not deserialize is dropped rather
/// than failing the graph — the contributor that malformed it is the party at
/// fault, and the rest of the composition still answered.
pub fn grown(announced: &Value, journal: &str) -> (Graph, BTreeMap<String, f64>) {
	let mut graph = Graph::new();
	for node in announced["nodes"].as_array().into_iter().flatten() {
		if let Ok(node) = serde_json::from_value::<Node>(node.clone()) {
			graph.upsert(node);
		}
	}
	for edge in announced["edges"].as_array().into_iter().flatten() {
		if let Ok(edge) = serde_json::from_value::<Edge>(edge.clone()) {
			graph.link(edge);
		}
	}
	(graph, counts_from_journal(journal))
}

#[cfg(test)]
#[path = "../.cartridge/tests/unit/src/fabric/graph_tests.rs"]
mod tests;
