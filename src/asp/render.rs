//! ASP answers as compact text, the form an agent reads: one line per node,
//! edge, action and source, with ids kept whole so any of them can be expanded.

use serde_json::Value;

fn text(value: &Value) -> &str {
	value.as_str().unwrap_or_default()
}

/// `contributor@revision` for each assertion, the revision cut to 8 characters
/// and `!stale` appended when the owner's revision has moved on.
fn asserted(assertions: &Value) -> String {
	let one = |a: &Value| {
		let mut out = text(&a["contributor"]).to_owned();
		if let Some(revision) = a["revision"].as_str() {
			out.push('@');
			out.push_str(&revision[..revision.len().min(8)]);
		}
		if a["stale"] == true {
			out.push_str(" !stale");
		}
		out
	};
	match assertions {
		Value::Array(all) => all.iter().map(one).collect::<Vec<_>>().join(" "),
		single => one(single),
	}
}

fn node_line(node: &Value) -> String {
	let id = text(&node["id"]);
	let mut parts = vec![id.to_owned()];
	let name = text(&node["name"]);
	if !name.is_empty() && !id.ends_with(name) {
		parts.push(name.to_owned());
	}
	let description = text(&node["description"]);
	if !description.is_empty() && description != name {
		parts.push(description.lines().next().unwrap_or_default().to_owned());
	}
	if let Some(attributes) = node["attributes"].as_object() {
		let attributes: Vec<String> = attributes
			.iter()
			.map(|(key, value)| match value {
				Value::String(value) => format!("{key}={value}"),
				other => format!("{key}={other}"),
			})
			.collect();
		if !attributes.is_empty() {
			parts.push(attributes.join(" "));
		}
	}
	format!(
		"{}  [{}]",
		parts.join(" | "),
		asserted(&node["contributors"])
	)
}

fn edge_line(edge: &Value) -> String {
	format!(
		"{} -{}-> {}  [{}]",
		text(&edge["from"]),
		text(&edge["kind"]),
		text(&edge["to"]),
		asserted(edge)
	)
}

fn tail(answer: &Value, out: &mut Vec<String>) {
	for action in answer["actions"].as_array().into_iter().flatten() {
		out.push(format!(
			"action {} -> {} {}",
			text(&action["name"]),
			text(&action["tool"]),
			action["args"]
		));
	}
	let sources: Vec<String> = answer["sources"]
		.as_array()
		.into_iter()
		.flatten()
		.map(|s| match s["error"].as_str() {
			Some(error) => format!(
				"{}:{} ({error})",
				text(&s["contributor"]),
				text(&s["state"])
			),
			None => format!("{}:{}", text(&s["contributor"]), text(&s["state"])),
		})
		.collect();
	if !sources.is_empty() {
		out.push(format!("sources {}", sources.join(", ")));
	}
}

/// An `expand`, `search` or `actions` answer as lines; anything else, such as
/// `types`, is already a table and stays JSON.
pub fn compact(op: &str, answer: &Value) -> String {
	let mut out = Vec::new();
	match op {
		"expand" => {
			for node in answer["nodes"].as_array().into_iter().flatten() {
				out.push(node_line(node));
			}
			for edge in answer["edges"].as_array().into_iter().flatten() {
				out.push(edge_line(edge));
			}
			tail(answer, &mut out);
		}
		"search" => {
			for hit in answer["hits"].as_array().into_iter().flatten() {
				let score = hit["score"].as_f64().unwrap_or_default();
				out.push(format!("{score:.1} {}", node_line(&hit["node"])));
			}
			for edge in answer["edges"].as_array().into_iter().flatten() {
				out.push(edge_line(edge));
			}
			tail(answer, &mut out);
		}
		"actions" => tail(answer, &mut out),
		_ => return answer.to_string(),
	}
	out.join("\n")
}
