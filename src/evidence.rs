//! Bounded, untrusted evidence: the contract three cartridges share for asking
//! many sources for context at once and getting back what is addressable and
//! within budget. Source adapters own transport limits and authority; nothing
//! here trusts what a source returns.
//!
//! It lives in core because more than one cartridge speaks it — the harness
//! collects roster evidence, the proxy prepares turn context, and the memo
//! record contributes its own — and a contract with three parties belongs to
//! the host they all load into, not to one of them.
use futures::{stream::FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::time::{timeout_at, Instant};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Reference {
	pub owner: String,
	pub kind: String,
	pub id: String,
	pub revision: String,
	pub revision_kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
	pub reference: Reference,
	pub source: String,
	pub text: String,
	pub selection_reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
	pub evidence: Evidence,
	pub private: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
	Available,
	Disabled,
	Absent,
	Unavailable,
	Empty,
	Timeout,
	Partial,
}

/// What one contributor answers. It is also the wire shape a `context.<kind>`
/// provider returns when the record asks it to contribute, so a cartridge can
/// offer evidence without the record carrying that cartridge's protocol.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Contribution {
	pub contributor: String,
	pub state: Availability,
	pub candidates: Vec<Candidate>,
	pub truncated: bool,
}

pub struct Task<'a> {
	pub contributor: String,
	pub state: Availability,
	pub run: Pin<Box<dyn Future<Output = Result<Contribution, String>> + Send + 'a>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Limits {
	pub deadline_ms: u64,
	pub max_rows: usize,
	pub max_bytes: usize,
}
/// The budget a collector gets when nothing configures one.
///
/// Two consumers, and they are independent rather than copies of one truth:
/// `proxy` declares `context_limits.*` and is settled from its own document, so
/// raising that declaration moves proxy alone; `memo` collects evidence without
/// declaring a budget and takes this. Change this only for the second.
impl Default for Limits {
	fn default() -> Self {
		Self {
			deadline_ms: 2000,
			max_rows: 64,
			max_bytes: 1024 * 1024,
		}
	}
}

impl Limits {
	/// A budget has to be a budget. How large one may be is declared by the
	/// cartridge that carries these keys — `context_limits.max_bytes` and its
	/// two neighbours — and its host has already refused anything outside that;
	/// what is left to check here is that none of the three is zero, because a
	/// deadline of nothing is not a short deadline.
	pub fn validate(&self) -> Result<(), String> {
		if self.deadline_ms == 0 || self.max_rows == 0 || self.max_bytes == 0 {
			return Err("invalid context limits".into());
		}
		Ok(())
	}
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContributorStatus {
	pub contributor: String,
	pub state: Availability,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Prepared {
	pub schema: String,
	pub revision: String,
	pub rows: Vec<Evidence>,
	pub sources: Vec<ContributorStatus>,
	pub complete: bool,
	pub truncated: bool,
	pub deadline_exceeded: bool,
	pub limits: Limits,
}
impl Prepared {
	/// Read only this frozen observation; a reference never authorizes a new query.
	pub fn read(&self, reference: &Reference) -> Option<&Evidence> {
		self.rows.iter().find(|row| &row.reference == reference)
	}
	fn seal(&mut self) {
		self.revision.clear();
		self.revision = format!(
			"{:x}",
			Sha256::digest(serde_json::to_vec(self).expect("serializable context"))
		);
	}
}

fn name(s: &str) -> bool {
	!s.is_empty()
		&& s.len() <= 64
		&& s.bytes()
			.all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
}
fn text(s: &str, cap: usize) -> bool {
	!s.is_empty() && s.len() <= cap && !s.contains('\0')
}
/// Whether one row is addressable and within the contract's caps: a bounded
/// reference, a recognised revision kind, and source and text that fit. A
/// provider checks this before it nominates a row, so what it hands over is
/// what the collector will keep.
pub fn valid(row: &Evidence) -> bool {
	let r = &row.reference;
	name(&r.owner)
		&& name(&r.kind)
		&& text(&r.id, 512)
		&& r.revision.len() == 64
		&& r.revision
			.bytes()
			.all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
		&& ["source_bytes", "observed_projection"].contains(&r.revision_kind.as_str())
		&& text(&row.source, 4096)
		&& row.text.len() <= 8192
		&& text(&row.selection_reason, 256)
}

/// Poll independent sources together under one deadline. Dropping a future does
/// not assert that a remote server acknowledged cancellation. Producers must not
/// block executor threads or launch detached retries.
pub async fn collect(mut tasks: Vec<Task<'_>>, limits: Limits) -> Result<Prepared, String> {
	let started = Instant::now();
	limits.validate()?;
	if tasks.len() > 16 {
		return Err("too many context contributors".into());
	}
	tasks.sort_by(|a, b| a.contributor.cmp(&b.contributor));
	if tasks.iter().any(|t| !name(&t.contributor))
		|| tasks
			.windows(2)
			.any(|t| t[0].contributor == t[1].contributor)
	{
		return Err("invalid or duplicate contributor identity".into());
	}
	let deadline = started + Duration::from_millis(limits.deadline_ms);
	let mut sources: Vec<_> = tasks
		.iter()
		.map(|t| ContributorStatus {
			contributor: t.contributor.clone(),
			state: t.state,
		})
		.collect();
	let mut rows: Vec<Vec<Evidence>> = vec![Vec::new(); tasks.len()];
	let mut producer_truncated = false;
	let mut pending = FuturesUnordered::new();
	let mut waiting = BTreeSet::new();
	for (index, task) in tasks.into_iter().enumerate() {
		if task.state == Availability::Available {
			waiting.insert(index);
			pending.push(async move { (index, task.run.await) });
		}
	}
	while !pending.is_empty() {
		if Instant::now() >= deadline {
			break;
		}
		let Ok(Some((index, result))) = timeout_at(deadline, pending.next()).await else {
			break;
		};
		waiting.remove(&index);
		if Instant::now() >= deadline {
			sources[index].state = Availability::Timeout;
			continue;
		}
		let Ok(contribution) = result else {
			sources[index].state = Availability::Unavailable;
			continue;
		};
		if contribution.contributor != sources[index].contributor {
			sources[index].state = Availability::Unavailable;
			continue;
		}
		sources[index].state = contribution.state;
		let mut public = Vec::new();
		let mut invalid = false;
		for candidate in contribution.candidates {
			if Instant::now() >= deadline {
				sources[index].state = Availability::Timeout;
				invalid = true;
				break;
			}
			if candidate.private {
				continue;
			}
			if public.len() == 128 || !valid(&candidate.evidence) {
				invalid = true;
				break;
			}
			public.push(candidate.evidence);
		}
		if invalid {
			if sources[index].state != Availability::Timeout {
				sources[index].state = Availability::Unavailable;
			}
			continue;
		}
		if !matches!(
			contribution.state,
			Availability::Available | Availability::Partial
		) {
			if !public.is_empty() {
				sources[index].state = Availability::Unavailable;
			}
			continue;
		}
		if public.is_empty() && contribution.state == Availability::Available {
			sources[index].state = Availability::Empty;
		}
		if contribution.truncated {
			producer_truncated = true;
			sources[index].state = Availability::Partial;
		}
		rows[index] = public;
	}
	drop(pending);
	for index in waiting {
		sources[index].state = Availability::Timeout;
	}
	// Conflicting identical references disqualify every involved contributor.
	let mut identities: BTreeMap<&Reference, Vec<(usize, &Evidence)>> = BTreeMap::new();
	for (index, entries) in rows.iter().enumerate() {
		for row in entries {
			identities
				.entry(&row.reference)
				.or_default()
				.push((index, row));
		}
	}
	let mut conflicts = BTreeSet::new();
	for entries in identities.values() {
		if entries.iter().any(|(_, row)| row != &entries[0].1) {
			conflicts.extend(entries.iter().map(|(index, _)| *index));
		}
	}
	for &index in &conflicts {
		sources[index].state = Availability::Unavailable;
	}
	drop(identities);
	let mut selected = BTreeMap::new();
	for (index, entries) in rows.into_iter().enumerate() {
		if !conflicts.contains(&index) {
			for row in entries {
				selected.insert(row.reference.clone(), row);
			}
		}
	}
	let exceeded = Instant::now() >= deadline;
	let mut prepared = Prepared {
		schema: "cartridge-context/v1".into(),
		revision: "0".repeat(64),
		rows: Vec::new(),
		complete: !exceeded
			&& sources.iter().all(|s| {
				matches!(
					s.state,
					Availability::Available | Availability::Disabled | Availability::Empty
				)
			}),
		truncated: producer_truncated,
		deadline_exceeded: exceeded,
		sources,
		limits,
	};
	// Reserve three bytes for any false/true flag length changes. Serialize each
	// candidate only once instead of repeatedly serializing an oversized result.
	let mut encoded_bytes = serde_json::to_vec(&prepared)
		.expect("serializable context")
		.len() + 3;
	if encoded_bytes > prepared.limits.max_bytes {
		return Err("context metadata exceeds byte budget".into());
	}
	for row in selected.into_values() {
		let size = serde_json::to_vec(&row)
			.expect("serializable evidence")
			.len() + usize::from(!prepared.rows.is_empty());
		if prepared.rows.len() == prepared.limits.max_rows
			|| size > prepared.limits.max_bytes - encoded_bytes
		{
			prepared.truncated = true;
			break;
		}
		encoded_bytes += size;
		prepared.rows.push(row);
	}
	prepared.complete &= !prepared.truncated;
	prepared.seal();
	if Instant::now() >= deadline && !prepared.deadline_exceeded {
		prepared.deadline_exceeded = true;
		prepared.complete = false;
		prepared.seal();
	}
	Ok(prepared)
}

#[cfg(test)]
#[path = "../.cartridge/tests/unit/src/fabric/evidence_tests.rs"]
mod tests;
