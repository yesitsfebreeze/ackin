//! Offline SDK child used by the process transport regression tests.
use serde_json::json;
use zirkle::sdk::Cartridge;

#[tokio::main]
async fn main() {
	Cartridge::new()
		.inject(&["lua"])
		.provide(&["roundtrip"])
		.run(|host, config| async move {
			if config["reject"] == true {
				return Err("rejected fixture migration".into());
			}
			if config["lifecycle"] == true {
				let observer = host.clone();
				host.on("probe", move |data| {
					observer.send("observed", data);
					async { Ok(json!(null)) }
				});
				let observer = host.clone();
				host.on_dispose(move || async move {
					observer.send("disposed", json!(true));
				});
			}
			if config["shutdown"] == true {
				let caller = host.clone();
				let pending = tokio::spawn(async move { caller.call("lua", json!("pending")).await });
				let finalizer = host.clone();
				host.on_dispose(move || async move {
					let pending = pending.await.unwrap().unwrap_err();
					let late = finalizer.call("lua", json!("late")).await.unwrap_err();
					finalizer.send("finished", json!({"pending": pending, "late": late}));
				});
			}
			let service_host = host.clone();
			let service_config = config.clone();
			host.provide("roundtrip", move |args| {
				let host = service_host.clone();
				let config = service_config.clone();
				async move {
					// Telemetry mode: one call, one published event on a channel.
					if let Some(channel) = args.get("publish").and_then(|v| v.as_str()) {
						host.publish(channel, args.get("data").cloned().unwrap_or(json!(null)));
						return Ok(json!(true));
					}
					if args == "config" {
						return Ok(config);
					}
					if args == "pending" {
						host.send("pending", json!(true));
						return std::future::pending().await;
					}
					if args == "exit" {
						std::process::exit(0);
					}
					// The turn this cartridge sees, and the one a Lua service sees when
					// this cartridge calls on into the host inside the same turn.
					if args == "turn" {
						let lua = host.call("lua", json!("turn")).await?;
						return Ok(json!({"cartridge": zirkle::sdk::Host::turn(), "lua": lua}));
					}
					let key = args
						.get("key")
						.and_then(|v| v.as_str())
						.unwrap_or("lua")
						.to_owned();
					let data = host.call(&key, args).await?;
					let meta = host.meta(&key).await?;
					let absent = host.meta("absent").await?;
					Ok(json!({"data": data, "meta": meta, "absent": absent}))
				}
			});
			if config["apply_call"] == true {
				host.send("apply", host.call("lua", json!(7)).await?);
			}
			if config["telemetry"] == true {
				// Watches `build` and echoes every envelope it receives onto the
				// socket, so a test reads the channel and the watcher through one
				// client: sequence order, publisher, and join/leave all in one stream.
				let watcher = host.clone();
				host.subscribe("build", move |envelope| {
					let watcher = watcher.clone();
					async move {
						watcher.send("watched", envelope);
						Ok(json!(null))
					}
				});
			}
			Ok(())
		})
		.await;
}
