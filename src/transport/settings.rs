use std::collections::BTreeMap;

use serde_json::{json, Value as Json};

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
	Integer,
	Number,
	Boolean,
	String,
	List,
	Table,
}

impl Kind {
	fn name(self) -> &'static str {
		match self {
			Kind::Integer => "integer",
			Kind::Number => "number",
			Kind::Boolean => "boolean",
			Kind::String => "string",
			Kind::List => "list",
			Kind::Table => "table",
		}
	}

	fn holds(self, value: &Json) -> bool {
		match self {
			Kind::Integer => value.is_i64() || value.is_u64(),
			Kind::Number => value.is_number(),
			Kind::Boolean => value.is_boolean(),
			Kind::String => value.is_string(),
			Kind::List => value.is_array(),
			Kind::Table => value.is_object(),
		}
	}
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
	#[serde(rename = "type")]
	pub kind: Kind,
	pub default: Json,
	#[serde(default)]
	pub optional: bool,
	#[serde(default)]
	pub min: Option<f64>,
	#[serde(default)]
	pub max: Option<f64>,
	#[serde(default)]
	pub doc: Option<String>,
}

impl Spec {
	fn check(&self, key: &str, value: &Json) -> std::result::Result<(), String> {
		if self.optional && value.is_null() {
			return Ok(());
		}
		if !self.kind.holds(value) {
			return Err(format!(
				"`{key}` must be {}, not {}",
				self.kind.name(),
				describe(value)
			));
		}
		let Some(n) = value.as_f64() else {
			return Ok(());
		};
		let bound = |n: f64| match n.fract() == 0.0 && n.abs() < 9.007_199_254_740_992e15 {
			true => (n as i64).to_string(),
			false => n.to_string(),
		};
		if self.min.is_some_and(|min| n < min) {
			return Err(format!(
				"`{key}` is below its minimum of {}",
				bound(self.min.unwrap())
			));
		}
		if self.max.is_some_and(|max| n > max) {
			return Err(format!(
				"`{key}` is above its maximum of {}",
				bound(self.max.unwrap())
			));
		}
		Ok(())
	}

	pub fn describe(&self) -> Json {
		let mut out = json!({"type": self.kind.name(), "default": self.default});
		let map = out.as_object_mut().expect("object");
		if self.optional {
			map.insert("optional".into(), json!(true));
		}
		let bound = |n: f64| match n.fract() == 0.0 && n.abs() < 9.007_199_254_740_992e15 {
			true => json!(n as i64),
			false => json!(n),
		};
		if let Some(min) = self.min {
			map.insert("min".into(), bound(min));
		}
		if let Some(max) = self.max {
			map.insert("max".into(), bound(max));
		}
		if let Some(doc) = &self.doc {
			map.insert("doc".into(), json!(doc));
		}
		out
	}
}

fn describe(value: &Json) -> &'static str {
	match value {
		Json::Null => "null",
		Json::Bool(_) => "a boolean",
		Json::Number(_) => "a number",
		Json::String(_) => "a string",
		Json::Array(_) => "a list",
		Json::Object(_) => "a table",
	}
}

pub type Specs = BTreeMap<String, Spec>;

pub fn merge(base: &mut Json, over: Json) {
	match (base, over) {
		(Json::Object(base), Json::Object(over)) => {
			for (key, value) in over {
				match value {
					Json::Null => {
						base.remove(&key);
					}
					value => match base.get_mut(&key) {
						Some(slot) => merge(slot, value),
						None => {
							base.insert(key, value);
						}
					},
				}
			}
		}
		(base, over) => *base = over,
	}
}

pub fn get<'a>(value: &'a Json, key: &str) -> Option<&'a Json> {
	key.split('.').try_fold(value, |at, step| at.get(step))
}

pub fn set(value: &mut Json, key: &str, leaf: Json) {
	if !value.is_object() {
		*value = json!({});
	}
	let mut at = value;
	let mut steps = key.split('.').peekable();
	while let Some(step) = steps.next() {
		let map = at.as_object_mut().expect("table on the way down");
		if steps.peek().is_none() {
			map.insert(step.to_owned(), leaf);
			return;
		}
		at = map
			.entry(step.to_owned())
			.and_modify(|slot| {
				if !slot.is_object() {
					*slot = json!({});
				}
			})
			.or_insert_with(|| json!({}));
	}
}

fn enclosing(key: &str) -> impl Iterator<Item = &str> {
	key.match_indices('.').map(|(at, _)| &key[..at])
}

fn absent(specs: &Specs, key: &str, settled: Option<&Json>) -> bool {
	enclosing(key).any(|outer| match specs.get(outer) {
		Some(spec) if spec.optional => match settled {
			Some(out) => get(out, outer).is_none_or(Json::is_null),
			None => spec.default.is_null(),
		},
		_ => false,
	})
}

pub const YOLO_ENV: &str = "CARTRIDGE_YOLO";

pub fn yolo() -> bool {
	std::env::var(YOLO_ENV).is_ok_and(|value| value == "1")
}

pub fn defaults(specs: &Specs) -> Json {
	let mut out = json!({});
	for (key, spec) in specs {
		if absent(specs, key, None) {
			continue;
		}
		set(&mut out, key, spec.default.clone());
	}
	out
}

pub fn apply(specs: &Specs, config: Json, at: &str) -> Result<Json, String> {
	let mut out = defaults(specs);
	merge(&mut out, config);
	for (key, spec) in specs {
		// Specs are ordered: a table settles before the keys inside it, so
		// `absent` reads what the layers actually left, not the default.
		if absent(specs, key, Some(&out)) {
			continue;
		}
		if get(&out, key).is_none() {
			set(&mut out, key, spec.default.clone());
		}
		let value = get(&out, key).expect("just filled");
		spec.check(key, value).map_err(|e| format!("{at}: {e}"))?;
	}
	Ok(out)
}

pub fn undeclared(specs: &Specs, config: &Json) -> Vec<String> {
	fn walk(at: &Json, prefix: &str, specs: &Specs, out: &mut Vec<String>) {
		let Some(map) = at.as_object() else {
			return;
		};
		for (key, value) in map {
			let dotted = match prefix.is_empty() {
				true => key.clone(),
				false => format!("{prefix}.{key}"),
			};
			if specs.contains_key(&dotted) {
				continue;
			}
			let inside = specs.keys().any(|k| k.starts_with(&format!("{dotted}.")));
			match value.is_object() && inside {
				true => walk(value, &dotted, specs, out),
				false => out.push(dotted),
			}
		}
	}
	let mut out = Vec::new();
	walk(config, "", specs, &mut out);
	out.sort();
	out
}

pub fn declared(document: &str) -> Json {
	#[derive(serde::Deserialize)]
	struct Document {
		#[serde(default)]
		settings: Specs,
	}
	let document: Document =
		serde_json::from_str(document).expect("a cartridge's own cartridge.json");
	defaults(&document.settings)
}
