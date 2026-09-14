//! Settings: the one place a tunable value is declared, layered and read.
//!
//! A number that changes behaviour is a setting, not a constant. Every cap,
//! timeout, queue depth and budget in this system is declared once — by the
//! host for its own, by each cartridge in its `cartridge.json` — and is then
//! readable, overridable and listable by name. There is no second place a
//! limit hides: if a value can be tuned, `cartridge settings` names it.
//!
//! **Three layers, outermost first.** A declaration carries the default, so an
//! unconfigured system is a configured one:
//!
//! ```text
//!   declared default        cartridge.json `settings`, or HOST below
//!   author default          cartridge.json `config`
//!   global config           ~/.cartridge/config.lua
//!   project config          <project>/.cartridge/config.lua
//!   profile entry config    init.lua `config = {...}` on the entry
//! ```
//!
//! Each layer is laid *over* the one above it and merges field by field, so a
//! project file naming one key keeps every other key the layers above settled.
//! Objects merge; scalars and lists replace, because half a list is not a
//! configuration anyone wrote.
//!
//! **A cartridge contributes keys by declaring them.** `settings` in the
//! document is a map of dotted key to `{type, default, min, max, doc}`. What it
//! declares appears in the global and project files under the cartridge's entry
//! id, is filled with its default when nothing names it, is range-checked
//! before the cartridge starts, and is printed by `cartridge settings`. A
//! cartridge that declares its keys does not carry its own fallbacks.
//!
//! **The host's own keys live under `host`.** They are declared in [`HOST`]
//! rather than a document, because the host has no `cartridge.json` to put them
//! in, and are read through [`host`].
//!
//! [`files`] reads the two configuration files; [`host`] is the host's own
//! declaration and its settled values.

mod files;
mod host;

use std::collections::BTreeMap;

use serde_json::{json, Value as Json};

use crate::error::{Error, Result};

pub use files::{global_path, layers, project_path, read, source};
pub use host::{host, host_specs, settle, Host};

/// What a setting's value must be. Narrow on purpose: a type that cannot be
/// checked before a cartridge starts is not a setting, it is an argument.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
	/// A whole number. `min`/`max` bound it; a bound is inclusive.
	Integer,
	/// A real number, bounded the same way.
	Number,
	Boolean,
	String,
	/// An ordered list. Replaced wholesale by an overriding layer.
	List,
	/// A nested table whose own keys are not individually declared. Prefer
	/// dotted keys — `ship.remote` — so each leaf is declared and checkable;
	/// reach for this only where the keys are the user's to invent.
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

	/// Whether a value is of this kind. `null` is never of any kind: a declared
	/// key is either configured or left at its default, and an explicit `null`
	/// is neither.
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

/// One declared key. Read straight off `cartridge.json`, so a misspelled field
/// is an error at load rather than a silently ignored intention.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
	#[serde(rename = "type")]
	pub kind: Kind,
	/// The value when no layer names the key. Required: a setting without a
	/// default is a required argument wearing a setting's clothes. It may be
	/// `null` only where `optional` says so.
	pub default: Json,
	/// Whether "nothing" is one of this key's values. A port the host binds only
	/// when something names one, a model that gates a review only where it is
	/// configured: absent is the answer, not a missing answer, and `null` is how
	/// the configuration says it. Off by default, so a key is filled unless its
	/// author says emptiness means something.
	#[serde(default)]
	pub optional: bool,
	/// Inclusive bounds, for `integer` and `number` only.
	#[serde(default)]
	pub min: Option<f64>,
	#[serde(default)]
	pub max: Option<f64>,
	/// What the key does, in one line. `cartridge settings` prints it.
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

	/// The declaration itself as data, for `cartridge settings --json` and for
	/// anything else that wants to render the surface without re-reading it.
	pub fn describe(&self) -> Json {
		let mut out = json!({"type": self.kind.name(), "default": self.default});
		let map = out.as_object_mut().expect("object");
		if self.optional {
			map.insert("optional".into(), json!(true));
		}
		// A bound on an integer reads as an integer. It is carried as `f64` so
		// one field bounds both numeric kinds; rendering `1000000.0` for a step
		// count would be the storage showing through.
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

/// A cartridge's declared surface: dotted key to declaration, ordered so two
/// listings of the same cartridge read the same way.
pub type Specs = BTreeMap<String, Spec>;

/// Lay `over` on top of `base`, field by field. Two tables merge key by key;
/// anything else replaces, because a list or a scalar is one value and half of
/// one is not a configuration. An explicit `null` in `over` removes the key,
/// which is how a layer says "back to the default" without knowing it.
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

/// Follow a dotted key into a table. `None` where any step is missing or is not
/// a table, which is the same answer as "nothing configured it".
pub fn get<'a>(value: &'a Json, key: &str) -> Option<&'a Json> {
	key.split('.').try_fold(value, |at, step| at.get(step))
}

/// Write a dotted key into a table, creating the tables on the way down.
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

/// The dotted keys `key` sits inside, outermost first: `owner.timeout_ms`
/// yields `owner`. The key itself is not one of them.
fn enclosing(key: &str) -> impl Iterator<Item = &str> {
	key.match_indices('.').map(|(at, _)| &key[..at])
}

/// Whether `key` lies inside a declared table that is not there. A key declared
/// `optional` with a `null` default means "nothing, until a layer says
/// otherwise", and the defaults of the keys *inside* it would conjure the very
/// table whose absence carries that meaning — an unattached memory would arrive
/// holding `{"owner": {"timeout_ms": 30000}}` and read as attached. So the
/// inside of an absent table stays absent, and fills in the moment a layer
/// names the table itself.
fn absent(specs: &Specs, key: &str, settled: Option<&Json>) -> bool {
	enclosing(key).any(|outer| match specs.get(outer) {
		Some(spec) if spec.optional => match settled {
			// Settling: the table is whatever the layers left, and only a table
			// that is really there admits its own keys.
			Some(out) => get(out, outer).is_none_or(Json::is_null),
			// Declaring: nothing has been laid over the defaults yet.
			None => spec.default.is_null(),
		},
		_ => false,
	})
}

/// Every declared default, as the table a configuration lays itself over.
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

/// Fill `config` with what it does not name and refuse what it names wrongly.
/// The result is complete: every declared key holds a value of its declared
/// kind inside its declared bounds, so the cartridge reading it needs no
/// fallback of its own and no second opinion about what a missing key means.
///
/// Keys the declaration does not mention are kept, not rejected. A cartridge
/// migrating onto settings has configuration older than its declaration, and
/// dropping it on the way in would be a silent behaviour change; `undeclared`
/// is how that backlog stays visible instead.
pub fn apply(specs: &Specs, config: Json, at: &str) -> Result<Json> {
	let mut out = defaults(specs);
	merge(&mut out, config);
	for (key, spec) in specs {
		// Specs are ordered, so a table is settled before the keys inside it and
		// `absent` reads the answer the layers actually left, not the default.
		if absent(specs, key, Some(&out)) {
			continue;
		}
		// A layer may name a key as `null`, which [`merge`] reads as "back to
		// the default" and removes. Putting the default back here is what makes
		// that true, and is why the result is complete whatever the layers did.
		if get(&out, key).is_none() {
			set(&mut out, key, spec.default.clone());
		}
		let value = get(&out, key).expect("just filled");
		spec.check(key, value)
			.map_err(|e| Error::Settings(format!("{at}: {e}")))?;
	}
	Ok(out)
}

/// Configured keys no declaration mentions, dotted, sorted. The migration
/// checklist: a cartridge is fully on settings exactly when this is empty.
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
			// A table may still be the *inside* of a declared dotted key, so
			// descend before calling it undeclared; only a leaf with nothing
			// declared under it is one.
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

/// What a cartridge's own document declares, as the complete table of defaults
/// its code can read without a host.
///
/// A cartridge normally never needs this: the host settles its configuration
/// against the same declarations and hands it a complete table at `apply`. It
/// is for the cases where no host did — a unit test, a standalone run — so that
/// a cartridge's tests exercise the values it actually ships rather than a
/// second copy of them written in Rust. Pass `include_str!("../cartridge.json")`
/// so the declarations travel with the binary.
///
/// Panics on a document that will not parse: a cartridge whose own manifest is
/// malformed has nothing to fall back to, and the failure belongs at the first
/// call rather than somewhere later that reads a zero.
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
