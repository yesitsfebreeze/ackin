//! `settings`: every tunable value this profile has, as a table, as JSON, or
//! as a `config.lua` template ready to save.

use std::path::Path;
use std::process::ExitCode;

use cartridge::loader;
use cartridge::lua::Host;
use cartridge::runtime::Runtime;
use cartridge::settings::{self, Spec, Specs};
use cartridge::{Error, Result};
use serde_json::{json, Value};

use super::{Project, FAILED};

pub(crate) fn settings(
	project: &Project,
	what: Option<&str>,
	as_json: bool,
	template: bool,
) -> Result<ExitCode> {
	let host = Host::new(Runtime::new(), &project.dir, &project.profile);
	let entries = host.settings().map_err(|e| {
		Error::Profile(format!(
			"{}: {e}",
			project.profile.join("init.lua").display()
		))
	})?;
	let profile = project.profile.as_path();
	if template {
		print_template(profile, &entries);
		return Ok(ExitCode::SUCCESS);
	}
	if as_json {
		println!("{}", as_json_document(profile, &entries));
		return Ok(ExitCode::SUCCESS);
	}
	// A cartridge configured with keys it never declared is the one
	// thing this listing is for; saying so in the exit status is what
	// keeps the sweep finishable.
	Ok(match table(profile, &entries, what) {
		0 => ExitCode::SUCCESS,
		_ => ExitCode::from(FAILED),
	})
}

/// The host's own settings, settled against its declarations, for the three
/// renderings below. A configuration the declarations refuse does not silently
/// become the defaults here: the reason is said once, and then the defaults
/// stand — a limit that will not parse must not take the listing of limits down
/// with it.
fn host_settled(profile: &Path) -> Value {
	let configured = settings::layers(profile)
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

/// One row of the table: dotted key, kind, value, source, documentation.
struct Row {
	key: String,
	kind: String,
	value: String,
	source: &'static str,
	doc: String,
}

/// Every tunable value, in one table: the host's own first, then each
/// cartridge's, each key with its type, what it is set to, and where that came
/// from. This is the listing the system's own documentation points at, so a
/// value that cannot be found here is a value that was never a setting.
///
/// Answers how many problems it found: a cartridge configured with keys it
/// never declared is one per cartridge, so finishing the migration is a
/// non-zero exit going to zero rather than a memory of which ones were done.
fn table(profile: &Path, entries: &[loader::SettingsInfo], what: Option<&str>) -> usize {
	let wanted = |section: &str, key: &str| match what {
		None => true,
		Some(w) => {
			let dotted = format!("{section}.{key}");
			section == w || dotted == w || dotted.starts_with(&format!("{w}."))
		}
	};
	let mut rows: Vec<Row> = Vec::new();
	let mut push = |section: &str, key: &str, spec: Option<&Spec>, value: &Value| {
		if !wanted(section, key) {
			return;
		}
		let dotted = format!("{section}.{key}");
		let declared = spec.map_or(Value::Null, |s| s.default.clone());
		let source = settings::source(profile, &dotted, value, &declared);
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
	let host = host_settled(profile);
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
		// A configured key with no declaration is listed too, and marked. It is
		// working configuration — dropping it from the listing would hide the
		// one thing this listing exists to find.
		for key in &entry.undeclared {
			let value = settings::get(&entry.settled, key)
				.cloned()
				.unwrap_or(Value::Null);
			push(&entry.id, key, None, &value);
		}
	}
	print_rows(&rows);
	// Only what was asked about is counted: narrowing the listing to one
	// cartridge asks about that cartridge, and answering for the rest of the
	// profile would make a clean one look dirty.
	entries
		.iter()
		.filter(|e| e.undeclared.iter().any(|key| wanted(&e.id, key)))
		.count()
}

/// One wide value — a whole table of per-tool rules is a common one — must
/// not push every other column off the terminal, so the value column is
/// elided past a readable width. `--json` is the un-elided answer.
fn print_rows(rows: &[Row]) {
	const VALUE_WIDTH: usize = 44;
	let width = |pick: fn(&Row) -> &str| rows.iter().map(|r| pick(r).len()).max().unwrap_or(0);
	let (w0, w1, w3) = (width(|r| &r.key), width(|r| &r.kind), width(|r| r.source));
	let w2 = width(|r| &r.value).min(VALUE_WIDTH);
	for row in rows {
		let value = match row.value.chars().count() > VALUE_WIDTH {
			true => format!(
				"{}…",
				row.value.chars().take(VALUE_WIDTH - 1).collect::<String>()
			),
			false => row.value.clone(),
		};
		let line = format!(
			"{:w0$}  {:w1$}  {:>w2$}  {:w3$}",
			row.key, row.kind, value, row.source
		);
		match row.doc.is_empty() {
			true => println!("{line}"),
			false => println!("{line}  {}", row.doc),
		}
	}
}

/// The listing as data: declarations, settled values and the file each came
/// from, for anything reading this surface rather than looking at it.
fn as_json_document(profile: &Path, entries: &[loader::SettingsInfo]) -> String {
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
					json!(settings::source(
						profile,
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
	let host_settled = host_settled(profile);
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

/// The same surface as a `config.lua`: every key, its documentation above it,
/// and its current value. Saving this as `~/.cartridge/config.lua` changes
/// nothing and leaves every knob in reach, which is the point — a person
/// tuning a system should not have to discover the key's name first.
fn print_template(profile: &Path, entries: &[loader::SettingsInfo]) {
	println!("-- Every setting this profile has, at its current value.");
	println!("-- Save as ~/.cartridge/config.lua for this machine, or as");
	println!("-- .cartridge/config.lua for this project alone. Delete what you");
	println!("-- do not want to pin: an absent key keeps its declared default.");
	println!("return {{");
	let host = host_settled(profile);
	section("host", settings::host_specs(), &host);
	for entry in entries {
		if entry.specs.is_empty() {
			continue;
		}
		section(&entry.id, &entry.specs, &entry.settled);
	}
	println!("}}");
}

/// One cartridge's table, written as the nested tables its dotted keys mean.
/// `ship.remote` is `ship = { remote = ... }` here and not a key with a dot in
/// its name, which is a different thing and would configure nothing.
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
		// A table with declarations under it is written out key by key, so each
		// leaf keeps its own line and its own comment. One the declaration does
		// not reach is written inline: its shape is the user's, not ours.
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

/// A name Lua can take bare, or the bracketed string form for one it cannot —
/// `live-record` is a key, not an identifier.
fn lua_key(key: &str) -> String {
	let bare = !key.is_empty()
		&& !key.starts_with(|c: char| c.is_ascii_digit())
		&& key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
	match bare {
		true => key.to_owned(),
		false => format!("[{key:?}]"),
	}
}

fn lua_value(value: &Value) -> String {
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
