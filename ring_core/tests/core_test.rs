//! `ring_core`, the composition point over three backends.
//!
//! # What this file is arranged around
//!
//! This file carries the reached-test for
//! `docs/feature/187_optional_crossbeam_queue_backend.md`, whose condition is
//! that "the swap between them is a **build flag rather than a rewrite**". A
//! claim of that shape is only as good as a test that runs the
//! *same program* against every backend. So the reached-test
//! ([`the_same_program_behaves_identically_on_every_backend`]) is written once
//! and parameterized over backends, rather than written three times.
//!
//! That shape has a failure mode of its own, and it is the one to watch. A
//! parameterized test that silently runs against one backend proves nothing
//! about the other two while still reporting green.
//! [`every_backend_the_build_offers_is_actually_exercised`] exists to make that
//! visible, and the crossbeam arm is `#[ cfg( feature = "crossbeam" ) ]`
//! throughout. Under the default build there are two backends, not three, and
//! the test says so rather than pretending.
//!
//! # What is deliberately not here
//!
//! No `is_closed`, and no test for it. Liveness is `ring_shutdown`'s, and a
//! handle-local copy of the flag is the failure that crate exists to prevent.
//! So this crate has no flag to test.
//!
//! No `loom` model. This crate contains no atomic of its own. Every ordering
//! question belongs to a backend, and each backend models its own
//! (`ring_spsc` and `ring_mpsc` both do). A model here would re-check theirs
//! while proving nothing about the composition, which is what this file is for.

#![cfg(test)]
// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure, so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use ring_config::RingConfig;
use ring_core::{Backend, Ring};
use ring_types::{OverflowPolicy, RingError};

/// The backends this build has.
///
/// Written twice under a `cfg` rather than once with a conditional `push`. The
/// list is what every test below loops over, so it should read as a literal
/// for each build rather than be assembled. It also avoids an
/// `unused_mut` that would otherwise appear only in the default build.
#[cfg(feature = "crossbeam")]
fn every_backend() -> Vec<Backend> {
  vec![Backend::Spsc, Backend::Mpsc, Backend::Crossbeam]
}

/// The backends the default build has, without crossbeam.
#[cfg(not(feature = "crossbeam"))]
fn every_backend() -> Vec<Backend> {
  vec![Backend::Spsc, Backend::Mpsc]
}

/// Build a ring on a named backend, with `slots` capacity and `overflow`.
fn ring_on<T: Send>(backend: Backend, slots: usize, overflow: OverflowPolicy) -> Result<Ring<T>, RingError> {
  let base = RingConfig::new(slots).expect("a power of two").with_overflow(overflow);

  match backend {
    Backend::Spsc => Ring::new(&base),
    Backend::Mpsc => Ring::new(&base.with_producers(4)),
    #[cfg(feature = "crossbeam")]
    Backend::Crossbeam => Ring::new_crossbeam(&base),
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// The optional crossbeam backend's reached-test.
// ─────────────────────────────────────────────────────────────────────────────

/// The same program, run against every backend, produces the same result.
///
/// This is `docs/feature/187_optional_crossbeam_queue_backend.md`'s condition
/// stated as a test. If swapping the backend is a build flag rather than a
/// rewrite, then a program written against this API cannot tell which
/// backend it got, for every behaviour the API promises.
///
/// The qualifier matters. Three behaviours are deliberately excluded because
/// the API does *not* promise them uniformly, and asserting them would be
/// asserting something false:
///
/// - **Drain order** across multiple producers. It is publication order at
///   SPSC, an interleaving at MPSC, and unspecified at crossbeam. The
///   assertion below sorts.
/// - **`free_capacity`'s exactness.** Binding at SPSC, advisory elsewhere.
/// - **`DropOldest`.** Only crossbeam can evict.
#[test]
fn the_same_program_behaves_identically_on_every_backend() {
  const CAPACITY: usize = 8;
  const RECORDS: u32 = 200;

  for backend in every_backend() {
    let mut ring: Ring<u32> = ring_on(backend, CAPACITY, OverflowPolicy::Fail).expect("Fail is supported everywhere");

    assert_eq!(ring.backend(), backend, "the requested backend was not the one built");
    assert_eq!(ring.capacity().get(), CAPACITY, "{backend:?} disagreed about capacity");

    let mut ends = ring.ends();
    let (mut producer, mut consumer) = ends.split();

    assert!(consumer.is_empty(), "{backend:?} started non-empty");
    assert_eq!(consumer.try_recv(), None, "{backend:?} yielded a record from an empty ring");

    // Push more than capacity, draining as we go, so the ring wraps ~25 times.
    let mut received = Vec::with_capacity(RECORDS as usize);
    let mut offered = 0_u32;

    while offered < RECORDS {
      if producer.try_push(offered).is_ok() {
        offered += 1;
      } else {
        // Refused means full, and full means draining must make room. If it
        // does not, the ring has lost the ability to release slots and this
        // loop would spin forever. So the drain is asserted, not assumed.
        assert!(
          consumer.try_recv_batch(&mut received) > 0,
          "{backend:?} refused a push while the consumer could drain nothing — deadlock"
        );
      }
    }

    let before = received.len();
    let drained = consumer.try_recv_batch(&mut received);

    assert_eq!(
      received.len() - before,
      drained,
      "{backend:?} reported a drained count its own append contradicts"
    );
    assert_eq!(received.len(), RECORDS as usize, "{backend:?} lost or duplicated records");

    // Sorted, because drain order is exactly what the API does not promise.
    received.sort_unstable();
    assert_eq!(
      received,
      (0..RECORDS).collect::<Vec<_>>(),
      "{backend:?} did not deliver every record exactly once"
    );

    assert!(consumer.is_empty(), "{backend:?} still reports records after a full drain");
  }
}

/// Every backend the build offers is reached by the reached-test.
///
/// The reached-test loops over [`every_backend`]. If that helper returned one
/// entry, the loop would pass while proving a third of what it claims. This
/// test pins the count to the build's feature set.
#[test]
fn every_backend_the_build_offers_is_actually_exercised() {
  let backends = every_backend();

  #[cfg(feature = "crossbeam")]
  assert_eq!(backends.len(), 3, "the crossbeam build must offer three backends");
  #[cfg(not(feature = "crossbeam"))]
  assert_eq!(backends.len(), 2, "the default build offers exactly two backends");

  // Distinct, so a copy-paste in the helper cannot inflate the count.
  let mut seen = backends.clone();
  seen.sort_by_key(|backend| format!("{backend:?}"));
  seen.dedup();
  assert_eq!(seen.len(), backends.len(), "a backend was listed twice");
}

// ─────────────────────────────────────────────────────────────────────────────
// Backend selection.
// ─────────────────────────────────────────────────────────────────────────────

/// The producer count in the configuration is what picks the in-house backend.
#[test]
fn the_producer_count_selects_between_spsc_and_mpsc() {
  let one = RingConfig::new(8).unwrap();
  assert!(!one.is_multi_producer());
  assert_eq!(Ring::<u8>::new(&one).unwrap().backend(), Backend::Spsc);

  for producers in 2..=8 {
    let many = RingConfig::new(8).unwrap().with_producers(producers);
    assert_eq!(
      Ring::<u8>::new(&many).unwrap().backend(),
      Backend::Mpsc,
      "{producers} producers must select the multi-producer backend"
    );
  }
}

/// Every backend hands back the capacity it was configured with.
///
/// Fix(crossbeam_capacity_was_revalidated_in_an_accessor): the crossbeam arm of
/// `Ring::capacity` used to rebuild a `Capacity` from `ArrayQueue::capacity`
/// and `expect` the result, which put this crate's only fallible validation
/// inside an infallible accessor. The variant now carries the validated value,
/// and this test is what holds the three backends to one answer rather than a
/// comment claiming they agree.
///
/// The backends come from [`every_backend`] and only the crossbeam *arm* is
/// gated, which is the point. This test used to carry
/// `#[ cfg( feature = "crossbeam" ) ]` on the function, a gate sized to its
/// widest arm rather than to the property it asserts. `default = []`, so that
/// gate deleted the `Spsc` and `Mpsc` assertions from the build most consumers
/// get. The sentence above claimed three backends were held to one answer
/// while, in the default build, none of them were. A `cfg` belongs on the code
/// that cannot compile without the feature, never on the general property that
/// code happens to be one case of.
#[test]
fn every_backend_reports_the_capacity_it_was_configured_with() {
  for requested in [1_usize, 2, 8, 1024] {
    let config = RingConfig::new(requested).unwrap();
    for backend in every_backend() {
      let ring: Ring<u8> = match backend {
        Backend::Spsc => Ring::new(&config.with_producers(1)),
        Backend::Mpsc => Ring::new(&config.with_producers(2)),
        #[cfg(feature = "crossbeam")]
        Backend::Crossbeam => Ring::new_crossbeam(&config),
      }
      .unwrap();

      assert_eq!(
        ring.capacity().get(),
        requested,
        "{backend:?} reported a capacity other than the one it was built from"
      );
    }
  }
}

/// The crossbeam backend is chosen by its constructor, not by the config.
///
/// The optional-backend feature calls the swap "a build flag rather than a
/// rewrite". A config field would make it a *runtime* choice, which would mean
/// shipping crossbeam in every build that might want it. That is the opposite
/// of an optional dependency.
#[cfg(feature = "crossbeam")]
#[test]
fn the_crossbeam_backend_ignores_the_producer_count() {
  for producers in [1, 4] {
    let config = RingConfig::new(8).unwrap().with_producers(producers);
    assert_eq!(
      Ring::<u8>::new_crossbeam(&config).unwrap().backend(),
      Backend::Crossbeam,
      "the crossbeam constructor deferred to the config"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Producer cardinality.
// ─────────────────────────────────────────────────────────────────────────────

/// `try_clone` reports the backend's cardinality rather than assuming it.
///
/// This is the API's only machine-checkable way to tell SPSC from the
/// others, which makes it the only way a caller can learn whether
/// `free_capacity` is binding.
#[test]
fn try_clone_refuses_at_spsc_and_permits_elsewhere() {
  for backend in every_backend() {
    let mut ring: Ring<u8> = ring_on(backend, 8, OverflowPolicy::Fail).unwrap();
    let mut ends = ring.ends();
    let (producer, _consumer) = ends.split();

    let cloned = producer.try_clone();

    match backend {
      Backend::Spsc => assert!(cloned.is_none(), "SPSC handed out a second producer"),
      _ => assert!(cloned.is_some(), "{backend:?} refused a second producer"),
    }
  }
}

/// Cloned producers write into one ring, and nothing is lost between them.
#[test]
fn cloned_producers_share_one_ring() {
  for backend in every_backend().into_iter().filter(|b| *b != Backend::Spsc) {
    let mut ring: Ring<u32> = ring_on(backend, 8, OverflowPolicy::Fail).unwrap();
    let mut ends = ring.ends();
    let (mut first, mut consumer) = ends.split();
    let mut second = first.try_clone().expect("not SPSC");

    first.try_push(1).unwrap();
    second.try_push(2).unwrap();

    let mut received = Vec::new();
    assert_eq!(
      consumer.try_recv_batch(&mut received),
      2,
      "{backend:?} did not drain both records in one call"
    );
    received.sort_unstable();

    assert_eq!(received, [1, 2], "{backend:?} lost a record written through a clone");
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Overflow policy.
// ─────────────────────────────────────────────────────────────────────────────

/// Under `Fail`, a full ring hands the record back rather than dropping it.
///
/// **This is the case that caught a real defect.** `ring_mpsc::Producer::push`
/// takes the value by move and returns `Result< Seq, RingError >`, so on a full
/// ring the record is consumed and gone. The first implementation here called
/// it directly and could not honour `Result< (), T >`. The fix claims a slot
/// *before* consuming the record. Only the MPSC arm was ever wrong, so a test
/// covering SPSC alone would have passed throughout.
#[test]
fn a_refused_record_comes_back_on_every_backend() {
  const CAPACITY: usize = 2;

  for backend in every_backend() {
    let mut ring: Ring<u32> = ring_on(backend, CAPACITY, OverflowPolicy::Fail).unwrap();
    let mut ends = ring.ends();
    let (mut producer, mut consumer) = ends.split();

    for record in 0..CAPACITY as u32 {
      producer
        .try_push(record)
        .unwrap_or_else(|_| panic!("{backend:?} refused early"));
    }

    assert!(producer.is_full(), "{backend:?} does not report a filled ring as full");
    assert_eq!(
      producer.try_push(99),
      Err(99),
      "{backend:?} swallowed a refused record instead of returning it"
    );

    // And the refusal cost the ring nothing. What was accepted is still there.
    let mut received = Vec::new();
    assert_eq!(
      consumer.try_recv_batch(&mut received),
      2,
      "{backend:?} drained the wrong number of records after a refusal"
    );
    received.sort_unstable();
    assert_eq!(received, [0, 1], "{backend:?} lost an accepted record to a later refusal");
  }
}

/// Under `DropNewest`, a full ring discards the record and reports success.
#[test]
fn drop_newest_discards_the_incoming_record_without_an_error() {
  const CAPACITY: usize = 2;

  for backend in every_backend() {
    let mut ring: Ring<u32> = ring_on(backend, CAPACITY, OverflowPolicy::DropNewest).unwrap();
    let mut ends = ring.ends();
    let (mut producer, mut consumer) = ends.split();

    for record in 0..10 {
      assert!(producer.try_push(record).is_ok(), "{backend:?} refused under DropNewest");
    }

    let mut received = Vec::new();
    assert_eq!(
      consumer.try_recv_batch(&mut received),
      2,
      "{backend:?} held more than the capacity under DropNewest"
    );
    received.sort_unstable();

    assert_eq!(
      received,
      [0, 1],
      "{backend:?} kept the wrong records — DropNewest must discard the incoming one"
    );
  }
}

/// `DropOldest` is refused at construction by the in-house backends.
///
/// Evicting an unread record contradicts the exactly-once delivery both of them
/// guarantee, so the ring refuses to exist rather than silently behaving as
/// `DropNewest`. The failure is at construction, where a caller can still act
/// on it, rather than at a push they will not be watching.
#[test]
fn drop_oldest_is_rejected_by_the_in_house_backends() {
  for producers in [1, 4] {
    let config = RingConfig::new(8)
      .unwrap()
      .with_producers(producers)
      .with_overflow(OverflowPolicy::DropOldest);

    assert_eq!(
      Ring::<u8>::new(&config).unwrap_err(),
      RingError::PolicyUnsupported,
      "an in-house backend accepted a policy it cannot honour"
    );
  }

  // And it is a configuration error, not a traffic condition. That distinction
  // tells a caller to fix their config rather than retry.
  assert!(RingError::PolicyUnsupported.is_configuration());
  assert!(!RingError::PolicyUnsupported.is_transient());
}

/// The crossbeam backend accepts `DropOldest` and actually evicts.
///
/// The one capability the in-house rings deliberately lack. Asserting that it
/// is *accepted* is not enough. A constructor that took the policy and then
/// behaved as `DropNewest` would pass that. The contents are what separates them.
#[cfg(feature = "crossbeam")]
#[test]
fn crossbeam_honours_drop_oldest_by_evicting() {
  const CAPACITY: usize = 2;

  let config = RingConfig::new(CAPACITY).unwrap().with_overflow(OverflowPolicy::DropOldest);

  let mut ring: Ring<u32> = Ring::new_crossbeam(&config).unwrap();
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();

  for record in 0..4 {
    assert!(producer.try_push(record).is_ok(), "DropOldest must never refuse");
  }

  let mut received = Vec::new();
  assert_eq!(
    consumer.try_recv_batch(&mut received),
    2,
    "DropOldest held more than the capacity"
  );
  received.sort_unstable();

  assert_eq!(
    received,
    [2, 3],
    "DropOldest kept the oldest records — it must evict them, not the incoming ones"
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Batch shapes.
// ─────────────────────────────────────────────────────────────────────────────

/// A partial batch push reports its count and hands back the refused record.
///
/// The iterator's position is the part worth pinning. The refused record was
/// already taken from the iterator when the push failed, so the iterator
/// resumes *after* it. The record comes back in the `Err`, so the two together
/// are everything that was not published, in order.
#[test]
fn a_partial_batch_push_reports_its_count_and_hands_back_the_refused_record() {
  const CAPACITY: usize = 2;

  for backend in every_backend() {
    let mut ring: Ring<u32> = ring_on(backend, CAPACITY, OverflowPolicy::Fail).unwrap();
    let mut ends = ring.ends();
    let (mut producer, _consumer) = ends.split();

    let mut records = 0..5_u32;
    assert_eq!(
      producer.try_push_batch(&mut records),
      Err((CAPACITY, 2)),
      "{backend:?}: two went in, then record 2 was refused and handed back"
    );
    assert_eq!(
      records.next(),
      Some(3),
      "{backend:?}: the iterator resumes after the refused record"
    );
  }
}

/// A batch push into a full `Fail` ring destroys no record on any backend.
///
/// The payload counts its own drops, so a record the call swallows shows up as
/// a drop while the ring, the iterator and the call's result are all still
/// alive. The result is held in a named binding rather than `let _`, which
/// would drop a handed-back record on the spot and count it as lost.
#[test]
fn a_refused_batch_push_drops_no_record_on_every_backend() {
  use std::sync::Arc;
  use std::sync::atomic::{AtomicUsize, Ordering};

  #[derive(Debug)]
  struct Tracked(Arc<AtomicUsize>);
  impl Drop for Tracked {
    fn drop(&mut self) {
      self.0.fetch_add(1, Ordering::SeqCst);
    }
  }

  // Every backend is measured before anything is asserted, so a failure names
  // all of them rather than stopping at the first.
  let destroyed: Vec<(Backend, usize)> = every_backend()
    .into_iter()
    .map(|backend| {
      let drops = Arc::new(AtomicUsize::new(0));
      let mut ring: Ring<Tracked> = ring_on(backend, 4, OverflowPolicy::Fail).unwrap();
      let mut ends = ring.ends();
      let (mut producer, _consumer) = ends.split();

      let mut records = (0..6).map(|_| Tracked(Arc::clone(&drops)));
      let _outcome = producer.try_push_batch(&mut records);

      (backend, drops.load(Ordering::SeqCst))
    })
    .collect();

  assert!(
    destroyed.iter().all(|(_, drops)| *drops == 0),
    "a refused record was destroyed, per backend: {destroyed:?}"
  );
}

/// An empty iterator is accepted as zero rather than treated as an error.
#[test]
fn an_empty_batch_push_is_a_no_op() {
  for backend in every_backend() {
    let mut ring: Ring<u32> = ring_on(backend, 8, OverflowPolicy::Fail).unwrap();
    let mut ends = ring.ends();
    let (mut producer, consumer) = ends.split();

    let mut nothing = core::iter::empty();
    assert_eq!(producer.try_push_batch(&mut nothing), Ok(0));
    assert!(consumer.is_empty(), "{backend:?} published something from an empty iterator");
  }
}

/// `try_recv_batch` appends rather than replacing, and reports only its own count.
#[test]
fn try_recv_batch_appends_to_the_caller_s_buffer() {
  for backend in every_backend() {
    let mut ring: Ring<u32> = ring_on(backend, 8, OverflowPolicy::Fail).unwrap();
    let mut ends = ring.ends();
    let (mut producer, mut consumer) = ends.split();

    let mut out = vec![900, 901];

    producer.try_push(1).unwrap();
    producer.try_push(2).unwrap();

    let moved = consumer.try_recv_batch(&mut out);

    assert_eq!(moved, 2, "{backend:?} reported the buffer's length rather than its own count");
    assert_eq!(out.len(), 4, "{backend:?} replaced the caller's buffer instead of appending");
    out[2..].sort_unstable();
    assert_eq!(out, [900, 901, 1, 2], "{backend:?} disturbed what the buffer already held");
  }
}

/// `try_recv_batch`'s returned count matches what actually lands in the
/// caller's buffer when the batch straddles the physical wraparound boundary.
///
/// Fix(ring-core-try-recv-batch-count-unproven): confirming test, not a bug
/// reproduction.
/// Root cause: `try_recv_batch` captures `len` from `Batch::len()` *before*
/// the `filter_map` loop, then returns that captured `len` rather than the
/// number of elements the loop actually pushed into `out`. A Finding filed
/// against `ring_core`'s own bug-hunt (tracked as a follow-up) worried this
/// could diverge if any offset in the batch mapped to an already-empty slot,
/// since `TypedSlot::take()` returns `None` for those and `filter_map` drops
/// them silently.
/// Pitfall: that divergence needs a batch offset landing on an empty slot,
/// and this crate has two independent, backend-specific guarantees against
/// it. SPSC's `drain()` bounds `len` by `consumer.distance_to(producer)`,
/// where the SPSC invariant (single producer writes before publishing) makes
/// every offset in range populated by construction. MPSC's `drain_up_to()`
/// calls `contiguous_end()`, which walks forward checking each slot's own
/// per-lap stamp and stops at the *first* unwritten-or-stale slot, so a
/// claimed-but-not-yet-written slot can never enter the batch at all. Both
/// mechanisms were read at their source (`ring_spsc::Consumer::drain`,
/// `ring_mpsc::contiguous_end`) rather than assumed. The wraparound case
/// here is the one most likely to expose an off-by-one in either mechanism,
/// since it is where the physical slot index reuses a previously-occupied
/// address.
#[test]
fn try_recv_batch_count_matches_the_buffer_across_a_wraparound() {
  const CAPACITY: usize = 4;

  for backend in every_backend() {
    let mut ring: Ring<u32> = ring_on(backend, CAPACITY, OverflowPolicy::Fail).unwrap();
    let mut ends = ring.ends();
    let (mut producer, mut consumer) = ends.split();

    // First lap: publish 3, drain all 3. The consumer cursor now sits at
    // logical position 3, one short of the first wrap.
    for value in 0..3_u32 {
      producer.try_push(value).unwrap();
    }
    let mut first = Vec::new();
    assert_eq!(consumer.try_recv_batch(&mut first), 3, "{backend:?} first drain");

    // The second publish straddles the wrap. Logical position 3 is still lap 0
    // (physical index 3), and positions 4 and 5 are lap 1 (physical index 0, 1),
    // the same physical slots [0, 1] the first drain already emptied and
    // committed.
    for value in 3..6_u32 {
      producer.try_push(value).unwrap();
    }

    let mut out = Vec::new();
    let moved = consumer.try_recv_batch(&mut out);

    assert_eq!(moved, 3, "{backend:?} reported a count other than what it drained");
    assert_eq!(
      moved,
      out.len(),
      "{backend:?}: returned count diverged from the buffer's real growth"
    );
    out.sort_unstable();
    assert_eq!(
      out,
      [3, 4, 5],
      "{backend:?}: wraparound lost, duplicated, or reordered a record"
    );
  }
}

/// Draining an empty ring moves nothing and reports zero.
#[test]
fn draining_an_empty_ring_is_zero_not_an_error() {
  for backend in every_backend() {
    let mut ring: Ring<u32> = ring_on(backend, 8, OverflowPolicy::Fail).unwrap();
    let mut ends = ring.ends();
    let (_producer, mut consumer) = ends.split();

    let mut out = Vec::new();
    assert_eq!(consumer.try_recv_batch(&mut out), 0, "{backend:?}");
    assert!(out.is_empty(), "{backend:?} pushed something into the buffer");
    assert_eq!(consumer.try_recv(), None, "{backend:?}");
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Liveness readings.
// ─────────────────────────────────────────────────────────────────────────────

/// `len` and `is_empty` never disagree, at every point of a lap.
///
/// These are two computations of the same fact. `is_empty` is a comparison and
/// `len` a subtraction or a load, so they are exactly the pair that can drift apart.
///
/// `clippy::len_zero` fires on every assertion below and its advice would
/// rewrite `len() == 0` into `is_empty()`, turning each one into
/// `is_empty() == is_empty()`, a tautology that passes whatever the ring does.
/// Comparing the two spellings is the entire test, so the lint is silenced here
/// rather than obeyed.
#[allow(clippy::len_zero)]
#[test]
fn len_and_is_empty_agree_at_every_point_of_a_lap() {
  const CAPACITY: usize = 4;

  for backend in every_backend() {
    let mut ring: Ring<u32> = ring_on(backend, CAPACITY, OverflowPolicy::Fail).unwrap();
    let mut ends = ring.ends();
    let (mut producer, mut consumer) = ends.split();

    for filled in 0..CAPACITY {
      assert_eq!(consumer.len() == 0, consumer.is_empty(), "{backend:?} at {filled} pushed");
      assert_eq!(consumer.len(), filled, "{backend:?} miscounted after {filled} pushes");
      producer.try_push(filled as u32).unwrap();
    }

    assert_eq!(consumer.len(), CAPACITY, "{backend:?} miscounted a full ring");
    assert!(!consumer.is_empty(), "{backend:?} called a full ring empty");

    for drained in 1..=CAPACITY {
      assert!(consumer.try_recv().is_some(), "{backend:?} lost record {drained}");
      assert_eq!(consumer.len(), CAPACITY - drained, "{backend:?} miscounted while draining");
      assert_eq!(
        consumer.len() == 0,
        consumer.is_empty(),
        "{backend:?} disagreed while draining"
      );
    }
  }
}

/// `free_capacity` and `is_full` agree, and `free_capacity` never overstates.
///
/// Overstating is the dangerous direction and the only one testable from a
/// single thread. A reported `n` that is larger than the room actually
/// available breaks the SPSC caller who treats it as binding. Understating is
/// permitted everywhere. That is what "advisory" means.
#[test]
fn free_capacity_never_overstates_the_room_available() {
  const CAPACITY: usize = 4;

  for backend in every_backend() {
    let mut ring: Ring<u32> = ring_on(backend, CAPACITY, OverflowPolicy::Fail).unwrap();
    let mut ends = ring.ends();
    let (mut producer, _consumer) = ends.split();

    for pushed in 0..CAPACITY {
      let claimed = producer.free_capacity();
      assert_eq!(
        claimed == 0,
        producer.is_full(),
        "{backend:?} disagreed with itself at {pushed}"
      );
      assert!(
        claimed <= CAPACITY - pushed,
        "{backend:?} reported {claimed} free with only {} room",
        CAPACITY - pushed
      );

      // The binding half of the contract, which the two assertions above cannot
      // reach. Both are satisfied by a `free_capacity` that answers zero every
      // time. Understating is permitted, and zero is the largest understatement
      // there is, while `is_full` is *defined* as `free_capacity() == 0` and so
      // moves with it rather than against it. At SPSC the reading is exact, and
      // exactness is the only form of this assertion a constant cannot pass.
      if matches!(backend, Backend::Spsc) {
        assert_eq!(
          claimed,
          CAPACITY - pushed,
          "SPSC free_capacity is binding, so it must be exact at {pushed}"
        );
      }
      producer.try_push(pushed as u32).unwrap();
    }

    assert_eq!(producer.free_capacity(), 0, "{backend:?} reports room in a full ring");
    assert!(producer.is_full(), "{backend:?}");
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Payloads and API hygiene.
// ─────────────────────────────────────────────────────────────────────────────

/// A non-`Copy` payload arrives with its contents, not a shallow copy.
///
/// The uniform API moves values through a `TypedSlot< T >` on two of the
/// three backends. A heap payload is what distinguishes a real move from a
/// bitwise one that would double-free or leak.
#[test]
fn a_heap_payload_round_trips_its_contents() {
  for backend in every_backend() {
    let mut ring: Ring<String> = ring_on(backend, 4, OverflowPolicy::Fail).unwrap();
    let mut ends = ring.ends();
    let (mut producer, mut consumer) = ends.split();

    producer
      .try_push("a string long enough not to fit inline".to_string())
      .unwrap();

    assert_eq!(
      consumer.try_recv().as_deref(),
      Some("a string long enough not to fit inline"),
      "{backend:?} did not carry the payload's contents"
    );
  }
}

/// A payload that was never drained is dropped exactly once with the ring.
#[test]
fn records_left_in_a_dropped_ring_are_released_exactly_once() {
  use std::sync::Arc;
  use std::sync::atomic::{AtomicUsize, Ordering};

  for backend in every_backend() {
    let drops = Arc::new(AtomicUsize::new(0));

    #[derive(Debug)]
    struct Tracked(Arc<AtomicUsize>);
    impl Drop for Tracked {
      fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
      }
    }

    {
      let mut ring: Ring<Tracked> = ring_on(backend, 4, OverflowPolicy::Fail).unwrap();
      let mut ends = ring.ends();
      let (mut producer, _consumer) = ends.split();

      for _ in 0..3 {
        producer.try_push(Tracked(Arc::clone(&drops))).unwrap();
      }

      assert_eq!(drops.load(Ordering::SeqCst), 0, "{backend:?} dropped a live record");
    }

    assert_eq!(
      drops.load(Ordering::SeqCst),
      3,
      "{backend:?} leaked or double-freed an undrained record"
    );
  }
}

/// Every public type is `Debug`, so a caller can print one in a panic message.
#[test]
fn every_public_type_is_debuggable() {
  fn assert_debug<T: core::fmt::Debug>() {}

  assert_debug::<Backend>();
  assert_debug::<Ring<u8>>();
  assert_debug::<ring_core::Ends<'_, u8>>();
  assert_debug::<ring_core::Producer<'_, u8>>();
  assert_debug::<ring_core::Consumer<'_, u8>>();

  // And the value actually formats. `derive( Debug )` on an enum with a
  // feature-gated variant is exactly where a cfg mistake would show.
  let config = RingConfig::new(4).unwrap();
  let ring: Ring<u8> = Ring::new(&config).unwrap();
  assert!(format!("{ring:?}").contains("Ring"));
}

/// The overflow policy the ring was built with is the one it reports.
#[test]
fn a_ring_reports_the_overflow_policy_it_was_built_with() {
  for policy in [OverflowPolicy::Fail, OverflowPolicy::DropNewest] {
    for backend in every_backend() {
      let ring: Ring<u8> = ring_on(backend, 4, policy).unwrap();
      assert_eq!(ring.overflow(), policy, "{backend:?} rewrote its policy");
    }
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Concurrency.
// ─────────────────────────────────────────────────────────────────────────────

/// Four threads publish through cloned producers and nothing is lost.
///
/// The single-threaded tests above establish the API's shape. This one
/// establishes that the shape survives being used the way the multi-producer
/// backends exist to be used. It is skipped for SPSC, where a second producer
/// is refused by construction. That refusal is itself the correct behaviour,
/// and [`try_clone_refuses_at_spsc_and_permits_elsewhere`] asserts it.
#[test]
fn four_threads_publishing_through_clones_lose_nothing() {
  const PRODUCERS: u32 = 4;
  const PER_PRODUCER: u32 = 2_000;
  const TOTAL: usize = (PRODUCERS * PER_PRODUCER) as usize;
  const PATIENCE: std::time::Duration = std::time::Duration::from_secs(30);

  for backend in every_backend().into_iter().filter(|b| *b != Backend::Spsc) {
    let mut ring: Ring<u32> = ring_on(backend, 64, OverflowPolicy::Fail).unwrap();
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    let mut received = Vec::with_capacity(TOTAL);

    std::thread::scope(|scope| {
      for id in 0..PRODUCERS {
        let mut mine = producer.try_clone().expect("not SPSC");

        scope.spawn(move || {
          for index in 0..PER_PRODUCER {
            let record = id * PER_PRODUCER + index;
            let deadline = std::time::Instant::now() + PATIENCE;

            // Retry on a refusal. The consumer is what makes room. Bounded, so
            // a ring that stops releasing slots fails with a message instead of
            // hanging, the failure mode `ring_mpsc`'s own suite was bitten by.
            //
            // The deadline is computed once, outside the loop, and never
            // refreshed inside it. This loop first refreshed it per iteration,
            // which pushes it forward faster than time passes, so the
            // assertion can never fire and the guard silently becomes the
            // unbounded spin it exists to prevent. A successful push is the
            // only progress available here, and that leaves the loop.
            while let Err(returned) = mine.try_push(record) {
              assert!(
                std::time::Instant::now() < deadline,
                "{backend:?}: producer {id} made no progress for 30s at record {returned}"
              );
              std::hint::spin_loop();
            }
          }
        });
      }

      let mut deadline = std::time::Instant::now() + PATIENCE;
      while received.len() < TOTAL {
        let drained = consumer.try_recv_batch(&mut received);

        if drained > 0 {
          deadline = std::time::Instant::now() + PATIENCE;
        } else {
          assert!(
            std::time::Instant::now() < deadline,
            "{backend:?}: drained {} of {TOTAL} then stalled for 30s",
            received.len()
          );
          std::hint::spin_loop();
        }
      }
    });

    assert_eq!(received.len(), TOTAL, "{backend:?} delivered the wrong number of records");

    received.sort_unstable();
    assert_eq!(
      received,
      (0..TOTAL as u32).collect::<Vec<_>>(),
      "{backend:?} lost or duplicated a record under contention"
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Boundary: the smallest capacity the type system allows.
// ─────────────────────────────────────────────────────────────────────────────

/// A capacity-1 ring cycles correctly on every backend.
///
/// `ring_types::Capacity::new( 1 )` is explicitly `Ok`, because capacity is
/// validated to a power of two and `1 == 2^0`. Nothing else in this suite
/// builds one. The smallest capacity exercised elsewhere is 4. Capacity 1 is
/// the sharpest boundary the power-of-two constraint can produce.
/// `Capacity::mask()` is `capacity - 1 = 0`, so every sequence number's slot
/// index collapses onto the same single slot on every lap. An off-by-one in a
/// slot-index computation, or a mask applied to the wrong operand, has no
/// second slot to hide behind the way it might at a wider capacity. There is
/// only ever one slot to be wrong about, and this ring reuses it fifty times.
#[test]
fn a_capacity_of_one_cycles_correctly_on_every_backend() {
  const LAPS: usize = 50;

  for backend in every_backend() {
    let mut ring: Ring<u32> = ring_on(backend, 1, OverflowPolicy::Fail).expect("capacity 1 is a valid power of two everywhere");

    assert_eq!(ring.capacity().get(), 1, "{backend:?} did not honour capacity 1");

    let mut ends = ring.ends();
    let (mut producer, mut consumer) = ends.split();

    assert!(consumer.is_empty(), "{backend:?} started non-empty");
    assert_eq!(
      producer.free_capacity(),
      1,
      "{backend:?} misreported room in an empty ring of 1"
    );
    assert!(!producer.is_full(), "{backend:?} reported full before any push");

    for lap in 0..LAPS {
      let value = lap as u32;

      producer
        .try_push(value)
        .unwrap_or_else(|_| panic!("{backend:?} refused lap {lap}'s only push"));

      assert!(producer.is_full(), "{backend:?} did not fill after one push into capacity 1");
      assert_eq!(producer.free_capacity(), 0, "{backend:?} reported room in a full ring of 1");
      assert!(!consumer.is_empty(), "{backend:?} called a freshly-filled ring empty");
      assert_eq!(consumer.len(), 1, "{backend:?} miscounted a full ring of 1");

      // The refusal path, exercised where refusal and success are adjacent by
      // exactly one push. `Fail` is the policy here, so a second push must be
      // handed back rather than accepted and silently overwrite the pending
      // record. `DropOldest`'s crossbeam-only eviction has its own tests.
      let extra = value.wrapping_add(1000);
      assert_eq!(
        producer.try_push(extra),
        Err(extra),
        "{backend:?} accepted a push into a full ring of 1, or lost the refused record"
      );

      let received = consumer.try_recv();
      assert_eq!(received, Some(value), "{backend:?} lost or corrupted lap {lap}'s only record");

      assert!(consumer.is_empty(), "{backend:?} did not empty after draining its only slot");
      assert_eq!(
        producer.free_capacity(),
        1,
        "{backend:?} did not free its only slot on lap {lap}"
      );
      assert!(
        !producer.is_full(),
        "{backend:?} still reported full after draining lap {lap}"
      );
      assert_eq!(consumer.try_recv(), None, "{backend:?} yielded a second record from one slot");
    }
  }
}
