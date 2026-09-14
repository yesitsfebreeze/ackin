//! The same public replacement operation at the profile and nested depths.
use super::{boot, write};
use crate::runtime::{State, Uid};
use serde_json::{json, Value};
use std::time::Duration;

fn provider(version: u64, fail: bool) -> String {
	format!(
		r#"return {{inject={{"router","pty"}},provide={{"counter"}},apply=function(ctx,config)
		ctx:provide("counter",function() return {{version={version},label=config.label,
			router=ctx.router(),pty=ctx.pty(),router_meta=ctx:meta("router")}} end)
		{failure}
	end}}"#,
		failure = if fail {
			"error('candidate rejected')"
		} else {
			""
		}
	)
}

const SHARED: &str = r#"return {provide={"router","pty"},apply=function(ctx)
	ctx:provide("router",function() return "shared-router" end)
	ctx:provide("pty",function() return "shared-pty" end)
	ctx:send("shared","applied")
end}"#;

async fn active(host: &std::sync::Arc<super::Host>, uid: Uid) {
	let fiber = host.runtime().handle(uid);
	tokio::time::timeout(Duration::from_secs(5), fiber.settled())
		.await
		.expect("generation did not settle");
	assert_eq!(fiber.state(), Some(State::Active), "{:?}", fiber.error());
}

#[tokio::test(flavor = "multi_thread")]
async fn replacement_has_one_contract_at_root_and_nested_depth() {
	for depth in [0, 1] {
		let dir = tempfile::tempdir().unwrap();
		write(dir.path(), "provider.lua", &provider(1, false));
		write(dir.path(), "shared.lua", SHARED);
		write(
			dir.path(),
			"consumer.lua",
			r#"return {inject={"counter"},provide={"entry"},apply=function(ctx)
			local counter=ctx.counter
			ctx:provide("entry",function() return counter() end)
		end}"#,
		);
		write(
			dir.path(),
			"outer.lua",
			r#"return {provide={"entry","child"},apply=function(ctx)
			local inner=ctx:isolate("counter"):isolate("entry"):intercept("router",{scope="nested"})
			local p=inner:cartridge("provider.lua",{label="configured"})
			local c=inner:cartridge("consumer.lua")
			ctx:provide("entry",function() return inner:peek("entry")() end)
			ctx:provide("child",function() return p:uid() end)
		end}"#,
		);
		write(
			dir.path(),
			"init.lua",
			if depth == 0 {
				r#"return {{id="shared",path="shared.lua"},{id="provider",path="provider.lua",config={label="configured"}},{id="consumer",path="consumer.lua"}}"#
			} else {
				r#"return {{id="shared",path="shared.lua"},{id="outer",path="outer.lua"}}"#
			},
		);
		let (host, mut events) = boot(dir.path()).await;
		let dependent_name = if depth == 0 { "consumer" } else { "outer" };
		let dependent = host.fiber_of(dependent_name).unwrap().uid();
		active(&host, dependent).await;
		let mut current = if depth == 0 {
			host.fiber_of("provider").unwrap().uid()
		} else {
			host.call("child", Value::Null)
				.await
				.unwrap()
				.as_u64()
				.unwrap()
		};
		active(&host, current).await;
		let shared = host.fiber_of("shared").unwrap().uid();
		let mut expected = json!({"version":1,"label":"configured","router":"shared-router","pty":"shared-pty","router_meta":null});
		if depth == 1 {
			expected["router_meta"] = json!({"scope":"nested"});
		}
		// Consumer readiness is independent of its provider's readiness.
		let first = tokio::time::timeout(Duration::from_secs(5), async {
			loop {
				if let Ok(value) = host.call("entry", Value::Null).await {
					break value;
				}
				tokio::task::yield_now().await;
			}
		})
		.await
		.unwrap();
		assert_eq!(first, expected, "depth={depth}");
		for version in [2, 3] {
			write(dir.path(), "provider.lua", &provider(version, false));
			let next = host.replace_node(current).await.unwrap();
			assert_ne!(
				next, current,
				"replacement returned retired UID at depth={depth}"
			);
			assert!(host.runtime().handle(current).state().is_none());
			active(&host, next).await;
			current = next;
			expected["version"] = json!(version);
			assert_eq!(host.call("entry", Value::Null).await.unwrap(), expected);
			if depth == 0 {
				assert_eq!(host.fiber_of("provider").unwrap().uid(), current);
			}
		}
		for bad in [
			provider(4, true),
			provider(4, false).replace("\"counter\"", "\"wrong-counter\""),
		] {
			write(dir.path(), "provider.lua", &bad);
			assert!(
				host.replace_node(current).await.is_err(),
				"bad candidate acknowledged at depth={depth}"
			);
			active(&host, current).await;
			assert_eq!(host.call("entry", Value::Null).await.unwrap(), expected);
		}
		write(dir.path(), "provider.lua", &provider(5, false));
		current = host.replace_node(current).await.unwrap();
		active(&host, current).await;
		expected["version"] = json!(5);
		assert_eq!(host.call("entry", Value::Null).await.unwrap(), expected);
		assert_eq!(host.fiber_of(dependent_name).unwrap().uid(), dependent);
		assert_eq!(host.fiber_of("shared").unwrap().uid(), shared);
		if depth == 1 {
			assert!(host.call("counter", Value::Null).await.is_err());
		} else {
			// The root adapter published current source stamps with its handle.
			host.replace(&dir.path().join("provider.lua")).await;
			assert_eq!(host.fiber_of("provider").unwrap().uid(), current);
		}
		let mut applications = 0;
		while let Ok(event) = events.try_recv() {
			if event["event"] == "shared" {
				applications += 1;
			}
		}
		assert_eq!(
			applications, 1,
			"shared providers were duplicated at depth={depth}"
		);
		for name in if depth == 0 {
			vec!["consumer", "provider", "shared"]
		} else {
			vec!["outer", "shared"]
		} {
			host.fiber_of(name).unwrap().dispose().await;
		}
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn failed_initial_profile_entry_recovers_with_its_current_handle() {
	let dir = tempfile::tempdir().unwrap();
	write(dir.path(), "shared.lua", SHARED);
	write(dir.path(), "provider.lua", &provider(1, true));
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="shared",path="shared.lua"},{id="provider",path="provider.lua",config={label="recovered"}}}"#,
	);
	let (host, _) = boot(dir.path()).await;
	let failed = host.fiber_of("provider").unwrap();
	// Settled is not failed: the entry is inactive until `shared` provides
	// what it injects, and only then does its apply run and refuse.
	tokio::time::timeout(Duration::from_secs(5), async {
		while failed.error().is_none() {
			tokio::time::sleep(Duration::from_millis(20)).await;
			failed.settled().await;
		}
	})
	.await
	.expect("the candidate never failed");
	write(dir.path(), "provider.lua", &provider(2, false));
	let recovered = host.replace_node(failed.uid()).await.unwrap();
	assert_ne!(recovered, failed.uid());
	assert_eq!(host.fiber_of("provider").unwrap().uid(), recovered);
	active(&host, recovered).await;
	assert_eq!(
		host.call("counter", Value::Null).await.unwrap()["label"],
		"recovered"
	);
	for name in ["provider", "shared"] {
		host.fiber_of(name).unwrap().dispose().await;
	}
}
