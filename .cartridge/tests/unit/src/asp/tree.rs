use super::*;

#[test]
fn roots_and_schemas_are_checked_before_a_cartridge_loads() {
	let declaration: crate::asp::protocol::Declaration = serde_json::from_value(json!({
		"roots":["missing:root"], "schemes":{"widget":{"owner":true}}
	}))
	.unwrap();
	assert!(declaration
		.problem("widgets", &["asp.widgets".into()])
		.unwrap()
		.contains("asp.roots"));
	let declaration: crate::asp::protocol::Declaration = serde_json::from_value(json!({
		"schemes":{"widget":{"schema":{"type":"not-a-json-type"}}}
	}))
	.unwrap();
	assert!(declaration
		.problem("widgets", &["asp.widgets".into()])
		.unwrap()
		.contains("schema"));
}

#[tokio::test]
async fn lua_extends_the_tree_and_records_event_activity() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"history",
		json!({
			"name":"history", "entry":"init.lua",
			"events":{"trace":{},"asp.history":{}}, "listen":["trace","asp.history"],
			"asp":{"roots":["trace:root"],"schemes":{"trace":{"owner":true}},
				"attributes":{"history.rows":{"schema":{"type":"array"}}}}
		}),
		r#"
        local rows = { {kind="boot"} }
        cartridge.listen("trace", function(request)
            rows[#rows + 1] = request.activity
            return {stored=true}
        end)
        cartridge.listen("asp.history", function(request)
            return {nodes={{id="trace:root", attributes={["history.rows"]=rows}}}}
        end)
    "#,
	);
	cartridge(
		dir.path(),
		"worker",
		json!({
			"name":"worker", "entry":"init.lua",
			"events":{"ping":{"schema":{"type":"object","required":["value"]}},"pulse":{},"asp.worker":{}},
			"listen":["ping","asp.worker"],
			"asp":{"roots":["widget:one"],"schemes":{"widget":{"owner":true}},
				"attributes":{"worker.count":{"schema":{"type":"integer"}}}}
		}),
		r#"
        local count = 0
        cartridge.listen("ping", function(request)
            count = count + 1
            cartridge.publish("pulse", {count=count})
            return request.value
        end)
        cartridge.listen("asp.worker", function(request)
            return {nodes={{id="widget:one", attributes={["worker.count"]=count}}}}
        end)
    "#,
	);
	let host = boot(dir.path(), &["history", "worker"]).await;
	let host_node = host
		.asp(json!({"op":"expand","entity":"cartridge:host","observe":false}))
		.await
		.unwrap();
	assert!(host_node["nodes"]
		.as_array()
		.unwrap()
		.iter()
		.any(|n| n["id"] == "cartridge:host"));
	let root = host
		.asp(json!({"op":"expand","entity":"asp:root","depth":2,"observe":false}))
		.await
		.unwrap();
	assert!(root["nodes"]
		.as_array()
		.unwrap()
		.iter()
		.any(|n| n["id"] == "widget:one"));
	assert!(root["nodes"]
		.as_array()
		.unwrap()
		.iter()
		.any(|n| n["id"] == "trace:root"));
	let event = host
		.asp(json!({"op":"expand","entity":"event:ping","observe":false}))
		.await
		.unwrap();
	assert!(event["nodes"]
		.as_array()
		.unwrap()
		.iter()
		.any(|n| n["attributes"]["host.event"]["schema"]["required"][0] == "value"));
	assert_eq!(
		host.bail("ping", json!({"value":42})).await.unwrap(),
		Some(json!(42))
	);
	let activity = host.asp(json!({"op":"activity"})).await.unwrap();
	assert!(activity["records"].as_array().unwrap().iter().any(|record| {
		record["operation"] == "dispatch" && record["entities"].as_array().unwrap().contains(&json!("event:ping"))
	}));
	descriptor(dir.path(), &["history"]);
	host.reconcile().await.unwrap();
	let types = host.asp(json!({"op":"types"})).await.unwrap();
	assert!(types["events"].get("ping").is_none());
	assert!(types["schemes"].get("widget").is_none());
	let root = host
		.asp(json!({"op":"expand","entity":"asp:root","depth":2,"observe":false}))
		.await
		.unwrap();
	assert!(!ids(&root).contains(&"widget:one"));
	assert!(ids(&root).contains(&"trace:root"));
	host.stop().await;
}

#[tokio::test]
async fn an_attribute_schema_refuses_a_lua_provider_with_the_wrong_value() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"bad",
		json!({
			"name":"bad","entry":"init.lua","events":{"asp.bad":{}},"listen":["asp.bad"],
			"asp":{"schemes":{"widget":{"owner":true}},"attributes":{"bad.count":{"schema":{"type":"integer"}}}}
		}),
		r#"cartridge.listen("asp.bad", function() return {nodes={{id="widget:one",attributes={["bad.count"]="wrong"}}}} end)"#,
	);
	let host = boot(dir.path(), &["bad"]).await;
	let reply = host
		.asp(json!({"op":"expand","entity":"widget:one"}))
		.await
		.unwrap();
	assert_eq!(reply["sources"][0]["state"], "refused");
	assert!(reply["sources"][0]["error"]
		.as_str()
		.unwrap()
		.contains("bad.count"));
	host.stop().await;
}
