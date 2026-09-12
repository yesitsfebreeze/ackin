use super::*;
use super::resolver::{alive, pids, ppid, try_pids, wait_for, zirkle_path};
use serde_json::json;

/// The document carries the cartridge's own configuration, so an author's
/// settings travel with the cartridge in the ledger shape, where no profile
/// `config.lua` lays fields on at composition time: a caller that names no
/// config inherits the document's, and a caller that names its own wins.
#[tokio::test(flavor = "multi_thread")]
async fn the_document_carries_the_cartridges_own_config() {
	let dir = tempfile::tempdir().unwrap();
	std::fs::create_dir_all(dir.path().join("p")).unwrap();
	write(
		dir.path(),
		"p/cartridge.json",
		&json!({"name": "p", "entry": "init.lua", "config": {"via": "document"}}).to_string(),
	);
	write(
		dir.path(),
		"p/init.lua",
		r#"return {apply=function(ctx, config) ctx:send("via", config) end}"#,
	);
	write(dir.path(), "init.lua", r#"return {{id="i", path="p"}}"#);
	let (host, mut rx) = boot(dir.path()).await;
	assert_eq!(next_event(&mut rx, "via").await, json!({"via": "document"}));

	// A caller that names its own config is laid over the document's.
	let component = host
		.component(&dir.path().join("p"), json!({"via": "caller"}))
		.unwrap();
	host.runtime().ctx().cartridge(component);
	assert_eq!(next_event(&mut rx, "via").await, json!({"via": "caller"}));
}

/// A chain of Lua cartridges is wired without touching Rust: the documents
/// declare what they provide and need, one ask brings the chain up from the
/// bottom, and the dependency's keys answer over the chain link. The tool's
/// handler calls its need (`echo`, one node down), a cartridge it composes
/// itself calls back up through the tool to the same dependency, and the
/// cascade still follows the dependency's death.
#[tokio::test(flavor = "multi_thread")]
async fn a_chain_node_calls_its_dependency_and_serves_its_dependents() {
	let dir = tempfile::tempdir().unwrap();
	let root = dir.path().to_path_buf();

	// The dependency: plain Lua, provides `echo`.
	std::fs::create_dir_all(root.join("echo")).unwrap();
	write(
		&root,
		"echo/cartridge.json",
		&json!({"name": "echo", "entry": "init.lua", "provide": ["echo"]}).to_string(),
	);
	write(
		&root,
		"echo/init.lua",
		r#"return {apply=function(ctx) ctx:provide("echo", function(args)
			return {said = args.word, from = "echo"}
		end) end}"#,
	);

	// A nested cartridge the tool composes itself: it injects the tool's own
	// key, so it starts once the tool is active, and provides a key the tool's
	// socket serves for it.
	std::fs::create_dir_all(root.join("tool/child")).unwrap();
	write(
		&root,
		"tool/child/cartridge.json",
		&json!({"name": "child", "entry": "init.lua", "provide": ["child.note"], "needs": ["tool.hello"]}).to_string(),
	);
	write(
		&root,
		"tool/child/init.lua",
		r#"return {apply=function(ctx)
		local hello = ctx:get("tool.hello")
		ctx:provide("child.note", function(args)
			local r = hello(args)
			r.via = "child"
			return r
		end)
	end}"#,
	);

	// The named tool: its document declares the need the ledger resolved into
	// `echo`, and the configuration the document carries.
	std::fs::create_dir_all(root.join("tool")).unwrap();
	write(
		&root,
		"tool/cartridge.json",
		&json!({"name": "tool", "entry": "init.lua", "provide": ["tool.hello"], "needs": ["echo"], "config": {"via": "document"}}).to_string(),
	);
	write(
		&root,
		"tool/init.lua",
		r#"return {apply=function(ctx, config)
		local echo = ctx:get("echo")
		ctx:cartridge("tool/child", {})
		ctx:provide("tool.hello", function(args)
			local r = echo({word = args.word})
			r.via = config.via
			return r
		end)
	end}"#,
	);

	let nodes_dir = root.join(".nodes");
	std::fs::create_dir_all(&nodes_dir).unwrap();

	let ask = std::process::Command::new(zirkle_path())
		.args(["up", "tool.hello", "--dir", &root.to_string_lossy()])
		.env("ZIRKLE_NODES", &nodes_dir)
		.output()
		.unwrap();
	assert!(ask.status.success(), "{}", String::from_utf8_lossy(&ask.stderr));
	let line = String::from_utf8_lossy(&ask.stdout)
		.lines()
		.rev()
		.find_map(|l| serde_json::from_str::<Value>(l).ok())
		.filter(|v| v.get("socket").is_some())
		.expect("the ask names the socket the tool answers at");
	assert_eq!(line["nodes"], json!(["echo", "tool"]));

	assert!(
		wait_for(
			|| try_pids(&root, &["echo", "tool"]).is_some_and(|p| p.len() == 2),
			10
		),
		"the hosted chain came up: {:?}",
		try_pids(&root, &["echo", "tool"])
	);
	let pids = pids(&root, &["echo", "tool"]);
	assert_eq!(ppid(pids[1]), pids[0], "the dependency launched its dependent");

	// The named tool's socket: the chain link answers through it.
	let socket = crate::socket::node_path(&root, "tool");
	let mut client = loop {
		match crate::socket::Client::connect(&socket).await {
			Ok(c) => break c,
			Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
		}
	};
	client
		.send(json!({"call": "tool.hello", "args": {"word": "hello"}, "id": 1}))
		.await
		.unwrap();
	let reply = loop {
		let m = tokio::time::timeout(Duration::from_secs(5), client.next())
			.await
			.unwrap()
			.expect("a reply from the tool");
		if m.get("reply").is_some() {
			break m;
		}
	};
	assert_eq!(
		reply["data"],
		json!({"said": "hello", "from": "echo", "via": "document"}),
		"the tool's handler called its need across the chain"
	);

	// The cartridge the tool composed itself serves through the same socket,
	// and its handler reaches back up through the tool to the dependency.
	let mut client = crate::socket::Client::connect(&socket).await.unwrap();
	client
		.send(json!({"call": "child.note", "args": {"word": "nested"}, "id": 1}))
		.await
		.unwrap();
	let reply = loop {
		let m = tokio::time::timeout(Duration::from_secs(5), client.next())
			.await
			.unwrap()
			.unwrap();
		if m.get("reply").is_some() {
			break m;
		}
	};
	assert_eq!(
		reply["data"],
		json!({"said": "nested", "from": "echo", "via": "child"}),
		"the composed cartridge called its parent, and the parent its dependency"
	);

	// A key that is neither the node's provision nor a need the chain bound
	// refuses at the socket, by the store's own refusal.
	let mut client = crate::socket::Client::connect(&socket).await.unwrap();
	client
		.send(json!({"call": "nowhere", "args": null, "id": 1}))
		.await
		.unwrap();
	let reply = loop {
		let m = tokio::time::timeout(Duration::from_secs(5), client.next())
			.await
			.unwrap()
			.unwrap();
		if m.get("reply").is_some() || m.get("error").is_some() {
			break m;
		}
	};
	assert_eq!(
		reply["reply"], json!(1),
		"a key nothing provides is refused by name: {reply}"
	);

	// The dependency's death still takes the node along: the socket carries
	// the calls, the stdin pipe carries the cascade.
	let (echo, tool) = (pids[0], pids[1]);
	std::process::Command::new("kill")
		.args(["-9", &echo.to_string()])
		.status()
		.unwrap();
	assert!(wait_for(|| !alive(tool), 10), "the dependent followed its dependency");
}