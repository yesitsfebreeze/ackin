//! `tool.asp`: ASP as one of the agent's tools.

use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::host::Host;

use super::{LIMIT, MAX_DEPTH};

impl Host {
	/// The tool envelope every harness speaks: `describe`, `call`, `cancel`.
	/// It offers no `act`: an action names a tool, and the agent runs that
	/// tool through its own dispatch, where policy is asked.
	pub async fn asp_tool(&self, args: Value) -> Result<Value> {
		match args["op"].as_str() {
			Some("describe") => Ok(json!({
				"name": "asp",
				"description": "The one interface to the project's world. `search` finds entities (files, symbols, memos, ...) by words; `expand` returns everything every cartridge knows about one entity `scheme:key` (nodes, edges, attributes, staleness) and the actions you can run on it, each naming a tool and its arguments; `types` lists the schemes and edge kinds that exist.",
				"reads": true,
				"input_schema": {
					"type": "object",
					"properties": {
						"op": { "type": "string", "enum": ["types", "expand", "search", "actions"] },
						"entity": { "type": "string", "description": "scheme:key, for example file:src/a.rs" },
						"query": { "type": "string" },
						"depth": { "type": "integer", "minimum": 1, "maximum": MAX_DEPTH },
						"limit": { "type": "integer", "minimum": 1, "maximum": LIMIT }
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
			Some("call") => Ok(match self.asp(args["input"].clone()).await {
				Ok(answer) => json!({ "content": answer.to_string(), "error": false }),
				Err(error) => json!({ "content": error.to_string(), "error": true }),
			}),
			other => Err(Error::Argument(format!("unknown tool.asp op: {other:?}"))),
		}
	}
}
