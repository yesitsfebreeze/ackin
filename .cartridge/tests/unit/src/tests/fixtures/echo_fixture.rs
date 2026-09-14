//! A process cartridge for the host tests. Provides `echo` and `whoami`, calls
//! `lua.name` for `relay`, listens to `ping`, publishes `ticks`.

use serde_json::{json, Value};

#[tokio::main]
async fn main() {
	transport::cartridge::run(|ctx, config: Value| async move {
		if config["fail"] == true {
			return Err("configured to fail".to_owned());
		}
		ctx.provide("echo", |args| async move { Ok(args) });
		let name = ctx.clone();
		ctx.provide("whoami", move |_| {
			let name = name.clone();
			async move { Ok(json!({ "name": name.name(), "needs": name.needs() })) }
		});
		let relay = ctx.clone();
		ctx.provide("relay", move |args| {
			let relay = relay.clone();
			async move { relay.call("lua.name", args).await }
		});
		let ticks = ctx.clone();
		ctx.on("ping", move |data| {
			let ticks = ticks.clone();
			async move {
				ticks.publish("ticks", data.clone());
				Ok(json!({ "pong": data }))
			}
		});
		Ok(())
	})
	.await;
}
