use super::{boot, settle, write};
use serde_json::{json, Value};

fn entry<'a>(snapshot: &'a Value, id: &str) -> &'a Value {
	snapshot["entries"]
		.as_array()
		.unwrap()
		.iter()
		.find(|row| row["id"] == id)
		.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_snapshot_distinguishes_provider_states_and_refreshes_committed_generations() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"good.lua",
		r#"return {provide={"answer"},apply=function(ctx) ctx:provide("answer",42) end}"#,
	);
	write(
		dir.path(),
		"consumer.lua",
		r#"return {inject={"answer"},apply=function() end}"#,
	);
	write(
		dir.path(),
		"waiting.lua",
		r#"return {inject={"absent"},apply=function() end}"#,
	);
	write(dir.path(), "bad.lua", "error('bad entry')");
	write(dir.path(), "disabled.lua", "error('must not evaluate')");
	write(
		dir.path(),
		"init.lua",
		r#"return {
		{id="good",path="good.lua",config={secret="never print me"}},
		{id="consumer",path="consumer.lua"},{id="waiting",path="waiting.lua"},
		{id="bad",path="bad.lua"},{id="missing",path="missing.lua"},
		{id="disabled",path="disabled.lua",disabled=true}}
	"#,
	);
	let (host, _) = boot(dir.path()).await;
	host.fiber_of("good").unwrap().settled().await;
	host.fiber_of("consumer").unwrap().settled().await;
	let first = host.snapshot();
	assert_eq!(first["profile"], json!(dir.path().canonicalize().unwrap()));
	assert!(!first.to_string().contains("never print me"));
	assert_eq!(entry(&first, "good")["state"], "Active");
	assert_eq!(
		entry(&first, "consumer")["dependencies"][0]["provider"],
		"good"
	);
	assert_eq!(entry(&first, "waiting")["state"], "Inactive");
	assert!(entry(&first, "waiting")["dependencies"][0]["provider"].is_null());
	assert_eq!(entry(&first, "bad")["state"], "Failed");
	assert!(entry(&first, "bad")["error"]
		.as_str()
		.unwrap()
		.contains("bad entry"));
	assert_eq!(entry(&first, "missing")["state"], "Missing");
	assert_eq!(entry(&first, "disabled")["state"], "Disabled");
	assert_eq!(entry(&first, "disabled")["declarations_available"], false);
	assert!(entry(&first, "disabled")["error"].is_null());

	write(dir.path(), "good.lua", "this is not valid lua");
	host.replace(&dir.path().join("good.lua")).await;
	let failed = host.snapshot();
	assert_eq!(
		entry(&failed, "good")["generation"],
		entry(&first, "good")["generation"]
	);
	assert_eq!(entry(&failed, "good")["state"], "Active");
	assert!(entry(&failed, "good")["error"].is_string());
	assert_eq!(entry(&failed, "good")["sources"][0]["changed"], true);
	write(
		dir.path(),
		"good.lua",
		r#"return {provide={"answer"},apply=function(ctx) ctx:provide("answer",12345) end}"#,
	);
	host.replace(&dir.path().join("good.lua")).await;
	host.fiber_of("good").unwrap().settled().await;
	let current = host.snapshot();
	assert_ne!(
		entry(&current, "good")["generation"],
		entry(&first, "good")["generation"]
	);
	assert!(entry(&current, "good")["error"].is_null());
	assert_eq!(entry(&current, "good")["sources"][0]["changed"], false);
	for id in ["good", "consumer", "waiting"] {
		host.fiber_of(id).unwrap().dispose().await;
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn the_snapshot_lists_dynamic_children_and_tracked_source_paths() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"child.lua",
		r#"return {provide={"answer"},apply=function(ctx) ctx:provide("answer",1) end}"#,
	);
	write(
		dir.path(),
		"parent.lua",
		r#"return {apply=function(ctx) ctx:cartridge("child.lua") end}"#,
	);
	write(
		dir.path(),
		"consumer.lua",
		r#"return {inject={"answer"},apply=function() end}"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="parent",path="parent.lua"},{id="consumer",path="consumer.lua"}}"#,
	);
	let run = |args: &[&str]| {
		let output = std::process::Command::new("git")
			.args(args)
			.current_dir(dir.path())
			.output()
			.unwrap();
		assert!(
			output.status.success(),
			"{}",
			String::from_utf8_lossy(&output.stderr)
		);
	};
	run(&["init"]);
	run(&["add", "."]);
	let (host, _) = boot(dir.path()).await;
	host.fiber_of("parent").unwrap().settled().await;
	settle().await;
	let snapshot = host.snapshot();
	let child = snapshot["entries"]
		.as_array()
		.unwrap()
		.iter()
		.find(|row| row["id"] == "child")
		.unwrap();
	assert_eq!(child["dynamic"], true);
	assert_eq!(child["parent"], "parent");
	assert_eq!(child["provide"], json!(["answer"]));
	assert_eq!(child["state"], "Active");
	assert_eq!(
		entry(&snapshot, "consumer")["dependencies"][0]["provider"],
		"child"
	);
	let consumer = entry(&snapshot, "consumer");
	assert_eq!(consumer["dependencies"][0]["key"], "answer");
	assert!(entry(&snapshot, "parent").get("tracked").is_none());
	host.fiber_of("parent").unwrap().dispose().await;
	host.fiber_of("consumer").unwrap().dispose().await;
}

fn node<'a>(graph: &'a Value, key: &str) -> &'a Value {
	graph["nodes"]
		.as_array()
		.unwrap()
		.iter()
		.find(|row| row["key"] == key)
		.unwrap_or_else(|| panic!("no `{key}` node in {graph}"))
}

fn keys(graph: &Value) -> Vec<&str> {
	graph["nodes"]
		.as_array()
		.unwrap()
		.iter()
		.filter_map(|row| row["key"].as_str())
		.collect()
}

fn has_edge(graph: &Value, from: &str, to: &str, kind: &str) -> bool {
	graph["edges"]
		.as_array()
		.unwrap()
		.iter()
		.any(|edge| edge["from"] == from && edge["to"] == to && edge["kind"] == kind)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_graph_is_what_the_composition_announces() {
	let dir = tempfile::tempdir().unwrap();
	// Announces one node and one edge, and tries to sign the node as somebody
	// else's contribution.
	write(
		dir.path(),
		"teller.lua",
		r#"return {provide={"answer"},apply=function(ctx)
			ctx:provide("answer",42)
			ctx:on("fabric.announce", function(announce)
				return {nodes={{kind="note",key="note:"..announce.scope.topic,name="told",
				                from="somebody else"}},
				        edges={{from="note:"..announce.scope.topic,to="answer",kind="links"}}}
			end)
		end}"#,
	);
	// Answers the announce with rows that are not rows: the graph drops them
	// rather than failing, and the cartridge is not counted as a contributor.
	write(
		dir.path(),
		"babbler.lua",
		r#"return {apply=function(ctx)
			ctx:on("fabric.announce", function()
				return {nodes={{kind="note",name="no key"},{key=""}},
				        edges={{from="a",kind="links"}}}
			end)
		end}"#,
	);
	// Never registered a listener at all.
	write(
		dir.path(),
		"quiet.lua",
		r#"return {inject={"answer"},apply=function() end}"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="teller",path="teller.lua"},{id="babbler",path="babbler.lua"},
		          {id="quiet",path="quiet.lua"}}"#,
	);
	let (host, _) = boot(dir.path()).await;
	for id in ["teller", "babbler", "quiet"] {
		host.fiber_of(id).unwrap().settled().await;
	}

	let graph = host.graph(json!({"topic": "birds"})).await;
	// What core knows first-hand: an entry is a node and its keys are edges,
	// with nobody's name on them.
	assert_eq!(node(&graph, "teller")["kind"], "cartridge");
	assert!(node(&graph, "teller")["from"].is_null());
	assert_eq!(node(&graph, "teller")["tags"], json!(["answer"]));
	assert!(has_edge(&graph, "teller", "answer", "provides"));
	assert!(has_edge(&graph, "quiet", "answer", "injects"));
	// What a cartridge says about itself, stamped with who actually said it.
	let told = node(&graph, "note:birds");
	assert_eq!(told["name"], "told");
	assert_eq!(
		told["from"], "teller",
		"provenance is the registry's name for the answering fiber, not the answer's claim"
	);
	assert!(has_edge(&graph, "note:birds", "answer", "links"));
	assert_eq!(graph["announced"], json!(["teller"]));
	assert!(
		!keys(&graph).iter().any(|key| key.is_empty()),
		"a malformed row is dropped, not folded"
	);
	assert!(!has_edge(&graph, "a", "", "links"));

	// The scope reaches the contributors: a different question, a different graph.
	let other = host.graph(json!({"topic": "trees"})).await;
	assert_eq!(node(&other, "note:trees")["from"], "teller");
	assert!(!keys(&other).contains(&"note:birds"));

	// Which cartridges are composed is what the graph is.
	host.fiber_of("teller").unwrap().dispose().await;
	settle().await;
	let without = host.graph(json!({"topic": "birds"})).await;
	assert!(!keys(&without).contains(&"note:birds"));
	assert_eq!(without["announced"], json!([]));
	host.fiber_of("babbler").unwrap().dispose().await;
	host.fiber_of("quiet").unwrap().dispose().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cartridge_announces_the_tools_it_provides() {
	let binary = super::built(&["-p", "cartridge", "--example", "tool_fixture"]);
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"tools.lua",
		&format!("return cartridge.process({})", json!(binary)),
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="tools",path="tools.lua"}}"#,
	);
	let (host, _) = boot(dir.path()).await;
	host.fiber_of("tools").unwrap().settled().await;

	let graph = host.graph(json!({"cwd": "/somewhere"})).await;
	// The cartridge registered no announce listener of its own: the SDK
	// answers with the tools it provides, as the tool describes itself.
	let tool = node(&graph, "tool.fixture");
	assert_eq!(tool["kind"], "tool");
	assert_eq!(tool["name"], "fixture");
	assert_eq!(tool["description"], "A fixture tool");
	assert_eq!(tool["from"], "tools");
	// The hook's own contribution arrives with it, scoped by the asking side.
	let hooked = node(&graph, "note:/somewhere");
	assert_eq!(hooked["description"], "contributed through the hook");
	assert_eq!(hooked["from"], "tools");
	assert!(has_edge(&graph, "note:/somewhere", "tool.fixture", "links"));
	assert!(has_edge(&graph, "tools", "tool.fixture", "provides"));
	assert_eq!(graph["announced"], json!(["tools"]));
	host.fiber_of("tools").unwrap().dispose().await;
}
