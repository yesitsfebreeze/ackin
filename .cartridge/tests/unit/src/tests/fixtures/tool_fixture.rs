//! An SDK child that provides a tool and contributes to the graph: the
//! announce regression's peer. It registers no listener for the announce
//! itself — every cartridge answers the tools it provides without asking —
//! and adds one node of its own through the hook, keyed by the scope it was
//! asked with, so a test can tell that the asking side's context travelled.
use cartridge::sdk::Cartridge;
use serde_json::json;

#[tokio::main]
async fn main() {
	Cartridge::new()
		.provide(&["tool.fixture"])
		.run(|host, _| async move {
			host.provide("tool.fixture", |args| async move {
				if args["op"] == "describe" {
					return Ok(json!({"name":"fixture","description":"A fixture tool"}));
				}
				Ok(json!({"content": "ran"}))
			});
			host.announce(|announce| async move {
				let cwd = announce["scope"]["cwd"].as_str().unwrap_or("nowhere");
				Ok(json!({
					"nodes": [{"kind":"note","key":format!("note:{cwd}"),"name":"scoped",
							   "description":"contributed through the hook",
							   "from":"a cartridge that is not this one"}],
					"edges": [{"from":format!("note:{cwd}"),"to":"tool.fixture","kind":"links"}],
				}))
			});
			Ok(())
		})
		.await;
}
