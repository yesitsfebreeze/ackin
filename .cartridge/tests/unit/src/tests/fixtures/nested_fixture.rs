//! Offline SDK sub-host used by the wire recursion regression tests: a
//! cartridge that hosts the cartridge `config.child` names over the same wire
//! it is itself hosted on.
use cartridge::sdk::Cartridge;
use serde_json::json;

#[tokio::main]
async fn main() {
	Cartridge::new()
		.inject(&["lua"])
		.provide(&["roundtrip"])
		.run(|host, config| async move {
			let child: Vec<String> = config["child"]
				.as_array()
				.expect("config.child")
				.iter()
				.map(|v| v.as_str().expect("child cmd entry").to_owned())
				.collect();
			let lifecycle = config["lifecycle"] == true;
			host.spawn("nested", &child, json!({ "lifecycle": lifecycle }))
				.await?;
			Ok(())
		})
		.await;
}
