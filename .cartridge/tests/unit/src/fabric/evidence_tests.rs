use super::*;
use std::sync::{
	atomic::{AtomicUsize, Ordering},
	Arc,
};

fn evidence(id: &str, body: &str) -> Evidence {
	Evidence {
		reference: Reference {
			owner: "fixture".into(),
			kind: "document".into(),
			id: id.into(),
			revision: format!("{:x}", Sha256::digest(body.as_bytes())),
			revision_kind: "source_bytes".into(),
		},
		source: format!("@fixture/{id}"),
		text: body.into(),
		selection_reason: "exact_reference".into(),
	}
}
fn task(name: &str, state: Availability, entries: Vec<Evidence>) -> Task<'static> {
	let contributor = name.to_owned();
	let result = Contribution {
		truncated: false,
		contributor: contributor.clone(),
		state: Availability::Available,
		candidates: entries
			.into_iter()
			.map(|evidence| Candidate {
				evidence,
				private: false,
			})
			.collect(),
	};
	Task {
		contributor,
		state,
		run: Box::pin(async move { Ok(result) }),
	}
}

#[tokio::test]
async fn canonical_order_exact_readback_and_conflicting_sources() {
	let first = collect(
		vec![
			task("b", Availability::Available, vec![evidence("z", "z")]),
			task("a", Availability::Available, vec![evidence("a", "a")]),
		],
		Limits::default(),
	)
	.await
	.unwrap();
	let second = collect(
		vec![
			task("a", Availability::Available, vec![evidence("a", "a")]),
			task("b", Availability::Available, vec![evidence("z", "z")]),
		],
		Limits::default(),
	)
	.await
	.unwrap();
	assert_eq!(first, second);
	assert!(first.complete);
	assert_eq!(first.rows[0].reference.id, "a");
	assert_eq!(first.read(&first.rows[0].reference), Some(&first.rows[0]));
	let mut stale = first.rows[0].reference.clone();
	stale.revision = "0".repeat(64);
	assert!(first.read(&stale).is_none());
	let mut forged = evidence("a", "a");
	forged.text = "contradiction".into();
	let conflict = collect(
		vec![
			task("a", Availability::Available, vec![evidence("a", "a")]),
			task("b", Availability::Available, vec![forged]),
		],
		Limits::default(),
	)
	.await
	.unwrap();
	assert!(conflict.rows.is_empty());
	assert!(conflict
		.sources
		.iter()
		.all(|s| s.state == Availability::Unavailable));
	let changed = collect(
		vec![task(
			"a",
			Availability::Available,
			vec![evidence("a", "changed")],
		)],
		Limits::default(),
	)
	.await
	.unwrap();
	assert_ne!(first.revision, changed.revision);
}

#[tokio::test]
async fn private_content_cannot_change_public_digest_or_capacity() {
	let public = evidence("public", "Visible");
	let baseline = collect(
		vec![task("docs", Availability::Available, vec![public.clone()])],
		Limits::default(),
	)
	.await
	.unwrap();
	let mut private = evidence("PRIVATE-ID", "PRIVATE-BODY");
	private.source = "PRIVATE-METADATA".into();
	private.reference.revision = "PRIVATE-DIGEST".into();
	let mut candidates = vec![Candidate {
		evidence: public,
		private: false,
	}];
	candidates.extend((0..200).map(|_| Candidate {
		evidence: private.clone(),
		private: true,
	}));
	let t = Task {
		contributor: "docs".into(),
		state: Availability::Available,
		run: Box::pin(async move {
			Ok(Contribution {
				truncated: false,
				contributor: "docs".into(),
				state: Availability::Available,
				candidates,
			})
		}),
	};
	let value = collect(vec![t], Limits::default()).await.unwrap();
	assert_eq!(value, baseline);
	assert!(!serde_json::to_string(&value).unwrap().contains("PRIVATE"));
}

struct Dropped(Arc<AtomicUsize>);
impl Drop for Dropped {
	fn drop(&mut self) {
		self.0.fetch_add(1, Ordering::SeqCst);
	}
}

#[tokio::test(start_paused = true)]
async fn hanging_first_source_does_not_starve_siblings_and_is_dropped() {
	let dropped = Arc::new(AtomicUsize::new(0));
	let mark = dropped.clone();
	let hanging = Task {
		contributor: "a_hanging".into(),
		state: Availability::Available,
		run: Box::pin(async move {
			let _guard = Dropped(mark);
			std::future::pending().await
		}),
	};
	let began = Instant::now();
	let prepared = collect(
		vec![
			hanging,
			task(
				"b_ready",
				Availability::Available,
				vec![evidence("one", "useful")],
			),
		],
		Limits {
			deadline_ms: 20,
			..Limits::default()
		},
	)
	.await
	.unwrap();
	assert_eq!(Instant::now() - began, Duration::from_millis(20));
	assert_eq!(dropped.load(Ordering::SeqCst), 1);
	assert_eq!(prepared.sources[0].state, Availability::Timeout);
	assert_eq!(prepared.rows[0].text, "useful");
	assert!(!prepared.complete);
}

#[tokio::test]
async fn availability_is_explicit_and_disabled_producers_never_poll() {
	let calls = Arc::new(AtomicUsize::new(0));
	let count = calls.clone();
	let disabled = Task {
		contributor: "disabled".into(),
		state: Availability::Disabled,
		run: Box::pin(async move {
			count.fetch_add(1, Ordering::SeqCst);
			Err("must not poll".into())
		}),
	};
	let failed = Task {
		contributor: "failed".into(),
		state: Availability::Available,
		run: Box::pin(async { Err("SECRET backend diagnostic".into()) }),
	};
	let result = collect(
		vec![
			disabled,
			failed,
			task("absent", Availability::Absent, vec![]),
			task("empty", Availability::Available, vec![]),
		],
		Limits::default(),
	)
	.await
	.unwrap();
	assert_eq!(calls.load(Ordering::SeqCst), 0);
	assert_eq!(
		result.sources.iter().map(|s| s.state).collect::<Vec<_>>(),
		vec![
			Availability::Absent,
			Availability::Disabled,
			Availability::Empty,
			Availability::Unavailable
		]
	);
	assert!(!serde_json::to_string(&result).unwrap().contains("SECRET"));
}

#[tokio::test]
async fn exact_serialized_bounds_count_metadata_and_keep_whole_utf8_rows() {
	let entries = (0..70)
		.map(|i| evidence(&format!("{i:03}"), &"é".repeat(1000)))
		.collect();
	let result = collect(
		vec![task("docs", Availability::Available, entries)],
		Limits {
			max_rows: 3,
			max_bytes: 4096,
			..Limits::default()
		},
	)
	.await
	.unwrap();
	let encoded = serde_json::to_vec(&result).unwrap();
	assert!(encoded.len() <= 4096);
	assert!(result.rows.len() <= 3 && !result.rows.is_empty());
	assert!(result.truncated && !result.complete);
	assert!(result.rows.iter().all(|r| r.text == "é".repeat(1000)));
	let bad = collect(
		vec![task(
			"docs",
			Availability::Available,
			vec![evidence("bad", &"x".repeat(8193))],
		)],
		Limits::default(),
	)
	.await
	.unwrap();
	assert_eq!(bad.sources[0].state, Availability::Unavailable);
	let excessive = collect(
		vec![task(
			"docs",
			Availability::Available,
			(0..129).map(|i| evidence(&i.to_string(), "x")).collect(),
		)],
		Limits::default(),
	)
	.await
	.unwrap();
	assert!(excessive.rows.is_empty());
	assert_eq!(excessive.sources[0].state, Availability::Unavailable);
}

#[tokio::test]
async fn duplicate_tasks_and_invalid_limits_fail_before_any_producer() {
	let calls = Arc::new(AtomicUsize::new(0));
	let make = || {
		let calls = calls.clone();
		Task {
			contributor: "same".into(),
			state: Availability::Available,
			run: Box::pin(async move {
				calls.fetch_add(1, Ordering::SeqCst);
				Err("called".into())
			}),
		}
	};
	assert!(collect(vec![make(), make()], Limits::default())
		.await
		.is_err());
	// Zero, not merely small: how large a budget may be is declared by the
	// cartridge carrying the keys and checked by its host, so what fails here
	// is a budget that is no budget at all.
	assert!(collect(
		vec![make()],
		Limits {
			max_bytes: 0,
			..Limits::default()
		}
	)
	.await
	.is_err());
	assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn producer_capacity_omissions_remain_explicit_in_prepared_snapshot() {
	let task = Task {
		contributor: "memory".into(),
		state: Availability::Available,
		run: Box::pin(async {
			Ok(Contribution {
				contributor: "memory".into(),
				state: Availability::Available,
				truncated: true,
				candidates: vec![Candidate {
					evidence: evidence("one", "retained"),
					private: false,
				}],
			})
		}),
	};
	let value = collect(vec![task], Limits::default()).await.unwrap();
	assert_eq!(value.rows.len(), 1);
	assert_eq!(value.sources[0].state, Availability::Partial);
	assert!(value.truncated && !value.complete);
}

#[tokio::test(start_paused = true)]
async fn late_ready_contribution_cannot_publish_after_deadline() {
	let task = Task {
		contributor: "late".into(),
		state: Availability::Available,
		run: Box::pin(async {
			tokio::time::advance(Duration::from_millis(10)).await;
			Ok(Contribution {
				contributor: "late".into(),
				state: Availability::Available,
				truncated: false,
				candidates: vec![Candidate {
					evidence: evidence("late", "late"),
					private: false,
				}],
			})
		}),
	};
	let value = collect(
		vec![task],
		Limits {
			deadline_ms: 5,
			..Limits::default()
		},
	)
	.await
	.unwrap();
	assert!(value.rows.is_empty());
	assert_eq!(value.sources[0].state, Availability::Timeout);
	assert!(value.deadline_exceeded && !value.complete);
}

#[tokio::test]
async fn dropping_collector_drops_pending_producers_without_spawning_work() {
	let dropped = Arc::new(AtomicUsize::new(0));
	let mark = dropped.clone();
	let t = Task {
		contributor: "pending2".into(),
		state: Availability::Available,
		run: Box::pin(async move {
			let _guard = Dropped(mark);
			std::future::pending().await
		}),
	};
	let mut boxed = Box::pin(collect(vec![t], Limits::default()));
	assert!(futures::poll!(&mut boxed).is_pending());
	drop(boxed);
	assert_eq!(dropped.load(Ordering::SeqCst), 1);
}
