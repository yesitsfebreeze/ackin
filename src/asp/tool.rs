//! `tool.asp`: ASP as one of the agent's tools.

use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::host::Host;

use super::{LIMIT, MAX_DEPTH};

/// What an agent gets when it names no limit: enough to choose the next
/// expand, few enough that an answer stays a short read.
const AGENT_LIMIT: u64 = 20;

impl Host {
	/// The tool envelope every harness speaks: `describe`, `call`, `cancel`.
	/// It offers no `act`: an action names a tool, and the agent runs that
	/// tool through its own dispatch, where policy is asked.
	pub async fn asp_tool(&self, args: Value) -> Result<Value> {
		match args["op"].as_str() {
			Some("describe") => Ok(json!({
				"name": "asp",
				"description": "The one interface to the project's world. `search` finds entities (files, symbols, memos, ...) by words; `expand` returns everything every cartridge knows about one entity `scheme:key` (nodes, edges, attributes, staleness) and the actions you can run on it, each naming a tool and its arguments; `types` lists the schemes and edge kinds that exist; `activity` reads retained observed uses without recording another use. Monitoring expansions can set `observe:false`.",
				"reads": true,
				"input_schema": {
					"type": "object",
					"properties": {
						"op": { "type": "string", "enum": ["types", "expand", "search", "actions", "activity"] },
						"entity": { "type": "string", "description": "scheme:key, for example file:src/a.rs" },
						"query": { "type": "string" },
						"provider_timeout_ms": { "type": "integer", "minimum": 1, "maximum": 30000, "description": "Optional shared retrieval deadline. Slow providers are reported unavailable; completed evidence is retained." },
						"observe": { "type": "boolean", "default": true, "description": "Record this lookup as observed use; set false for monitoring. Activity reads never record use." },
						"depth": { "type": "integer", "minimum": 1, "maximum": MAX_DEPTH },
						"limit": { "type": "integer", "minimum": 1, "maximum": LIMIT, "description": "at most this many nodes or hits; 20 when absent" },
						"format": { "type": "string", "enum": ["text", "json"], "description": "text (the default): one line per node, edge, action and source; json: the full answer" }
					},
					"required": ["op"],
					"additionalProperties": false
				}
			})),
			Some("cancel") => Ok(json!({ "content": "nothing to cancel", "error": false })),
			Some("call") if args["input"]["op"] == "act" => Ok(json!({
				"content": "asp does not run actions; call the tool the action names",
				"error": true,
			})),
			Some("call") => {
				let mut input = args["input"].clone();
				let op = input["op"].as_str().unwrap_or_default().to_owned();
				if input["format"].is_null() {
					input["format"] = json!("text");
				}
				if input["limit"].is_null() && matches!(op.as_str(), "search" | "expand") {
					input["limit"] = json!(AGENT_LIMIT);
				}
				Ok(match self.asp(input).await {
					Ok(Value::String(text)) => json!({ "content": text, "error": false }),
					Ok(answer) => json!({ "content": answer.to_string(), "error": false }),
					Err(error) => json!({ "content": error.to_string(), "error": true }),
				})
			}
			other => Err(Error::Argument(format!("unknown tool.asp op: {other:?}"))),
		}
	}
}
