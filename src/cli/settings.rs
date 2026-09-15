use std::path::Path;
use std::process::ExitCode;

use cartridge::host::Host;
use cartridge::loader;
use cartridge::settings::{self, Sources, Spec, Specs};
use cartridge::{Error, Result};
use serde_json::{json, Value};

use super::width::{cells, fit, pad, pad_start};
use super::{Project, FAILED};

pub(crate) fn settings(
	project: &Project,
	what: Option<&str>,
	as_json: bool,
	template: bool,
) -> Result<ExitCode> {
	let host = Host::new(&project.dir, &project.descriptor)?;
	let entries = host.settings().map_err(|e| {
		Error::Descriptor(format!(
			"{}: {e}",
			project.descriptor.join("init.lua").display()
		))
	})?;
	let descriptor = project.descriptor.as_path();
	if template {
		print_template(descriptor, &entries);
		return Ok(ExitCode::SUCCESS);
	}
	if as_json {
		println!("{}", as_json_document(descriptor, &entries));
		return Ok(ExitCode::SUCCESS);
	}
	Ok(match table(descriptor, &entries, what) {
		0 => ExitCode::SUCCESS,
		_ => ExitCode::from(FAILED),
	})
}

fn host_settled(descriptor: &Path) -> Value {
	let configured = settings::layers(descriptor)
		.unwrap_or_else(|e| {
			tracing::warn!(target: "cartridge", "settings: {e}");
			json!({})
		})
		.get("host")
		.cloned()
		.unwrap_or_else(|| json!({}));
	settings::apply(settings::host_specs(), configured, "host").unwrap_or_else(|e| {
		tracing::warn!(target: "cartridge", "settings: {e}; showing declared defaults");
		settings::defaults(settings::host_specs())
	})
}

struct Row {
	key: String,
	kind: String,
	value: String,
	source: &'static str,
	doc: String,
}

fn table(descriptor: &Path, entries: &[loader::SettingsInfo], what: Option<&str>) -> usize {
	let wanted = |section: &str, key: &str| match what {
		None => true,
		Some(w) => {
			let dotted = format!("{section}.{key}");
			section == w || dotted == w || dotted.starts_with(&format!("{w}."))
		}
	};
	let mut rows: Vec<Row> = Vec::new();
	let sources = Sources::read(descriptor);
	let mut push = |section: &str, key: &str, spec: Option<&Spec>, value: &Value| {
		if !wanted(section, key) {
			return;
		}
		let dotted = format!("{section}.{key}");
		let declared = spec.map_or(Value::Null, |s| s.default.clone());
		let source = sources.of(&dotted, value, &declared);
		let kind = spec.map_or("undeclared".to_owned(), |s| {
			s.describe()["type"].as_str().unwrap_or("?").to_owned()
		});
		rows.push(Row {
			key: dotted,
			kind,
			value: serde_json::to_string(value).unwrap_or_default(),
			source,
			doc: spec.and_then(|s| s.doc.clone()).unwrap_or_default(),
		});
	};
	let host = host_settled(descriptor);
	for (key, spec) in settings::host_specs() {
		let value = settings::get(&host, key).cloned().unwrap_or(Value::Null);
		push("host", key, Some(spec), &value);
	}
	for entry in entries {
		for (key, spec) in &entry.specs {
			let value = settings::get(&entry.settled, key)
				.cloned()
				.unwrap_or(Value::Null);
			push(&entry.id, key, Some(spec), &value);
		}
		for key in &entry.undeclared {
			let value = settings::get(&entry.settled, key)
				.cloned()
				.unwrap_or(Value::Null);
			push(&entry.id, key, None, &value);
		}
	}
	print_rows(&rows);
	entries
		.iter()
		.filter(|e| e.undeclared.iter().any(|key| wanted(&e.id, key)))
		.count()
}

fn print_rows(rows: &[Row]) {
	const VALUE_WIDTH: usize = 44;
	let width = |pick: fn(&Row) -> &str| rows.iter().map(|r| cells(pick(r))).max().unwrap_or(0);
	let (w0, w1, w3) = (width(|r| &r.key), width(|r| &r.kind), width(|r| r.source));
	let w2 = width(|r| &r.value).min(VALUE_WIDTH);
	for row in rows {
		let value = match cells(&row.value) > VALUE_WIDTH {
			true => format!("{}…", fit(&row.value, VALUE_WIDTH - 1)),
			false => row.value.clone(),
		};
		let line = format!(
			"{}  {}  {}  {}",
			pad(&row.key, w0),
			pad(&row.kind, w1),
			pad_start(&value, w2),
			pad(row.source, w3)
		);
		match row.doc.is_empty() {
			true => println!("{line}"),
			false => println!("{line}  {}", row.doc),
		}
	}
}

fn as_json_document(descriptor: &Path, entries: &[loader::SettingsInfo]) -> String {
	let sources = Sources::read(descriptor);
	let describe = |section: &str, specs: &Specs, settled: &Value, undeclared: &[String]| {
		let keys: serde_json::Map<String, Value> = specs
			.iter()
			.map(|(key, spec)| {
				let mut out = spec.describe();
				let map = out.as_object_mut().expect("object");
				map.insert(
					"value".into(),
					settings::get(settled, key).cloned().unwrap_or(Value::Null),
				);
				map.insert(
					"source".into(),
					json!(sources.of(
						&format!("{section}.{key}"),
						settings::get(settled, key).unwrap_or(&Value::Null),
						&spec.default,
					)),
				);
				(key.clone(), out)
			})
			.collect();
		json!({"keys": keys, "undeclared": undeclared})
	};
	let host_settled = host_settled(descriptor);
	let mut out = serde_json::Map::new();
	out.insert(
		"host".into(),
		describe("host", settings::host_specs(), &host_settled, &[]),
	);
	for entry in entries {
		out.insert(
			entry.id.clone(),
			describe(&entry.id, &entry.specs, &entry.settled, &entry.undeclared),
		);
	}
	serde_json::to_string_pretty(&Value::Object(out)).unwrap_or_default()
}

fn print_template(descriptor: &Path, entries: &[loader::SettingsInfo]) {
	println!("-- Every setting this descriptor has, at its current value.");
	println!("-- Save as ~/.cartridge/config.lua for this machine, or as");
	println!("-- .cartridge/config.lua for this project alone. Delete what you");
	println!("-- do not want to pin: an absent key keeps its declared default.");
	println!("return {{");
	let host = host_settled(descriptor);
	section("host", settings::host_specs(), &host);
	for entry in entries {
		if entry.specs.is_empty() {
			continue;
		}
		section(&entry.id, &entry.specs, &entry.settled);
	}
	println!("}}");
}

fn section(id: &str, specs: &Specs, settled: &Value) {
	println!("\t{} = {{", lua_key(id));
	nested(settled, specs, "", 2);
	println!("\t}},");
}

fn nested(value: &Value, specs: &Specs, prefix: &str, depth: usize) {
	let Some(map) = value.as_object() else {
		return;
	};
	let pad = "\t".repeat(depth);
	for (key, value) in map {
		let dotted = match prefix.is_empty() {
			true => key.clone(),
			false => format!("{prefix}.{key}"),
		};
		if let Some(doc) = specs.get(&dotted).and_then(|s| s.doc.as_deref()) {
			println!("{pad}-- {doc}");
		}
		let inside = specs.keys().any(|k| k.starts_with(&format!("{dotted}.")));
		match value.is_object() && inside {
			true => {
				println!("{pad}{} = {{", lua_key(key));
				nested(value, specs, &dotted, depth + 1);
				println!("{pad}}},");
			}
			false => println!("{pad}{} = {},", lua_key(key), lua_value(value)),
		}
	}
}

pub(crate) fn lua_key(key: &str) -> String {
	let bare = !key.is_empty()
		&& !key.starts_with(|c: char| c.is_ascii_digit())
		&& key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
	match bare {
		true => key.to_owned(),
		false => format!("[{key:?}]"),
	}
}

pub(crate) fn lua_value(value: &Value) -> String {
	match value {
		Value::Null => "nil".to_owned(),
		Value::Bool(b) => b.to_string(),
		Value::Number(n) => n.to_string(),
		Value::String(s) => format!("{s:?}"),
		Value::Array(items) => format!(
			"{{ {} }}",
			items.iter().map(lua_value).collect::<Vec<_>>().join(", ")
		),
		Value::Object(map) => format!(
			"{{ {} }}",
			map.iter()
				.map(|(k, v)| format!("{} = {}", lua_key(k), lua_value(v)))
				.collect::<Vec<_>>()
				.join(", ")
		),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn lua_keys_and_values_render_as_lua_source() {
		assert_eq!(lua_key("max_steps"), "max_steps");
		assert_eq!(lua_key("live-record"), "[\"live-record\"]");
		assert_eq!(lua_key("1st"), "[\"1st\"]");
		assert_eq!(lua_value(&json!(null)), "nil");
		assert_eq!(lua_value(&json!([1, "a"])), "{ 1, \"a\" }");
		assert_eq!(lua_value(&json!({"k": true})), "{ k = true }");
	}
}
