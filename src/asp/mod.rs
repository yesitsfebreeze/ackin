//! The host's `asp` service: one world, composed on request from every loaded
//! cartridge that declares an `asp` block. ASP keeps no store. It asks the
//! live providers each time, so a cartridge that leaves the composition takes
//! its types and its facts with it, and nothing has to be retracted.

pub(crate) mod activity;
mod admit;
mod ask;
mod expand;
pub(crate) mod own;
pub mod protocol;
pub mod rank;
mod registry;
mod render;
mod search;
mod tool;
mod tree;
mod world;

use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::host::Host;

pub(crate) use own::own_types;
use protocol::scheme_of;
use registry::Registry;

/// The key the host answers ASP on: `cartridge call asp`, and the `asp` host
/// method a cartridge reaches through `cartridge.host`.
pub const SERVICE: &str = "asp";

/// The event the base owns and listens to, so ASP is one of the agent's tools.
pub const TOOL: &str = "tool.asp";

/// The id the base contributes under.
const HOST: &str = "host";

const DEPTH: u64 = 1;
const MAX_DEPTH: u64 = 4;
const LIMIT: usize = 256;

impl Host {
	/// The `asp` service: `{op: types | expand | search | actions | act | activity}`.
	/// `format: "text"` answers compact lines instead of JSON.
	pub async fn asp(&self, request: Value) -> Result<Value> {
		let op = request["op"].as_str().unwrap_or_default().to_owned();
		let text = request["format"] == "text";
		let observe = request["observe"] != false;
		let entity = request["entity"].as_str().map(str::to_owned);
		let answer = self.asp_answer(request).await?;
		if observe && matches!(op.as_str(), "search" | "expand" | "actions" | "act") {
			self.asp_activity
				.record(&op, entity.into_iter().collect(), Some(&answer));
		}
		Ok(match text {
			true => Value::String(render::compact(&op, &answer)),
			false => answer,
		})
	}

	async fn asp_answer(&self, request: Value) -> Result<Value> {
		let registry = Registry::of(&self.participants());
		let entity = || {
			let id = request["entity"].as_str().unwrap_or_default();
			match scheme_of(id) {
				Some(_) => Ok(id),
				None => Err(Error::Argument(format!(
					"`entity` must be `scheme:key`, got `{id}`"
				))),
			}
		};
		match request["op"].as_str() {
			Some("activity") => Ok(self.asp_activity.snapshot()),
			Some("types") => Ok(registry.types(&self.participants())),
			Some("expand") => {
				let depth = request["depth"]
					.as_u64()
					.unwrap_or(DEPTH)
					.clamp(1, MAX_DEPTH);
				let limit = request["limit"]
					.as_u64()
					.map_or(LIMIT, |n| (n as usize).clamp(1, LIMIT));
				self.asp_expand(&registry, entity()?, depth, limit).await
			}
			Some("search") => {
				let query = request["query"].as_str().unwrap_or_default();
				if query.trim().is_empty() {
					return Err(Error::Argument("`query` must not be empty".into()));
				}
				let limit = request["limit"]
					.as_u64()
					.map_or(LIMIT, |n| (n as usize).clamp(1, LIMIT));
				self.asp_search(&registry, query, limit).await
			}
			Some("actions") => Ok(json!({ "actions": registry.actions(entity()?) })),
			Some("act") => {
				let entity = entity()?;
				let name = request["action"].as_str().unwrap_or_default();
				let action = registry
					.actions(entity)
					.into_iter()
					.find(|a| a.name == name)
					.ok_or_else(|| {
						Error::Argument(format!(
							"no loaded cartridge offers `{name}` on `{entity}`"
						))
					})?;
				// The tool event is the whole dispatch: whatever answers or
				// refuses it, policy included, reaches the caller as it is.
				// `args` is the tool's input, as an agent would pass it; the
				// tool event itself takes the envelope every harness sends.
				let call = json!({ "op": "call", "input": action.args });
				Ok(Box::pin(self.bail(&action.tool, call))
					.await?
					.unwrap_or(Value::Null))
			}
			other => Err(Error::Argument(format!(
				"unknown asp op {other:?}; one of types, expand, search, actions, act, activity"
			))),
		}
	}
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/asp/mod.rs"]
mod tests;
