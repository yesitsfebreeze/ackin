//! Offline SDK child that provides `lua`, used as an isolated alternate
//! provider in the composition regression tests.
use serde_json::json;
use zirkle::sdk::Cartridge;

#[tokio::main]
async fn main() {
	Cartridge::new()
		.provide(&["lua"])
		.run(|host, _config| async move {
			host.provide("lua", |args| async move { Ok(json!({"value": args})) });
			Ok(())
		})
		.await;
}
