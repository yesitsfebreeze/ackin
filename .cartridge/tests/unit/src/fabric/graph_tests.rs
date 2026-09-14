use super::*;

fn node(kind: &str, key: &str, description: &str, when: &[&str]) -> Node {
	Node {
		kind: kind.into(),
		key: key.into(),
		description: description.into(),
		when: when.iter().map(|s| s.to_string()).collect(),
		name: String::new(),
		tags: Vec::new(),
		from: None,
	}
}

#[test]
fn search_returns_kinds_with_descriptions() {
	let nodes = vec![
		node(
			"tool",
			"tool.memo",
			"Read and write the workspace memo record",
			&["memos", "record"],
		),
		node(
			"routine",
			"routine/check-gates",
			"Run just check and just test before landing",
			&["gates"],
		),
		node(
			"memo",
			"note/tool-graph",
			"Notes about the ranking database",
			&["ranking"],
		),
	];
	let hits = search("memos record", &nodes, &BTreeMap::new());
	assert_eq!(hits.len(), 1);
	assert_eq!(hits[0].node.kind, "tool");
	assert_eq!(
		hits[0].node.description,
		"Read and write the workspace memo record"
	);
	let hits = search("gates", &nodes, &BTreeMap::new());
	assert_eq!(hits[0].node.kind, "routine");
}

#[test]
fn observed_use_outranks_equal_match() {
	let a = node("tool", "tool.a", "Format a file", &["format"]);
	let b = node("tool", "tool.b", "Format a file", &["format"]);
	let mut counts = BTreeMap::new();
	counts.insert("tool.b".to_owned(), 3.0);
	let hits = search("format", &[a.clone(), b.clone()], &counts);
	assert_eq!(hits[0].node.key, "tool.b");
	assert!(hits[0].uses > hits[1].uses);
	// Without observations the order is stable and deterministic by key.
	let hits = search("format", &[a, b], &BTreeMap::new());
	assert_eq!(hits[0].node.key, "tool.a");
}

#[test]
fn upsert_grows_the_node_set_in_place() {
	let mut graph = Graph::new();
	assert!(graph.is_empty());
	assert!(!graph.upsert(node("tool", "tool.memo", "Read the record", &[])));
	assert!(!graph.upsert(node("routine", "routine/check", "Run the gates", &[])));
	assert_eq!(graph.len(), 2);
	// A tool that re-registers with a new description updates instead of
	// doubling: the node set grows as things appear, not as they repeat.
	assert!(graph.upsert(node("tool", "tool.memo", "Read and write the record", &[])));
	assert_eq!(graph.len(), 2);
	assert_eq!(
		graph.node("tool.memo").unwrap().description,
		"Read and write the record"
	);
}

#[test]
fn edges_connect_present_nodes_and_tolerate_ones_to_come() {
	let mut graph = Graph::new();
	graph.upsert(node("memo", "work/graph", "The ranking database", &[]));
	graph.upsert(node("tool", "tool.memo", "Read and write the record", &[]));
	// The dispatch edge names a tool that has not been upserted yet.
	graph.link(Edge {
		from: "work/graph".into(),
		to: "tool.graph".into(),
		kind: "dispatch".into(),
	});
	graph.link(Edge {
		from: "work/graph".into(),
		to: "tool.memo".into(),
		kind: "dispatch".into(),
	});
	// A repeated edge is one edge.
	graph.link(Edge {
		from: "work/graph".into(),
		to: "tool.memo".into(),
		kind: "dispatch".into(),
	});
	assert_eq!(graph.edges().count(), 2);
	assert_eq!(graph.edges_of("tool.memo").len(), 1);
	let neighbours: Vec<_> = graph
		.neighbours("work/graph")
		.iter()
		.map(|n| n.key.as_str())
		.collect();
	assert_eq!(neighbours, vec!["tool.memo"]);
	// Once the named node appears, the waiting edge connects it.
	graph.upsert(node("tool", "tool.graph", "Query the tool graph", &[]));
	assert_eq!(graph.neighbours("work/graph").len(), 2);
}

#[test]
fn graph_search_keeps_the_seed_ranking() {
	let journal = concat!(
		r#"{"version":1,"dropped":0}"#,
		"\n",
		r#"{"id":"1","at_ms":1,"origin":{"session":"s","run":"r","call":"c"},"observation":{"stage":"used","path":"tool.b","usage":"run","revision":"0"}}"#,
		"\n",
	);
	let counts = counts_from_journal(journal);
	let mut graph = Graph::new();
	graph.upsert(node("tool", "tool.a", "Format a file", &["format"]));
	graph.upsert(node("tool", "tool.b", "Format a file", &["format"]));
	let hits = graph.search("format", &counts);
	assert_eq!(hits[0].node.key, "tool.b");
	assert!(hits[0].uses > 0);
	// The standing comes from the journal, so an empty journal is the
	// deterministic by-key order.
	assert_eq!(
		graph.search("format", &BTreeMap::new())[0].node.key,
		"tool.a"
	);
}

#[test]
fn journal_counts_weight_stages() {
	let journal = concat!(
		r#"{"version":1,"dropped":0}"#,
		"\n",
		r#"{"id":"1","at_ms":1,"origin":{"session":"s","run":"r","call":"c"},"observation":{"stage":"used","path":"tool.a","usage":"run","revision":"0"}}"#,
		"\n",
		r#"{"id":"2","at_ms":2,"origin":{"session":"s","run":"r","call":"c"},"observation":{"stage":"outcome","path":"tool.a","usage":"run","revision":"0","parent":"1","outcome":"success"}}"#,
		"\n",
		r#"{"id":"3","at_ms":3,"origin":{"session":"s","run":"r","call":"c"},"observation":{"stage":"surfaced","path":"tool.b","usage":"run","revision":"0"}}"#,
		"\n",
	);
	let counts = counts_from_journal(journal);
	assert_eq!(counts["tool.a"], 2.0);
	assert_eq!(counts.get("tool.b"), Some(&0.0));
}
