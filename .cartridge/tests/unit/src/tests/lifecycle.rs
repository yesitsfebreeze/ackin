use std::sync::Arc;
use std::time::Duration;

use crate::runtime::{Component, Ctx, Disposer, Error, Runtime, State, Value};
use futures::StreamExt;
use parking_lot::Mutex;

type Log<T> = Arc<Mutex<Vec<T>>>;

fn log<T>() -> Log<T> {
	Arc::new(Mutex::new(Vec::new()))
}

fn record(log: &Log<String>, s: &str) -> Disposer {
	let log = log.clone();
	let s = s.to_string();
	Box::new(move || Box::pin(async move { log.lock().push(s) }))
}

fn sync(
	name: &str,
	f: impl Fn(Ctx) -> Result<Vec<Disposer>, Error> + Send + Sync + 'static,
) -> Component {
	Component::new(
		name,
		Arc::new(move |ctx| match f(ctx) {
			Ok(ds) => futures::stream::iter(ds.into_iter().map(Ok)).boxed(),
			Err(e) => futures::stream::once(async move { Err(e) }).boxed(),
		}),
	)
}

fn value<T: Send + Sync + 'static>(v: T) -> Value {
	Arc::new(v)
}

/// Deliberately shorter than `super::settle`: `a_target_change_stops_the_iterator_at_the_boundary`
/// must observe a fiber mid-stream, before its 60ms step completes.
async fn settle() {
	tokio::time::sleep(Duration::from_millis(30)).await;
}

/// A component whose whole job is to publish `key`.
fn provider(name: &str, key: &'static str, v: Value) -> Component {
	sync(name, move |ctx| ctx.provide(key, v.clone()).map(|_| vec![])).provide([key])
}

/// A component that injects `key` and records each value it binds to.
fn watcher<T: Copy + Send + Sync + 'static>(
	name: &str,
	key: &'static str,
	seen: Log<T>,
) -> Component {
	sync(name, move |ctx| {
		seen.lock()
			.push(*ctx.get(key)?.downcast_ref::<T>().unwrap());
		Ok(vec![])
	})
	.inject([key])
}

#[tokio::test(flavor = "multi_thread")]
async fn effects_revert_in_lifo_order() {
	let rt = Runtime::new();
	let log = log();
	let l = log.clone();
	let f = rt.ctx().cartridge(sync("three", move |_| {
		Ok(vec![record(&l, "1"), record(&l, "2"), record(&l, "3")])
	}));
	f.settled().await;
	assert_eq!(f.state(), Some(State::Active));
	f.dispose().await;
	assert_eq!(*log.lock(), vec!["3", "2", "1"]);
	assert!(rt.fibers().iter().all(|i| i.uid != f.uid()));
}

#[tokio::test(flavor = "multi_thread")]
async fn dependents_are_drained_before_any_provider_inverse() {
	let rt = Runtime::new();
	let log = log();
	let l = log.clone();
	let provider = rt.ctx().cartridge(
		sync("db", move |ctx| {
			ctx.provide("db", value(1))?;
			Ok(vec![record(&l, "db closed")])
		})
		.provide(["db"]),
	);
	let l = log.clone();
	let consumer = rt.ctx().cartridge(
		sync("app", move |ctx| {
			assert!(ctx.get("db").is_ok());
			let l = l.clone();
			Ok(vec![Box::new(move || {
				Box::pin(async move {
					tokio::time::sleep(Duration::from_millis(40)).await;
					l.lock().push("app cleaned up".into());
				})
			})])
		})
		.inject(["db"]),
	);
	settle().await;
	assert_eq!(consumer.state(), Some(State::Active));
	provider.dispose().await;
	assert_eq!(*log.lock(), vec!["app cleaned up", "db closed"]);
	assert_eq!(consumer.state(), Some(State::Inactive));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_replaced_provider_reloads_its_dependents() {
	let rt = Runtime::new();
	let seen: Log<i32> = log();
	let consumer = rt.ctx().cartridge(watcher("app", "cfg", seen.clone()));
	settle().await;
	assert_eq!(consumer.state(), Some(State::Inactive));
	let p1 = rt.ctx().cartridge(provider("cfg1", "cfg", value(1)));
	settle().await;
	assert_eq!(consumer.state(), Some(State::Active));
	p1.dispose().await;
	settle().await;
	assert_eq!(consumer.state(), Some(State::Inactive));
	rt.ctx().cartridge(provider("cfg2", "cfg", value(2)));
	settle().await;
	assert_eq!(*seen.lock(), vec![1, 2]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cycle_stays_inactive() {
	let rt = Runtime::new();
	let a = rt
		.ctx()
		.cartridge(provider("a", "a", value(())).inject(["b"]));
	let b = rt
		.ctx()
		.cartridge(provider("b", "b", value(())).inject(["a"]));
	settle().await;
	assert_eq!(a.state(), Some(State::Inactive));
	assert_eq!(b.state(), Some(State::Inactive));
}

#[tokio::test(flavor = "multi_thread")]
async fn isolated_realms_bind_independently() {
	let rt = Runtime::new();
	let left = rt.ctx().isolate("k");
	let right = rt.ctx().isolate("k");
	left.cartridge(provider("l", "k", value("left")));
	right.cartridge(provider("r", "k", value("right")));
	let seen: Log<&str> = log();
	left.cartridge(watcher("lc", "k", seen.clone()));
	right.cartridge(watcher("rc", "k", seen.clone()));
	let shared = rt.ctx().cartridge(sync("sc", |_| Ok(vec![])).inject(["k"]));
	settle().await;
	let mut got = seen.lock().clone();
	got.sort();
	assert_eq!(got, vec!["left", "right"]);
	assert_eq!(shared.state(), Some(State::Inactive));
}

#[tokio::test(flavor = "multi_thread")]
async fn access_is_enforced_at_the_point_of_use() {
	let rt = Runtime::new();
	assert_eq!(
		rt.ctx().get("nothing").unwrap_err(),
		Error::Undeclared("nothing".into())
	);
	rt.ctx().cartridge(provider("p", "k", value(7)));
	let seen = log();
	let s = seen.clone();
	rt.ctx().cartridge(
		sync("c", move |ctx| {
			let s = s.clone();
			ctx.cartridge(sync("child", move |ctx| {
				s.lock().push(*ctx.get("k")?.downcast_ref::<i32>().unwrap());
				assert_eq!(
					ctx.get("other").unwrap_err(),
					Error::Undeclared("other".into())
				);
				Ok(vec![])
			}));
			Ok(vec![])
		})
		.inject(["k"]),
	);
	settle().await;
	assert_eq!(*seen.lock(), vec![7]);
	let bare = rt
		.ctx()
		.cartridge(sync("bare", |ctx| ctx.get("k").map(|_| vec![])));
	settle().await;
	assert_eq!(bare.state(), Some(State::Failed));
	assert_eq!(bare.error().as_deref(), Some("undeclared access to `k`"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_target_change_stops_the_iterator_at_the_boundary() {
	let rt = Runtime::new();
	let log = log();
	let provider = rt.ctx().cartridge(provider("p", "k", value(())));
	let l = log.clone();
	let consumer = rt.ctx().cartridge(
		Component::new(
			"slow",
			Arc::new(move |_| {
				let l = l.clone();
				futures::stream::unfold(0, move |step| {
					let l = l.clone();
					async move {
						if step == 3 {
							return None;
						}
						if step == 1 {
							tokio::time::sleep(Duration::from_millis(60)).await;
						}
						l.lock().push(format!("step {step}"));
						Some((Ok(record(&l, &format!("undo {step}"))), step + 1))
					}
				})
				.boxed()
			}),
		)
		.inject(["k"]),
	);
	settle().await;
	assert_eq!(consumer.state(), Some(State::Loading));
	provider.dispose().await;
	consumer.settled().await;
	assert_eq!(consumer.state(), Some(State::Inactive));
	assert_eq!(*log.lock(), vec!["step 0", "step 1", "undo 1", "undo 0"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_raising_step_fails_the_fiber_with_nothing_installed() {
	let rt = Runtime::new();
	let log = log();
	let l = log.clone();
	let attempts = Arc::new(Mutex::new(0));
	let a = attempts.clone();
	let f = rt.ctx().cartridge(sync("flaky", move |ctx| {
		*a.lock() += 1;
		if *a.lock() == 1 {
			let l = l.clone();
			ctx.effect_sync(move || record(&l, "undone"));
			return Err(ctx.error("port busy"));
		}
		Ok(vec![])
	}));
	f.settled().await;
	assert_eq!(f.state(), Some(State::Failed));
	assert_eq!(f.error().as_deref(), Some("port busy"));
	assert_eq!(*log.lock(), vec!["undone"]);
	f.retry().await;
	assert_eq!(f.state(), Some(State::Active));
}

#[tokio::test(flavor = "multi_thread")]
async fn disposing_a_parent_retires_its_children() {
	let rt = Runtime::new();
	let log = log();
	let l = log.clone();
	let parent = rt.ctx().cartridge(sync("parent", move |ctx| {
		let inner = l.clone();
		ctx.cartridge(sync("child", move |_| {
			Ok(vec![record(&inner, "child undone")])
		}));
		Ok(vec![record(&l, "parent undone")])
	}));
	settle().await;
	assert_eq!(rt.fibers().len(), 3);
	parent.dispose().await;
	assert_eq!(*log.lock(), vec!["parent undone", "child undone"]);
	assert_eq!(rt.fibers().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn listeners_fire_in_order_and_bail_stops() {
	let rt = Runtime::new();
	let log: Log<String> = log();
	let l1 = log.clone();
	let l2 = log.clone();
	let f = rt.ctx().cartridge(sync("ears", move |ctx| {
		let l = l1.clone();
		ctx.on(
			"ping",
			Arc::new(move |_| {
				let l = l.clone();
				Box::pin(async move {
					l.lock().push("first".into());
					Ok(None)
				})
			}),
		);
		let l = l2.clone();
		ctx.on(
			"ping",
			Arc::new(move |_| {
				let l = l.clone();
				Box::pin(async move {
					l.lock().push("second".into());
					Ok(Some(value("answer")))
				})
			}),
		);
		ctx.on(
			"ping",
			Arc::new(|_| Box::pin(async { panic!("never reached by bail") })),
		);
		Ok(vec![])
	}));
	f.settled().await;
	let answer = rt.ctx().bail("ping", value(())).await.unwrap().unwrap();
	assert_eq!(*answer.downcast_ref::<&str>().unwrap(), "answer");
	assert_eq!(*log.lock(), vec!["first", "second"]);
	f.dispose().await;
	assert!(rt.ctx().bail("ping", value(())).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn user_values_and_callbacks_are_dropped_outside_the_registry_lock() {
	struct Probe {
		runtime: std::sync::Weak<Runtime>,
		observed: Log<bool>,
	}
	impl Drop for Probe {
		fn drop(&mut self) {
			// A bare `try_lock` miss proves nothing: a disposer's own notify wakes
			// tasks that take the registry, so an unrelated holder is ordinary
			// contention. Reentrancy is what this asserts against, and it never
			// clears — waiting separates the two.
			let locked = self.runtime.upgrade().is_some_and(|rt| {
				rt.reg
					.try_lock_for(std::time::Duration::from_millis(250))
					.is_none()
			});
			self.observed.lock().push(locked);
		}
	}
	let rt = Runtime::new();
	let observed = log();
	let weak = Arc::downgrade(&rt);
	let record = observed.clone();
	let owner = Probe {
		runtime: weak.clone(),
		observed: record.clone(),
	};
	let fiber = rt.ctx().cartridge(
		sync("drop-probes", move |ctx| {
			let _keep_owner = &owner;
			ctx.provide(
				"probe",
				Arc::new(Probe {
					runtime: weak.clone(),
					observed: record.clone(),
				}),
			)?;
			let callback = Arc::new(Probe {
				runtime: weak.clone(),
				observed: record.clone(),
			});
			ctx.on(
				"unused",
				Arc::new(move |_| {
					let held = callback.clone();
					Box::pin(async move {
						drop(held);
						Ok(None)
					})
				}),
			);
			Ok(vec![])
		})
		.provide(["probe"]),
	);
	fiber.settled().await;
	fiber.dispose().await;
	tokio::time::timeout(Duration::from_secs(2), async {
		while observed.lock().len() < 3 {
			tokio::task::yield_now().await;
		}
	})
	.await
	.unwrap();
	assert_eq!(*observed.lock(), vec![false, false, false]);
}

#[tokio::test]
async fn disposing_an_apply_that_never_yields_finishes_and_runs_prior_inverses() {
	let rt = Runtime::new();
	let log = log();
	let held = log.clone();
	let (started, mut seen) = tokio::sync::mpsc::unbounded_channel();
	let fiber = rt.ctx().cartridge(Component::new(
		"stalled",
		Arc::new(move |ctx| {
			let held = held.clone();
			ctx.effect_sync(move || record(&held, "released"));
			let _ = started.send(());
			futures::stream::pending().boxed()
		}),
	));
	seen.recv().await.unwrap();
	tokio::time::timeout(Duration::from_secs(1), fiber.dispose())
		.await
		.unwrap();
	assert_eq!(*log.lock(), ["released"]);
	assert!(fiber.state().is_none());
}

#[tokio::test]
async fn disposing_an_effect_that_never_yields_finishes() {
	let rt = Runtime::new();
	let effect = rt.ctx().effect(futures::stream::pending());
	tokio::time::timeout(Duration::from_secs(1), effect.dispose())
		.await
		.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn gather_keeps_every_answer_with_its_fiber() {
	let rt = Runtime::new();
	// Two cartridges answer, one stays silent, one fails: the announce keeps
	// what it was told and the failure stays with the cartridge that broke.
	let speaker = |answer: &'static str| {
		move |ctx: Ctx| {
			ctx.on(
				"announce",
				Arc::new(move |_| Box::pin(async move { Ok(Some(value(answer))) })),
			);
			Ok(vec![])
		}
	};
	let first = rt.ctx().cartridge(sync("first", speaker("one")));
	let second = rt.ctx().cartridge(sync("second", speaker("two")));
	let silent = rt.ctx().cartridge(sync("silent", |ctx: Ctx| {
		ctx.on("announce", Arc::new(|_| Box::pin(async { Ok(None) })));
		Ok(vec![])
	}));
	let broken = rt.ctx().cartridge(sync("broken", |ctx: Ctx| {
		ctx.on(
			"announce",
			Arc::new(|_| Box::pin(async { Err(Error::Apply("no answer".into())) })),
		);
		Ok(vec![])
	}));
	for f in [&first, &second, &silent, &broken] {
		f.settled().await;
	}

	let gathered = rt.ctx().gather("announce", value(())).await;
	// Sorted, not asserted in arrival order: two cartridges register their
	// listeners concurrently, so which one lands first is not a contract.
	let mut answers: Vec<(u64, &str)> = gathered
		.iter()
		.map(|(uid, v)| (*uid, *v.downcast_ref::<&str>().unwrap()))
		.collect();
	answers.sort();
	assert_eq!(
		answers,
		vec![(first.uid(), "one"), (second.uid(), "two")],
		"only the answering fibers contribute, each under its own uid"
	);
	settle().await;
	assert_eq!(broken.state(), Some(State::Failed));
	assert_eq!(first.state(), Some(State::Active));

	first.dispose().await;
	let gathered = rt.ctx().gather("announce", value(())).await;
	assert_eq!(
		gathered.len(),
		1,
		"a disposed cartridge stops contributing to the graph"
	);
}
