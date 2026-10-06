//! Integration tests for `ring_mpsc`.
//!
//! Carries `docs/feature/172_multi_producer_claim.md`'s reached-test, in which
//! four producer threads exchange 100 000 items with byte-parity, no sequence
//! granted twice, and each producer's own items preserved in its issue order.
//!
//! # These tests can see an ordering bug, and that was measured rather than assumed
//!
//! `ring_mpsc::PUBLISH`'s documentation says a `Relaxed` publish works on
//! x86-64, where the hardware supplies the ordering the code failed to ask
//! for. So a green suite on x86-64 is no evidence for the publication
//! ordering. That reasoning is right, but its premise does not hold here. This
//! workspace's host is `aarch64-unknown-linux-gnu` (ARM Neoverse-N1), which is
//! weakly ordered. Check it with `rustc -vV | grep host`.
//!
//! So the mutation was run. Weakening `PUBLISH` from `Release` to `Relaxed` and
//! running the parity test alone, sixty times each way:
//!
//! | `PUBLISH` | 60 runs | |
//! |-----------|---------|---|
//! | `Ordering::Release` | 60 pass, 0 fail | |
//! | `Ordering::Relaxed` | 46 pass, **14 fail** | `sequence Seq(2234) was drained as published but its slot was empty` |
//!
//! Roughly one run in four. That is the "once in a billion transactions"
//! heisenbug the invariant describes, made frequent by 100 000 elements of
//! sustained contention on hardware that can reorder. It is also why the parity
//! test is at that scale rather than a convenient smaller one.
//!
//! **A 23% detection rate is a real check, not a reliable one.** Three quarters
//! of runs would still report green against a broken publish, so a single
//! passing run of this file is not evidence either. The `exhaustive` module
//! closes that gap. `loom` enumerates the interleavings exhaustively rather
//! than sampling them, so its verdict does not depend on which one the
//! scheduler happened to pick. The two complement each other: real hardware at
//! scale, and every interleaving at small scale.
//!
//! The whole ordinary body is `#[ cfg( not( loom ) ) ]` because a `Ring`'s
//! cursors and stamps are `ring_atomic` cells, which panic outside a
//! `loom::model` under `--cfg loom`.
//!
//! `tests/manual/readme.md`'s run record exercises both halves: the ordinary
//! suite under an ordinary build, the models under `RUSTFLAGS="--cfg loom"`,
//! and both again under the weakened publish above.

#![cfg(test)]

#[cfg(not(loom))]
mod threaded {
  use core::sync::atomic::{AtomicUsize, Ordering};
  use std::collections::HashSet;
  use std::sync::Mutex;

  use ring_atomic::SeqCell;
  use ring_config::RingConfig;
  use ring_mpsc::{COMMIT, Consumer, OBSERVE, OWN, PUBLISH, Producer, Reserved, Ring, UNSTAMPED};
  use ring_slot::{BytesSlot, Slot, TypedSlot};
  use ring_types::{Capacity, RingError, Seq};

  fn capacity(slots: usize) -> Capacity {
    Capacity::new(slots).expect("a power of two")
  }

  /// How long a thread in the concurrent test may make no progress before it
  /// gives up and says so.
  ///
  /// **Found by mutation, not by design.** The first version of the parity test
  /// had unbounded retry loops on both sides. Weakening `PUBLISH` to `Relaxed`
  /// killed the consumer thread on a failed `expect`, whereupon the producers
  /// spun on `RingError::Full` forever and the whole suite hung instead of
  /// failing. The test could not report the defect it was written to catch.
  /// Every wait below is bounded so the failure arrives as a message.
  const PATIENCE: std::time::Duration = std::time::Duration::from_secs(30);

  // ───────────────────────────────────────────────────────────────────────────
  // The reached-test.
  // ───────────────────────────────────────────────────────────────────────────

  /// Four producers, 25 000 items each, exchanged with byte-parity.
  ///
  /// This is `docs/feature/172_multi_producer_claim.md`'s acceptance criterion
  /// and the only test in the crate that is one. Three separate claims, checked
  /// together because a mechanism can satisfy any two and fail the third:
  ///
  /// - **Byte-parity as a multiset.** All 100 000 items arrive, none twice.
  /// - **No sequence granted twice.** Every claimed sequence is distinct, which
  ///   is the contended claim's own property rather than the ring's.
  /// - **Per-producer issue order.** A producer's own items arrive in the order
  ///   it issued them. Nothing constrains the interleaving *between* producers,
  ///   which is what concurrent claiming means. But a mechanism that reordered
  ///   one producer's own items would have granted its sequences out of order.
  #[test]
  fn four_producers_exchange_one_hundred_thousand_items_with_byte_parity() {
    const PRODUCERS: u64 = 4;
    const PER_PRODUCER: u64 = 25_000;
    const TOTAL: usize = (PRODUCERS * PER_PRODUCER) as usize;

    // The payload encodes both its producer and its position within that
    // producer's stream, so one `u64` carries everything all three claims need.
    let encode = |producer: u64, index: u64| producer * PER_PRODUCER + index;

    let mut ring: Ring<TypedSlot<u64>> = Ring::new(capacity(1024));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    let granted: Mutex<Vec<Seq>> = Mutex::new(Vec::with_capacity(TOTAL));
    let received: Mutex<Vec<u64>> = Mutex::new(Vec::with_capacity(TOTAL));

    std::thread::scope(|scope| {
      for id in 0..PRODUCERS {
        // No rebind needed. `Producer` is `Copy`, so `move` copies it into each
        // closure rather than moving the one handle into the first.
        let granted = &granted;

        scope.spawn(move || {
          let mut mine = Vec::with_capacity(PER_PRODUCER as usize);

          for index in 0..PER_PRODUCER {
            let value = encode(id, index);
            let deadline = std::time::Instant::now() + PATIENCE;

            loop {
              match producer.claim() {
                Ok(mut reserved) => {
                  mine.push(reserved.sequence());
                  reserved.set(value);
                  break;
                }
                // Back-pressure means the consumer has not caught up. Retrying
                // is the whole of the `Fail` policy's contract, but not
                // forever, or a dead consumer becomes a hang rather than a
                // failure.
                Err(RingError::Full) => {
                  assert!(
                    std::time::Instant::now() < deadline,
                    "producer {id} stalled at item {index}: back-pressure never cleared, \
                     which means the consumer stopped draining"
                  );
                  std::thread::yield_now();
                }
                Err(other) => panic!("unexpected claim failure: {other:?}"),
              }
            }
          }

          // Fix(mpsc_test_granted_received_lock_poison_recovery): all four
          // producer threads (`granted`, here) and the one consumer thread
          // (`received`, below) share one `Mutex< Vec< _ > >` apiece for the
          // whole `std::thread::scope` block above. A panic in any one
          // producer's tiny critical section (an allocator failure inside
          // `extend`, say) would poison the lock. Every other still-running
          // producer's own `.expect()` here would then panic too, and the
          // confusing "no panic while holding the lock" message would bury
          // whichever producer's panic was the real, original one.
          // `thread::scope` already re-panics with *a* real panic once every
          // thread is joined, regardless, so recovering here does not change
          // whether this test fails. It changes only whether the message
          // reported is the actual bug or a poisoned-lock echo of it.
          // Pitfall: a lock scoped to a single test function still has live
          // sibling threads racing on it while that function's own
          // `thread::scope` block runs. "It can't outlive the test" does not
          // mean "it can't poison a sibling mid-test."
          granted.lock().unwrap_or_else(std::sync::PoisonError::into_inner).extend(mine);
        });
      }

      let received = &received;

      scope.spawn(move || {
        let mut drained = Vec::with_capacity(TOTAL);
        let mut deadline = std::time::Instant::now() + PATIENCE;

        while drained.len() < TOTAL {
          let mut batch = consumer.drain();

          if batch.is_empty() {
            assert!(
              std::time::Instant::now() < deadline,
              "consumer stalled after {} of {TOTAL} records: nothing further became visible",
              drained.len()
            );
            std::thread::yield_now();
            continue;
          }

          let start = batch.start();

          for offset in 0..batch.len() {
            let value = batch.get_mut(offset).and_then(TypedSlot::take).unwrap_or_else(|| {
              panic!(
                "sequence {:?} was drained as published but its slot was empty — \
                 the stamp became visible before the payload write it was supposed to release",
                start.advanced_by(offset as u64)
              )
            });
            drained.push(value);
          }

          deadline = std::time::Instant::now() + PATIENCE;
        }

        // Fix(mpsc_test_granted_received_lock_poison_recovery): same hazard as
        // `granted` above, for the consumer thread's own `received` lock.
        received
          .lock()
          .unwrap_or_else(std::sync::PoisonError::into_inner)
          .extend(drained);
      });
    });

    // Fix(mpsc_test_granted_received_lock_poison_recovery): `into_inner` can
    // observe the same poisoning that `granted`/`received` above already guard
    // against. It is unreachable today only because `thread::scope` above
    // would already have re-panicked on any real thread panic before
    // execution reaches here. This file should not have to keep proving that
    // invariant by inspection every time the threading above changes.
    let granted = granted.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner);
    let received = received.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner);

    // Claim 1 is byte-parity as a multiset.
    assert_eq!(received.len(), TOTAL, "every offered item arrived exactly once");
    let mut sorted = received.clone();
    sorted.sort_unstable();
    let expected: Vec<u64> = (0..TOTAL as u64).collect();
    assert_eq!(sorted, expected, "the multiset of received items is the multiset offered");

    // Claim 2 is that no sequence is granted twice.
    assert_eq!(granted.len(), TOTAL);
    let distinct: HashSet<Seq> = granted.iter().copied().collect();
    assert_eq!(distinct.len(), TOTAL, "no two producers were granted the same sequence");

    // Claim 3 is that each producer's own items stayed in its issue order.
    for id in 0..PRODUCERS {
      let lo = encode(id, 0);
      let hi = encode(id, PER_PRODUCER - 1);
      let mine: Vec<u64> = received.iter().copied().filter(|value| (lo..=hi).contains(value)).collect();

      assert_eq!(mine.len(), PER_PRODUCER as usize);
      assert!(
        mine.windows(2).all(|pair| pair[0] < pair[1]),
        "producer {id}'s own items arrived out of its issue order"
      );
    }
  }

  /// The reached-test again, with batched producers: each producer claims
  /// groups of 32 contiguous sequences through `claim_batch`, writes them
  /// through the guard, and publishes with the drop. All three claims of the
  /// original must hold unchanged — the grant width changes how the
  /// contended step is amortised, not the contract.
  #[test]
  fn four_producers_exchange_one_hundred_thousand_items_with_byte_parity_in_batches() {
    const PRODUCERS: u64 = 4;
    const PER_PRODUCER: u64 = 25_000;
    const BATCH: usize = 32;
    const TOTAL: usize = (PRODUCERS * PER_PRODUCER) as usize;

    let encode = |producer: u64, index: u64| producer * PER_PRODUCER + index;

    let mut ring: Ring<TypedSlot<u64>> = Ring::new(capacity(1024));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    let granted: Mutex<Vec<Seq>> = Mutex::new(Vec::with_capacity(TOTAL));
    let received: Mutex<Vec<u64>> = Mutex::new(Vec::with_capacity(TOTAL));

    std::thread::scope(|scope| {
      for id in 0..PRODUCERS {
        let granted = &granted;

        scope.spawn(move || {
          let mut mine = Vec::with_capacity(PER_PRODUCER as usize);
          let mut next = 0u64;

          while next < PER_PRODUCER {
            let deadline = std::time::Instant::now() + PATIENCE;
            // Ask for no more than this producer still owes: a wider grant
            // would be written past the quota, and the byte-parity claim
            // would rightly catch the overflow.
            let want = usize::try_from(PER_PRODUCER - next).unwrap().min(BATCH);

            match producer.claim_batch(want) {
              Ok(mut batch) => {
                for offset in 0..batch.len() {
                  batch.slot_mut(offset).expect("within the grant").set(encode(id, next));
                  mine.push(batch.sequence(offset).expect("within the grant"));
                  next += 1;
                }
                drop(batch);
              }
              // Back-pressure: the consumer has not caught up. Bounded, as in
              // the single-record test above — a dead consumer must fail the
              // test, not hang it.
              Err(RingError::Full) => {
                assert!(
                  std::time::Instant::now() < deadline,
                  "producer {id} stalled at item {next}: back-pressure never cleared, \
                   which means the consumer stopped draining"
                );
                std::thread::yield_now();
              }
              Err(other) => panic!("unexpected claim failure: {other:?}"),
            }
          }

          granted.lock().unwrap_or_else(std::sync::PoisonError::into_inner).extend(mine);
        });
      }

      let received = &received;

      scope.spawn(move || {
        let mut drained = Vec::with_capacity(TOTAL);
        let mut deadline = std::time::Instant::now() + PATIENCE;

        while drained.len() < TOTAL {
          let mut batch = consumer.drain();

          if batch.is_empty() {
            assert!(
              std::time::Instant::now() < deadline,
              "consumer stalled after {} of {TOTAL} records: nothing further became visible",
              drained.len()
            );
            std::thread::yield_now();
            continue;
          }

          let start = batch.start();

          for offset in 0..batch.len() {
            let value = batch.get_mut(offset).and_then(TypedSlot::take).unwrap_or_else(|| {
              panic!(
                "sequence {:?} was drained as published but its slot was empty — \
                 the stamp became visible before the payload write it was supposed to release",
                start.advanced_by(offset as u64)
              )
            });
            drained.push(value);
          }

          deadline = std::time::Instant::now() + PATIENCE;
        }

        received
          .lock()
          .unwrap_or_else(std::sync::PoisonError::into_inner)
          .extend(drained);
      });
    });

    let granted = granted.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner);
    let received = received.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner);

    assert_eq!(received.len(), TOTAL, "every offered item arrived exactly once");
    let mut sorted = received.clone();
    sorted.sort_unstable();
    let expected: Vec<u64> = (0..TOTAL as u64).collect();
    assert_eq!(sorted, expected, "the multiset of received items is the multiset offered");

    assert_eq!(granted.len(), TOTAL);
    let distinct: HashSet<Seq> = granted.iter().copied().collect();
    assert_eq!(distinct.len(), TOTAL, "no two producers were granted the same sequence");

    for id in 0..PRODUCERS {
      let lo = encode(id, 0);
      let hi = encode(id, PER_PRODUCER - 1);
      let mine: Vec<u64> = received.iter().copied().filter(|value| (lo..=hi).contains(value)).collect();

      assert_eq!(mine.len(), PER_PRODUCER as usize);
      assert!(
        mine.windows(2).all(|pair| pair[0] < pair[1]),
        "producer {id}'s own items arrived out of its issue order"
      );
    }
  }

  // ───────────────────────────────────────────────────────────────────────────
  // Adversarial contention audit, measured rather than inferred.
  // ───────────────────────────────────────────────────────────────────────────

  /// Eight producers against a ring sixteen times smaller than the reached-test
  /// above, with every `RingError::Full` retry counted rather than assumed from
  /// thread count.
  ///
  /// The reached-test above proves the mechanism holds at scale (100 000
  /// items, capacity 1024, ~97 wraps) but never asks whether the producers
  /// collided. It retries silently on `RingError::Full` and never counts how
  /// often that happened. A green run of that test alone is consistent with a
  /// contention-free execution where the consumer always kept ahead. This test
  /// closes that gap with a counter. `full_retries` increments on every
  /// observed `RingError::Full` across all eight producers, and the test fails
  /// if that counter is ever zero. A pass here is evidence that contention
  /// happened during this run, not an inference from thread count.
  ///
  /// Three things this shape stresses harder than any existing test:
  ///
  /// - **Slot-corruption sensitivity.** The payload is `[ u64 ; 8 ]`, all eight
  ///   words set to the same producer/index encoding. A claim-exclusivity bug
  ///   letting two producers' writes land in the same slot would need all
  ///   sixteen words (both producers' eight) to interleave into a
  ///   self-consistent record purely by chance to escape detection. The
  ///   single-word payload above cannot see this class of corruption at all,
  ///   because one word cannot be internally inconsistent with itself.
  /// - **Wraparound density under real contention.** Capacity 8 against 32 000
  ///   items is 4 000 laps, eight times denser than
  ///   `every_slot_is_reused_across_many_laps_without_loss_or_duplication`'s
  ///   500 laps. That test is single-threaded. Here every lap happens while
  ///   producers contend for the slots the consumer is reclaiming.
  /// - **Measured back-pressure.** The test asserts `full_retries > 0` instead
  ///   of hoping for it. Eight producers against eight slots with one consumer
  ///   make `RingError::Full` all but certain, and the assertion turns that
  ///   near-certainty into a checked fact about the run that happened, rather
  ///   than a claim resting on the thread count alone.
  #[test]
  fn producers_under_measured_contention_at_small_capacity_show_no_torn_or_duplicated_records() {
    const PRODUCERS: u64 = 8;
    const PER_PRODUCER: u64 = 4_000;
    const TOTAL: usize = (PRODUCERS * PER_PRODUCER) as usize;
    const WORDS: usize = 8;

    let encode = |producer: u64, index: u64| producer * PER_PRODUCER + index;

    let mut ring: Ring<TypedSlot<[u64; WORDS]>> = Ring::new(capacity(8));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    let granted: Mutex<Vec<Seq>> = Mutex::new(Vec::with_capacity(TOTAL));
    let received: Mutex<Vec<u64>> = Mutex::new(Vec::with_capacity(TOTAL));
    let full_retries = AtomicUsize::new(0);
    let full_retries = &full_retries;

    std::thread::scope(|scope| {
      for id in 0..PRODUCERS {
        let granted = &granted;

        scope.spawn(move || {
          let mut mine = Vec::with_capacity(PER_PRODUCER as usize);

          for index in 0..PER_PRODUCER {
            let value = encode(id, index);
            let record = [value; WORDS];
            let deadline = std::time::Instant::now() + PATIENCE;

            loop {
              match producer.claim() {
                Ok(mut reserved) => {
                  mine.push(reserved.sequence());
                  reserved.set(record);
                  break;
                }
                Err(RingError::Full) => {
                  // The counter this test exists to provide. Each increment is
                  // an observed collision against capacity, not an inference
                  // from thread count.
                  full_retries.fetch_add(1, Ordering::Relaxed);
                  assert!(
                    std::time::Instant::now() < deadline,
                    "producer {id} stalled at item {index}: back-pressure never cleared"
                  );
                  std::thread::yield_now();
                }
                Err(other) => panic!("unexpected claim failure: {other:?}"),
              }
            }
          }

          granted.lock().unwrap_or_else(std::sync::PoisonError::into_inner).extend(mine);
        });
      }

      let received = &received;

      scope.spawn(move || {
        let mut drained = Vec::with_capacity(TOTAL);
        let mut deadline = std::time::Instant::now() + PATIENCE;

        while drained.len() < TOTAL {
          let mut batch = consumer.drain();

          if batch.is_empty() {
            assert!(
              std::time::Instant::now() < deadline,
              "consumer stalled after {} of {TOTAL} records",
              drained.len()
            );
            std::thread::yield_now();
            continue;
          }

          let start = batch.start();

          for offset in 0..batch.len() {
            let record = batch.get_mut(offset).and_then(TypedSlot::take).unwrap_or_else(|| {
              panic!(
                "sequence {:?} was drained as published but its slot was empty",
                start.advanced_by(offset as u64)
              )
            });

            // Self-consistency check. Every word of a single-owner record is
            // identical. A claim-exclusivity bug letting a second
            // producer's write land in this slot would need all eight words
            // to agree by chance to hide from this check.
            let first = record[0];
            assert!(
              record.iter().all(|word| *word == first),
              "sequence {:?} carries an internally inconsistent record {record:?} — \
               two producers' writes landed in the same slot",
              start.advanced_by(offset as u64)
            );

            drained.push(first);
          }

          deadline = std::time::Instant::now() + PATIENCE;
        }

        received
          .lock()
          .unwrap_or_else(std::sync::PoisonError::into_inner)
          .extend(drained);
      });
    });

    let granted = granted.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner);
    let received = received.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner);

    // The measured-contention assertion is the whole point of the test. A
    // pass here is direct evidence that producers collided against capacity
    // during this run, not an assumption resting on thread count.
    assert!(
      full_retries.load(Ordering::Relaxed) > 0,
      "zero RingError::Full observed across {PRODUCERS} producers at capacity 8 — \
       this run measured no contention at all, so it proves nothing about it"
    );

    // Claim 1 is byte-parity as a multiset.
    assert_eq!(received.len(), TOTAL, "every offered item arrived exactly once");
    let mut sorted = received.clone();
    sorted.sort_unstable();
    let expected: Vec<u64> = (0..TOTAL as u64).collect();
    assert_eq!(sorted, expected, "the multiset of received items is the multiset offered");

    // Claim 2 is that no sequence is granted twice.
    assert_eq!(granted.len(), TOTAL);
    let distinct: HashSet<Seq> = granted.iter().copied().collect();
    assert_eq!(distinct.len(), TOTAL, "no two producers were granted the same sequence");

    // Claim 3 is that each producer's own items stayed in its issue order.
    for id in 0..PRODUCERS {
      let lo = encode(id, 0);
      let hi = encode(id, PER_PRODUCER - 1);
      let mine: Vec<u64> = received.iter().copied().filter(|value| (lo..=hi).contains(value)).collect();

      assert_eq!(mine.len(), PER_PRODUCER as usize);
      assert!(
        mine.windows(2).all(|pair| pair[0] < pair[1]),
        "producer {id}'s own items arrived out of its issue order"
      );
    }
  }

  // ───────────────────────────────────────────────────────────────────────────
  // The shape the soundness argument in `docs/workaround/readme.md` rests on.
  // ───────────────────────────────────────────────────────────────────────────

  /// The producer is shareable and the consumer is not.
  ///
  /// `Ring`'s `unsafe impl Sync` argues that producers and the consumer touch
  /// disjoint slots. That argument is worth nothing if a second consumer is
  /// constructible, because two consumers would each scan-then-commit the same
  /// cursor and hand out records the other had already taken.
  ///
  /// This test asserts the positive half. It cannot assert the negative half,
  /// that `Consumer` is neither `Clone` nor `Sync`, because a type not
  /// implementing a trait has no runtime evidence. The negative half lives in
  /// `src/lib.rs` as `compile_fail` doc tests, which is the only executable
  /// form a negative has. It lives there rather than here because rustdoc
  /// collects doc tests from the library target only.
  #[test]
  fn the_producer_is_send_and_sync_and_copy_which_is_what_multi_producer_means() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    fn assert_copy<T: Copy>() {}

    assert_send::<Producer<'static, TypedSlot<u8>>>();
    assert_sync::<Producer<'static, TypedSlot<u8>>>();
    assert_copy::<Producer<'static, TypedSlot<u8>>>();

    // The consumer crosses a thread boundary once, at the split. It is `Send`
    // for that and no more.
    assert_send::<Consumer<'static, TypedSlot<u8>>>();
  }

  /// The claim cursor and the consumer cursor do not share a cache line.
  ///
  /// This crate states the padding as a contract and `ring_cursor` implements
  /// it. A sibling change dropping the alignment would put the single most
  /// contended write in the crate on the same line as the consumer's commit,
  /// and break nothing that compiles.
  #[test]
  fn the_claim_cursor_and_the_consumer_cursor_are_on_distinct_cache_lines() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(8));
    let mut ends = ring.ends();
    let (producer, _consumer) = ends.split();

    assert!(producer.on_distinct_lines());
  }

  // ───────────────────────────────────────────────────────────────────────────
  // Claim-cursor persistence — a second `ends` continues, never restarts.
  // ───────────────────────────────────────────────────────────────────────────

  /// A second `ends()` continues the claim cursor from the first generation's
  /// watermark.
  ///
  /// The claim cursor is the ring's own cell: each `ends()` builds its
  /// claimer over the same cell, so the second generation's first claim is
  /// the sequence after the first generation's last, not zero.
  #[test]
  fn a_second_ends_continues_the_claim_cursor_rather_than_restarting_it() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    {
      let mut ends = ring.ends();
      let (producer, mut consumer) = ends.split();

      for value in 1..=3 {
        producer.push(value).expect("room");
      }
      drop(consumer.drain());
    }
    // The first generation's claim cursor stands at 3, its consumer at 3.

    {
      let mut ends = ring.ends();
      let (producer, mut consumer) = ends.split();

      let reserved = producer.claim().expect("room");
      assert_eq!(
        reserved.sequence(),
        Seq(3),
        "the second generation continues from the first's watermark"
      );
      drop(reserved); // publishes an empty slot at sequence 3
      producer.push(7).expect("room"); // sequence 4

      let mut batch = consumer.drain();
      assert_eq!(batch.len(), 2, "the whole grant is published, empty slot included");
      assert_eq!(batch.get(0).and_then(TypedSlot::get), None);
      assert_eq!(batch.get_mut(1).and_then(TypedSlot::take), Some(7));
    }
  }

  /// The defect the persistence fixes, kept as a regression test: before the
  /// claim cursor moved into the ring, a push through a second `ends()`
  /// answered `Ok` at sequence zero under a consumer that had moved past it —
  /// accepted, published into a slot the consumer never rescanned, and never
  /// delivered.
  #[test]
  fn a_record_pushed_through_a_second_ends_is_delivered_rather_than_lost() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    {
      let mut ends = ring.ends();
      let (producer, mut consumer) = ends.split();

      for value in 1..=3 {
        producer.push(value).expect("room");
      }
      drop(consumer.drain());
    }

    {
      let mut ends = ring.ends();
      let (producer, mut consumer) = ends.split();

      assert_eq!(
        producer.push(4).expect("the grant continues, so the ring has room"),
        Seq(3),
        "the push lands on the continuation sequence, not back at zero"
      );

      let mut batch = consumer.drain();
      assert_eq!(batch.len(), 1, "delivered — the pre-fix behaviour drained nothing here");
      assert_eq!(batch.get_mut(0).and_then(TypedSlot::take), Some(4));
    }
  }

  // ───────────────────────────────────────────────────────────────────────────
  // Total order rests on the published watermark.
  // ───────────────────────────────────────────────────────────────────────────

  /// A gap below a published sequence hides everything above it.
  ///
  /// The one property that distinguishes a correct MPSC drain from an
  /// almost-correct one. Producers claim in a serialized order and publish in
  /// an arbitrary one, so at any instant the set of published sequences may
  /// have holes. A drain that stopped at the *highest* published sequence
  /// rather than the first gap would deliver sequence 3 before sequence 1
  /// existed. Element counts would still reconcile, and total order would be
  /// silently broken.
  #[test]
  fn the_drain_stops_at_the_first_unpublished_sequence_not_the_highest_published() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(8));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    let mut first = producer.claim().expect("room");
    let mut second = producer.claim().expect("room");
    let mut third = producer.claim().expect("room");
    first.set(1);
    second.set(2);
    third.set(3);

    // Publish out of order: 1 and 2 land, 0 does not.
    drop(third);
    drop(second);

    assert_eq!(consumer.available(), 0, "a hole at sequence 0 hides 1 and 2");
    assert_eq!(producer.ring().published_through(), None);
    assert!(consumer.drain().is_empty());

    drop(first);

    assert_eq!(consumer.available(), 3, "closing the hole reveals all three at once");
    assert_eq!(producer.ring().published_through(), Some(Seq(2)));

    let mut batch = consumer.drain();
    let drained: Vec<u8> = (0..batch.len())
      .map(|offset| batch.get_mut(offset).and_then(TypedSlot::take).expect("published"))
      .collect();
    assert_eq!(drained, vec![1, 2, 3], "and in sequence order, not publication order");
  }

  /// The watermark is recomputed on every drain, not cached.
  #[test]
  fn a_second_drain_sees_what_was_published_after_the_first() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    producer.push(1).expect("room");

    // `committed` is asserted everywhere else in this suite against `Seq::ZERO`
    // and nowhere against anything else, which makes it indistinguishable from
    // a method that returns the default and never reads the cursor at all. A
    // watermark is only a watermark if it moves; this is the one place that
    // says so.
    assert_eq!(consumer.ring().committed(), Seq::ZERO, "nothing has been drained yet");
    assert_eq!(consumer.drain().len(), 1);
    assert_eq!(
      consumer.ring().committed(),
      Seq(1),
      "the drain advanced the consumer cursor past the record it took",
    );

    producer.push(2).expect("room");
    let mut batch = consumer.drain();
    assert_eq!(batch.start(), Seq(1));
    assert_eq!(batch.get_mut(0).and_then(TypedSlot::take), Some(2));
  }

  /// An unwritten claim on the first lap publishes an empty record, not a torn
  /// one.
  ///
  /// `Reserved`'s `Drop` publishes unconditionally, which is what makes the
  /// publish impossible to skip. One objection is that a guard therefore
  /// publishes a partially-written slot on a panic between claim and write. It
  /// does not. It publishes the slot as it stands. On the first lap the slot is
  /// still `Default`, so the observable result is one empty record: defined,
  /// drainable, and distinguishable from a written one. On a later lap the slot
  /// holds whatever record the consumer did not take, which this test does not
  /// cover.
  #[test]
  fn a_claim_dropped_without_a_write_publishes_an_empty_record() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    drop(producer.claim().expect("room"));
    producer.push(7).expect("room");

    let mut batch = consumer.drain();
    assert_eq!(batch.len(), 2, "the skipped write still occupies its sequence");
    assert_eq!(batch.get(0).and_then(TypedSlot::get), None, "empty, not torn");
    assert_eq!(batch.get_mut(1).and_then(TypedSlot::take), Some(7));
  }

  /// A claim never published stalls the ring, which is the cost the guard
  /// exists to remove, shown here by holding a guard rather than leaking one.
  #[test]
  fn an_unpublished_claim_blocks_every_later_sequence_while_it_is_held() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, consumer) = ends.split();

    let held = producer.claim().expect("room");
    producer.push(1).expect("room");
    producer.push(2).expect("room");

    assert_eq!(consumer.available(), 0, "two published records, none reachable");

    drop(held);

    assert_eq!(consumer.available(), 3);
  }

  // ───────────────────────────────────────────────────────────────────────────
  // Batched claims — one gate check and one exchange per group of slots.
  // ───────────────────────────────────────────────────────────────────────────

  /// The grant is capped by headroom, not by the ask: a ring with three slots
  /// free answers `claim_batch( 8 )` with three, and a full ring answers with
  /// `Full` and no movement at all.
  #[test]
  fn a_batch_claim_grants_what_headroom_allows_rather_than_the_full_ask() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, _consumer) = ends.split();

    producer.push(9).expect("room"); // producer 1, consumer 0: three free

    let batch = producer.claim_batch(8).expect("three slots free");
    assert_eq!(batch.len(), 3, "granted the headroom, not the ask");
    assert_eq!(batch.sequence(0), Some(Seq(1)));
    assert_eq!(batch.sequence(2), Some(Seq(3)));
    assert_eq!(batch.sequence(3), None);
    drop(batch);

    assert_eq!(producer.claim_batch(8).err(), Some(RingError::Full));
    assert_eq!(producer.free_capacity(), 0, "a refused claim advances nothing");
  }

  /// An unwritten offset of a dropped batch publishes an empty slot, one per
  /// offset — the batched form of `a_claim_dropped_without_a_write_publishes_
  /// an_empty_record` above — and the drain still counts exactly the grant's
  /// length, in issue order.
  #[test]
  fn a_batch_dropped_without_writing_every_offset_publishes_an_empty_slot_per_offset() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    let mut batch = producer.claim_batch(3).expect("room");
    batch.slot_mut(0).expect("offset 0").set(7);
    batch.slot_mut(2).expect("offset 2").set(9);
    drop(batch); // offset 1 was never written

    let mut drained = consumer.drain();
    assert_eq!(drained.len(), 3, "every offset of the grant is published");
    assert_eq!(drained.get_mut(0).and_then(TypedSlot::take), Some(7));
    assert_eq!(drained.get(1).and_then(TypedSlot::get), None, "empty, not torn");
    assert_eq!(drained.get_mut(2).and_then(TypedSlot::take), Some(9));
  }

  /// A grant that crosses a lap wraps its slot indices and preserves issue
  /// order — `ring_batch::drain_order`'s fold, exercised from the producer
  /// side: sequences 3..7 over capacity 4 address slots 3, 0, 1, 2.
  #[test]
  fn a_batch_claim_spanning_a_lap_wraps_its_slots_and_preserves_issue_order() {
    let mut ring: Ring<TypedSlot<u64>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    for value in 0..3 {
      producer.push(value).expect("room");
    }
    drop(consumer.drain());

    let mut batch = producer.claim_batch(4).expect("the whole ring is free");
    assert_eq!(batch.start(), Seq(3));
    for offset in 0..batch.len() {
      batch.slot_mut(offset).expect("within the grant").set(3 + offset as u64);
    }
    drop(batch);

    let mut drained = consumer.drain();
    assert_eq!(drained.len(), 4);
    let received: Vec<u64> = (0..drained.len())
      .map(|offset| drained.get_mut(offset).and_then(TypedSlot::take).expect("published"))
      .collect();
    assert_eq!(received, vec![3, 4, 5, 6], "issue order, across the wrap");
  }

  /// `push_batch` drains exactly the granted prefix out of the caller's vec,
  /// leaves the rest in order for the next attempt, and refuses a full ring
  /// with the vec untouched — the `push` contract, per group.
  #[test]
  fn push_batch_drains_the_granted_prefix_and_refuses_a_full_ring_untouched() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    let mut records: Vec<u8> = (1..=6).collect();
    assert_eq!(producer.push_batch(&mut records).expect("room for four"), 4);
    assert_eq!(records, vec![5, 6], "the refused tail stays, in order");
    assert_eq!(producer.push_batch(&mut records).err(), Some(RingError::Full));
    assert_eq!(records, vec![5, 6], "a refused batch moves nothing");

    let mut drained = consumer.drain();
    assert_eq!(drained.len(), 4);
    let received: Vec<u8> = (0..drained.len())
      .map(|offset| drained.get_mut(offset).and_then(TypedSlot::take).expect("published"))
      .collect();
    assert_eq!(received, vec![1, 2, 3, 4]);
    drop(drained);

    assert_eq!(producer.push_batch(&mut records).expect("room again"), 2);
    assert!(records.is_empty());
    assert_eq!(producer.push_batch(&mut Vec::new()).expect("nothing to do"), 0);
  }

  /// Two copies of the `Copy` producer batch-claim disjoint ranges — the
  /// cloned-producer cursor sharing holds for grants of any width.
  #[test]
  fn cloned_producers_batch_claims_are_disjoint() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();
    let second = producer;

    let first_batch = producer.claim_batch(2).expect("room");
    let second_batch = second.claim_batch(2).expect("room");
    assert_eq!(first_batch.start(), Seq::ZERO);
    assert_eq!(second_batch.start(), Seq(2), "the second grant starts past the first");

    drop((first_batch, second_batch));
    assert_eq!(consumer.drain().len(), 4);
  }

  // ───────────────────────────────────────────────────────────────────────────
  // Back-pressure under the `Fail` policy, and the capacity bound it enforces.
  // ───────────────────────────────────────────────────────────────────────────

  #[test]
  fn a_claim_past_capacity_reports_full_rather_than_overwriting() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(2));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    producer.push(1).expect("room");
    producer.push(2).expect("room");

    assert_eq!(producer.push(3), Err(RingError::Full));
    assert_eq!(producer.free_capacity(), 0);

    // The two published records are untouched. That is what distinguishes the
    // `Fail` policy from `DropOldest`, and it is the exactly-once contract at
    // the overflow edge.
    let mut batch = consumer.drain();
    assert_eq!(batch.len(), 2);
    assert_eq!(batch.get_mut(0).and_then(TypedSlot::take), Some(1));
  }

  #[test]
  fn a_commit_restores_exactly_the_capacity_it_released() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    for value in 1..=4 {
      producer.push(value).expect("room");
    }
    assert_eq!(producer.free_capacity(), 0);

    drop(consumer.drain_up_to(2));
    assert_eq!(producer.free_capacity(), 2);

    drop(consumer.drain());
    assert_eq!(producer.free_capacity(), 4);
  }

  /// The batch holds the slots until it is dropped, not until it is read.
  ///
  /// This is what makes `COMMIT`'s `Release` meaningful. Were the cursor
  /// advanced at scan time, a producer could overwrite a slot the caller was
  /// still reading through `get`.
  #[test]
  fn a_live_batch_still_holds_its_slots_against_reuse() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(2));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    producer.push(1).expect("room");
    producer.push(2).expect("room");

    let batch = consumer.drain();
    assert_eq!(producer.free_capacity(), 0, "drained but not committed is still occupied");
    assert_eq!(batch.get(0).and_then(TypedSlot::get), Some(&1));

    drop(batch);
    assert_eq!(producer.free_capacity(), 2);
  }

  #[test]
  fn draining_an_empty_ring_yields_an_empty_batch_and_moves_nothing() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (_producer, mut consumer) = ends.split();

    assert!(consumer.is_empty());
    let batch = consumer.drain();
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
    drop(batch);
    assert_eq!(consumer.position(), Seq::ZERO);
  }

  #[test]
  fn drain_up_to_zero_takes_nothing_and_leaves_the_records_drainable() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    producer.push(1).expect("room");
    assert!(!consumer.is_empty(), "nothing was pending, so taking nothing proves nothing");
    assert!(consumer.drain_up_to(0).is_empty());
    assert_eq!(consumer.drain().len(), 1);
  }

  #[test]
  /// Fix(weak_len_assert_sweep_1633):
  /// Root Cause: the assertion checked only `.len() == 4`, never the batch's
  /// actual contents or their order.
  /// Why Not Caught: a `start`-offset miscalculation could cap the batch at
  /// the right length by coincidence, for example by beginning one slot early
  /// or late, or by wrapping into a stale/already-taken slot. It would still
  /// report `len() == 4` and pass silently.
  /// Fix Applied: drain each slot's value via `get(offset).and_then(
  /// TypedSlot::get).copied()` (the same non-consuming read idiom
  /// `ring_spsc`'s `get_and_iter_agree_at_every_offset` test uses) and assert
  /// the exact pushed sequence `[1, 2, 3, 4]`, in order, in addition to the
  /// length.
  /// Prevention: prefer content/order assertions over bare length checks for
  /// any collection-returning API, especially one whose whole contract (as
  /// this test's own name states) is about *which* elements are returned,
  /// not merely how many.
  /// Pitfall: `TypedSlot::get` is declared `pub const fn`, not `pub fn`, so a
  /// plain `grep "pub fn"` sweep for its API silently misses it.
  fn drain_up_to_more_than_capacity_is_capped_rather_than_scanning_past_the_ring() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    for value in 1..=4 {
      producer.push(value).expect("room");
    }

    let batch = consumer.drain_up_to(usize::MAX);
    assert_eq!(batch.len(), 4);

    let values: Vec<u8> = (0..batch.len())
      .filter_map(|offset| batch.get(offset).and_then(TypedSlot::get).copied())
      .collect();
    assert_eq!(values, vec![1, 2, 3, 4]);
  }

  // ───────────────────────────────────────────────────────────────────────────
  // Laps, the property the stamp's width buys.
  // ───────────────────────────────────────────────────────────────────────────

  /// A stamp from the previous lap is not mistaken for this lap's publication.
  ///
  /// The reason the stamp is a full `Seq` rather than a ready flag. Slot `i`
  /// carries sequences `i`, `i + capacity`, `i + 2·capacity`, …, a different
  /// value each lap. So a stale stamp fails the drain's equality test for the
  /// same reason a never-written one does, with no clearing step on the
  /// consumer's hot path.
  #[test]
  fn a_stale_stamp_from_the_previous_lap_does_not_read_as_published() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(2));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    producer.push(1).expect("room");
    producer.push(2).expect("room");
    drop(consumer.drain());

    // Slot 0 still stamps `Seq( 0 )` from the first lap; the drain is now
    // looking for `Seq( 2 )`.
    assert_eq!(producer.ring().stamps()[0].load(Ordering::Relaxed), Seq::ZERO);
    assert_eq!(consumer.available(), 0);

    producer.push(3).expect("room");
    assert_eq!(producer.ring().stamps()[0].load(Ordering::Relaxed), Seq(2));
    assert_eq!(consumer.available(), 1);
  }

  #[test]
  fn every_slot_is_reused_across_many_laps_without_loss_or_duplication() {
    const LAPS: u64 = 500;
    const CAPACITY: u64 = 8;

    let mut ring: Ring<TypedSlot<u64>> = Ring::new(capacity(CAPACITY as usize));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    let mut received = Vec::new();

    for lap in 0..LAPS {
      for index in 0..CAPACITY {
        producer.push(lap * CAPACITY + index).expect("the previous lap was drained");
      }

      let mut batch = consumer.drain();
      assert_eq!(batch.len(), CAPACITY as usize);

      for offset in 0..batch.len() {
        received.push(batch.get_mut(offset).and_then(TypedSlot::take).expect("published"));
      }
    }

    let expected: Vec<u64> = (0..LAPS * CAPACITY).collect();
    assert_eq!(received, expected);
  }

  #[test]
  fn stamps_start_unstamped_and_there_is_exactly_one_per_slot() {
    let ring: Ring<TypedSlot<u8>> = Ring::new(capacity(16));

    assert_eq!(ring.stamps().len(), 16);
    assert!(ring.stamps().iter().all(|s| s.load(Ordering::Relaxed) == UNSTAMPED));
    assert_eq!(ring.published_through(), None);
    assert_eq!(ring.committed(), Seq::ZERO);
  }

  // ───────────────────────────────────────────────────────────────────────────
  // Record accounting, the class of defect no test above can see.
  // ───────────────────────────────────────────────────────────────────────────

  static DROPPED: AtomicUsize = AtomicUsize::new(0);

  #[derive(Default)]
  struct Tracked;

  impl Drop for Tracked {
    fn drop(&mut self) {
      DROPPED.fetch_add(1, Ordering::Relaxed);
    }
  }

  /// Every record written is destroyed exactly once.
  ///
  /// A ring that forgot its records would pass every other test in this file,
  /// and so would one that dropped them twice on a platform tolerant enough not
  /// to abort. Byte-parity checks that the right values arrived; it says nothing
  /// about what happened to the storage they arrived in.
  ///
  /// Two laps at capacity two. The second lap's `set` replaces the first lap's
  /// records, which must destroy exactly the two the consumer left behind.
  #[test]
  fn every_record_written_is_destroyed_exactly_once() {
    DROPPED.store(0, Ordering::Relaxed);

    {
      let mut ring: Ring<TypedSlot<Tracked>> = Ring::new(capacity(2));
      let mut ends = ring.ends();
      let (producer, mut consumer) = ends.split();

      for _ in 0..2 {
        producer.push(Tracked).expect("room");
        producer.push(Tracked).expect("room");
        // Drained but not taken, so the records stay in their slots.
        drop(consumer.drain());
      }

      assert_eq!(
        DROPPED.load(Ordering::Relaxed),
        2,
        "the second lap's writes destroyed the first lap's undrained records"
      );
    }

    assert_eq!(
      DROPPED.load(Ordering::Relaxed),
      4,
      "the ring going out of scope destroyed the two records still in it"
    );
  }

  /// A record taken out of a batch is moved, not copied.
  #[test]
  fn a_taken_record_leaves_its_slot_empty() {
    DROPPED.store(0, Ordering::Relaxed);

    {
      let mut ring: Ring<TypedSlot<Tracked>> = Ring::new(capacity(2));
      let mut ends = ring.ends();
      let (producer, mut consumer) = ends.split();

      producer.push(Tracked).expect("room");

      let mut batch = consumer.drain();
      let taken = batch.get_mut(0).and_then(TypedSlot::take);
      assert!(taken.is_some());
      assert!(batch.get(0).is_some_and(Slot::is_empty), "the slot is empty after the take");
      assert_eq!(DROPPED.load(Ordering::Relaxed), 0, "still alive in the caller's hand");

      drop(batch);
      drop(taken);
      assert_eq!(DROPPED.load(Ordering::Relaxed), 1);
    }

    assert_eq!(DROPPED.load(Ordering::Relaxed), 1, "the ring held nothing more to destroy");
  }

  /// A non-`Copy` payload survives the round trip intact.
  #[test]
  fn a_heap_payload_arrives_with_its_contents_rather_than_a_shallow_copy() {
    let mut ring: Ring<TypedSlot<String>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    producer.push("the first".to_string()).expect("room");
    producer.push("the second".to_string()).expect("room");

    let mut batch = consumer.drain();
    assert_eq!(batch.get_mut(0).and_then(TypedSlot::take).as_deref(), Some("the first"));
    assert_eq!(batch.get_mut(1).and_then(TypedSlot::take).as_deref(), Some("the second"));
  }

  // ───────────────────────────────────────────────────────────────────────────
  // Public API details.
  // ───────────────────────────────────────────────────────────────────────────

  #[test]
  fn a_bytes_payload_round_trips_its_written_length() {
    let mut ring: Ring<BytesSlot<16>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    let mut reserved = producer.claim().expect("room");
    reserved.write(b"payload").expect("seven bytes fit in sixteen");
    drop(reserved);

    let batch = consumer.drain();
    assert_eq!(batch.get(0).map(BytesSlot::read), Some(&b"payload"[..]));
  }

  #[test]
  fn a_config_supplies_the_capacity_and_its_other_fields_are_deliberately_unread() {
    // `with_producers` is the field a ring might be tempted to trust. It is
    // not read. The claim is a compare-exchange, correct for any number of
    // producers because of its shape rather than because it was told one.
    let config = RingConfig::new(8).expect("a power of two").with_producers(64);
    let ring: Ring<TypedSlot<u8>> = Ring::with_config(&config);

    assert_eq!(ring.capacity(), config.capacity());
    assert_eq!(ring.stamps().len(), 8);
  }

  #[test]
  fn a_capacity_that_is_not_a_power_of_two_is_refused_before_a_ring_exists() {
    assert!(Capacity::new(3).is_err());
    assert!(RingConfig::new(100).is_err());
  }

  #[test]
  fn the_batch_reports_the_sequences_it_covers() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    producer.push(1).expect("room");
    producer.push(2).expect("room");
    drop(consumer.drain_up_to(1));

    producer.push(3).expect("room");
    let batch = consumer.drain();
    assert_eq!(batch.sequences().collect::<Vec<_>>(), vec![Seq(1), Seq(2)]);
    assert_eq!(batch.iter().count(), 2);
  }

  #[test]
  fn reading_past_the_end_of_a_batch_yields_none_rather_than_the_next_lap() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    producer.push(1).expect("room");

    let mut batch = consumer.drain();
    assert!(batch.get(1).is_none());
    assert!(batch.get_mut(1).is_none());
  }

  #[test]
  fn the_claim_advances_before_the_publish_and_the_two_are_separate_cursors() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, _consumer) = ends.split();

    let reserved = producer.claim().expect("room");
    assert_eq!(producer.claimed(), Seq(1), "claimed");
    assert_eq!(producer.ring().published_through(), None, "and not yet published");

    drop(reserved);
    assert_eq!(producer.ring().published_through(), Some(Seq::ZERO));
  }

  #[test]
  fn the_orderings_are_the_ones_the_publication_invariant_names() {
    // The publication-ordering invariant states these as contract, in one
    // place, rather than at each use. This is the assertion that the constants
    // have not drifted from it.
    assert_eq!(PUBLISH, Ordering::Release);
    assert_eq!(OBSERVE, Ordering::Acquire);
    assert_eq!(COMMIT, Ordering::Release);
    assert_eq!(OWN, Ordering::Relaxed);
    assert_eq!(ring_cursor::GATING, Ordering::Acquire);
  }

  #[test]
  fn the_debug_rendering_names_the_ring_state_a_reader_would_want() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, _consumer) = ends.split();
    producer.push(1).expect("room");

    let rendered = format!("{:?}", producer.ring());
    assert!(rendered.contains("capacity: 4"), "{rendered}");
    assert!(rendered.contains("published_through: Some"), "{rendered}");
  }

  /// A producer can read back what it wrote before publishing it.
  ///
  /// `Reserved` derefs both ways. The mutable half is how a payload is written,
  /// and every other test exercises it. The shared half lets a producer that
  /// builds a record incrementally check its own work. It reads a slot nobody
  /// else may touch, since the slot is claimed and unstamped.
  #[test]
  fn a_claimed_slot_is_readable_through_the_guard_before_it_is_published() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, consumer) = ends.split();

    let mut reserved = producer.claim().expect("room");
    assert_eq!(reserved.get(), None, "a reclaimed slot starts empty");

    reserved.set(42);
    assert_eq!(reserved.get(), Some(&42), "read back through Deref, not DerefMut");
    assert_eq!(consumer.available(), 0, "and still invisible to the consumer");
  }

  /// Both handles name the ring they act on, and it is the same ring.
  #[test]
  fn both_ends_and_the_handles_they_split_into_name_one_ring() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(8));
    let mut ends = ring.ends();
    assert_eq!(ends.ring().capacity().get(), 8);

    let (producer, consumer) = ends.split();

    assert_eq!(producer.ring().capacity().get(), 8);
    assert_eq!(consumer.ring().capacity().get(), 8);
    assert!(
      core::ptr::eq(producer.ring(), consumer.ring()),
      "the two ends address one allocation — that is what the unsafe rests on"
    );
  }

  /// An explicitly cloned producer is the same producer.
  ///
  /// `Producer` is `Copy`, so `clone` is never reached implicitly. It exists
  /// because `Clone` is `Copy`'s supertrait, and a caller writing generic code
  /// over `T : Clone` would find it. So it is tested rather than assumed
  /// correct for being trivial.
  ///
  /// `clone_on_copy` is allowed rather than obeyed. Obeying it would delete the
  /// only call site the explicit impl has, which is the thing under test.
  #[test]
  #[allow(clippy::clone_on_copy)]
  fn a_cloned_producer_shares_the_claim_cursor_rather_than_starting_a_new_one() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    let second = producer.clone();
    producer.push(1).expect("room");
    second.push(2).expect("room");

    assert_eq!(producer.claimed(), Seq(2), "one cursor, advanced twice");
    assert_eq!(second.claimed(), Seq(2));

    let mut batch = consumer.drain();
    assert_eq!(batch.len(), 2);
    assert_eq!(batch.get_mut(0).and_then(TypedSlot::take), Some(1));
    assert_eq!(batch.get_mut(1).and_then(TypedSlot::take), Some(2));
  }

  /// The primary handle mints from the shared claim cursor, coexists with
  /// ordinary `Copy` producers, and its grants are disjoint from theirs —
  /// the compare-exchange arbitrates, exactly as it does between copies.
  #[test]
  fn a_primary_producer_and_a_copy_producer_grant_disjoint_sequences() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (mut producer, mut consumer) = ends.split();

    producer.push(1).expect("room"); // sequence 0, the ordinary path

    let mut primary = producer.primary();
    primary.try_push(2).expect("room"); // sequence 1, the cached fast path

    producer.push(3).expect("room"); // sequence 2, the ordinary path again

    assert_eq!(primary.claimed(), Seq(3));
    assert_eq!(producer.claimed(), Seq(3), "one cursor, three grants");

    let mut batch = consumer.drain();
    assert_eq!(batch.len(), 3);
    assert_eq!(batch.get_mut(0).and_then(TypedSlot::take), Some(1));
    assert_eq!(batch.get_mut(1).and_then(TypedSlot::take), Some(2));
    assert_eq!(batch.get_mut(2).and_then(TypedSlot::take), Some(3));
  }

  /// The cached head can only lag the consumer, so after the cache reports
  /// full the next claim refreshes it against the consumer's commit and
  /// succeeds — the conservative refusal never wedges the handle.
  #[test]
  fn the_primary_cache_is_refreshed_before_the_claim_refuses() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(2));
    let mut ends = ring.ends();
    let (mut producer, mut consumer) = ends.split();
    let mut primary = producer.primary();

    primary.try_push(1).expect("room");
    primary.try_push(2).expect("room");
    assert_eq!(
      primary.claim().err(),
      Some(RingError::Full),
      "the cache says full, and the ring is"
    );

    drop(consumer.drain());

    primary.try_push(3).expect("the refresh finds the room the drain made");
    primary.try_push(4).expect("room");

    let mut batch = consumer.drain();
    assert_eq!(batch.len(), 2);
    assert_eq!(batch.get_mut(0).and_then(TypedSlot::take), Some(3));
    assert_eq!(batch.get_mut(1).and_then(TypedSlot::take), Some(4));
  }

  /// A primary reservation leaked with `mem::forget` keeps its sequence
  /// unpublished: the claim's exchange moved the shared cursor past it, and
  /// without the drop there is no stamp, so the consumer stalls at the hole
  /// and every later record is unreachable behind it. This is the ring's
  /// documented lost-claim pitfall, pinned for the cached handle: the loss
  /// lives in the missing publish, not in the cursor arithmetic, and the
  /// cache changes nothing about it.
  #[test]
  fn a_leaked_primary_reservation_stalls_the_drain_at_its_hole() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (mut producer, mut consumer) = ends.split();
    let mut primary = producer.primary();

    primary.try_push(1).expect("room"); // sequence 0, published

    std::mem::forget(primary.claim().expect("room")); // sequence 1: cursor moved, no stamp

    primary.try_push(2).expect("room"); // sequence 2, published past the hole

    assert_eq!(consumer.drain().len(), 1, "only the record before the hole drains",);
    assert_eq!(consumer.drain().len(), 0, "the hole never fills");
  }

  #[test]
  fn a_reserved_guard_reports_the_sequence_it_will_publish() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(capacity(4));
    let mut ends = ring.ends();
    let (producer, _consumer) = ends.split();

    let first: Reserved<'_, TypedSlot<u8>> = producer.claim().expect("room");
    let second = producer.claim().expect("room");

    assert_eq!(first.sequence(), Seq::ZERO);
    assert_eq!(second.sequence(), Seq(1));
  }

  /// A producer thread that panics while holding `granted`'s lock does not
  /// poison a still-running sibling producer's own access to it.
  ///
  /// Root Cause: `four_producers_exchange_one_hundred_thousand_items_with_byte_parity`
  /// shares one `Mutex< Vec< Seq > >` (`granted`) across its four producer
  /// threads and one `Mutex< Vec< u64 > >` (`received`) with its consumer
  /// thread, and used to acquire both with
  /// `.expect( "no panic while holding the lock" )`. `std::sync::Mutex`
  /// poisons on *any* panic while a guard is held, by *any* thread. A panic
  /// in one producer's tiny critical section (an allocator failure inside
  /// `Vec::extend`, say) would have poisoned `granted` and turned every other
  /// still-running producer's own `.expect()` into a panic too, each with the
  /// same confusing "no panic while holding the lock" message rather than
  /// the first producer's real one.
  ///
  /// Why Not Caught: `granted`/`received` are locals of one `#[ test ]`
  /// function, not reachable from any other test, so nothing exercises a
  /// panic mid-critical-section for them; the crate's own parity test only
  /// ever runs its four producers and one consumer to completion.
  ///
  /// Fix Applied: both locks in the real test now take
  /// `.unwrap_or_else( std::sync::PoisonError::into_inner )` instead of
  /// `.expect(...)`, recovering the stale-but-valid guard rather than
  /// panicking. This does not change whether a real panic still fails the
  /// test, because `std::thread::scope` re-panics once every spawned thread
  /// is joined, regardless of what any lock did. It only stops sibling
  /// threads from adding confusing poisoned-lock panics on top of the real
  /// one. This test reproduces the identical shape (`Mutex< Vec< Seq > >`,
  /// the same `granted` element type) with a lock-then-panic thread standing
  /// in for a producer, and a locker that runs after it standing in for a
  /// sibling producer, to prove the idiom recovers.
  ///
  /// Prevention: the guarded critical section in both the real test and here
  /// never runs anything but `Vec::extend` on plain `Copy` elements. There is
  /// no user callback and no `Drop` impl that could itself panic, so there is
  /// no half-established invariant poisoning could ever be protecting.
  /// Recovering is unconditionally safe for this shape of lock.
  ///
  /// Pitfall: a `Mutex` local to one test function still has live sibling
  /// threads racing on it for the duration of that function's own
  /// `thread::scope` block. "It cannot outlive the test" says nothing about
  /// whether it can poison a sibling thread *during* the test, and that is
  /// the window this defect class targets.
  #[test]
  fn a_sibling_producer_recovers_a_lock_poisoned_by_another_producers_panic() {
    // Mirrors `granted`'s own type from the parity test above exactly.
    let granted: Mutex<Vec<Seq>> = Mutex::new(vec![Seq::ZERO]);

    // Suppress the panic hook's stderr backtrace for the two intentional
    // panics below.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let granted_ref = &granted;
    let poisoned = std::thread::scope(|scope| {
      scope
        .spawn(move || {
          let _guard = granted_ref.lock().unwrap();
          panic!("simulated allocator failure inside one producer's extend");
        })
        .join()
    });

    assert!(
      poisoned.is_err(),
      "the spawned producer must actually have panicked while locked"
    );
    assert!(
      granted.is_poisoned(),
      "a panic while holding the lock must poison it for every sibling"
    );

    // The bug. The pre-fix `.expect( "no panic while holding the lock" )`
    // would panic here too, so one producer's unrelated panic would cascade
    // into a different, still-healthy sibling's own access.
    let old_pattern_would_panic = std::panic::catch_unwind(|| {
      drop(granted.lock().expect("no panic while holding the lock"));
    });
    assert!(
      old_pattern_would_panic.is_err(),
      "the pre-fix `.expect(...)` idiom must panic on a poisoned lock — this is the bug",
    );

    std::panic::set_hook(hook);

    // The fix: `.unwrap_or_else( PoisonError::into_inner )`, the real test's
    // own idiom after this fix, recovers the stale-but-valid guard instead.
    let recovered = granted.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(
      recovered.as_slice(),
      &[Seq::ZERO],
      "the prior state survives an unrelated sibling panic"
    );
  }
}

/// `loom` models of the publication protocol.
///
/// Compiled only under `RUSTFLAGS="--cfg loom"`; an ordinary build never sees
/// this module and never pays for it. `ring_atomic` makes it work. It swaps
/// `AtomicSeq` for an instrumented one under the same cfg, so what loom
/// explores is this crate's own stamp protocol rather than a re-implementation
/// of it.
///
/// # Why the assertion is over a separate atomic
///
/// A slot's payload lives behind an `UnsafeCell`, which is plain memory loom
/// does not model. A model asserting "the drained record carried the right
/// value" would therefore pass identically whether the publish released or
/// not. That is a green check that was never capable of being red. So the
/// payload here is a loom `AtomicUsize` stored *before* the publish and loaded
/// *after* the drain, which makes the thing whose visibility is checked
/// something loom can see.
/// `ring_publish/tests/handshake_test.rs` establishes the shape.
#[cfg(loom)]
mod exhaustive {
  use loom::sync::atomic::{AtomicUsize, Ordering};
  use ring_mpsc::{Ends, Ring};
  use ring_slot::TypedSlot;
  use ring_types::Capacity;

  const WRITTEN: usize = 0xABC;

  /// Ends that outlive the model's threads.
  ///
  /// `loom::thread::spawn` requires `'static` closures, so nothing a model
  /// spawns may borrow a local. The ring and its ends are therefore leaked
  /// rather than scoped. That is one leak per execution, which is what loom's
  /// own test runner expects and why its models are kept to two slots.
  fn leaked_ends() -> &'static mut Ends<'static, TypedSlot<u8>> {
    let capacity = Capacity::new(2).expect("a power of two");
    let ring: &'static mut Ring<TypedSlot<u8>> = Box::leak(Box::new(Ring::new(capacity)));

    Box::leak(Box::new(ring.ends()))
  }

  /// A drained record never precedes the write that came before its publish.
  ///
  /// Weakening `PUBLISH` from `Release` to `Relaxed` must fail this model. That
  /// mutation is step 1 of `tests/manual/readme.md`'s M9, and a model that
  /// still passes under it is checking nothing.
  #[test]
  fn a_published_record_is_never_observed_before_the_write_that_preceded_it() {
    loom::model(|| {
      let payload = &*Box::leak(Box::new(AtomicUsize::new(0)));
      let (producer, mut consumer) = leaked_ends().split();

      let writer = loom::thread::spawn(move || {
        payload.store(WRITTEN, Ordering::Relaxed);
        drop(producer.claim().expect("an empty ring has room"));
      });

      let reader = loom::thread::spawn(move || {
        if consumer.drain().is_empty() {
          return;
        }
        assert_eq!(
          payload.load(Ordering::Relaxed),
          WRITTEN,
          "a drained record did not carry the write that preceded its publish"
        );
      });

      writer.join().expect("no panic");
      reader.join().expect("no panic");
    });
  }

  /// The consumer never sees further than what was published.
  ///
  /// Two producers publishing in either order; the drain must never report
  /// more records than were published, whichever interleaving loom picks.
  #[test]
  fn the_consumer_never_drains_further_than_the_producers_published() {
    loom::model(|| {
      let (producer, mut consumer) = leaked_ends().split();
      let second = producer;

      let first_writer = loom::thread::spawn(move || {
        drop(producer.claim().expect("an empty ring has room"));
      });

      let second_writer = loom::thread::spawn(move || {
        drop(second.claim().expect("a two-slot ring has room for two"));
      });

      let reader = loom::thread::spawn(move || {
        let first = consumer.drain().len();
        let second = consumer.drain().len();
        assert!(first + second <= 2, "drained {} of at most 2", first + second);
      });

      first_writer.join().expect("no panic");
      second_writer.join().expect("no panic");
      reader.join().expect("no panic");
    });
  }

  /// A batch publish never observes a payload before the write that preceded
  /// its stamp.
  ///
  /// The batched drop stores two stamps where the single guard stores one;
  /// the visibility argument of `docs/invariant/002_publication_ordering.md`
  /// must hold for the pair, under every interleaving loom can pick. As in
  /// the models above, the payload rides on loom atomics — the slot's own
  /// memory is `UnsafeCell` loom does not model — and record `i` of a drained
  /// batch may be observed only after the payload store that preceded its
  /// stamp became visible.
  ///
  /// Mutation, M9-style: weakening `PUBLISH` to `Relaxed` must fail this
  /// model — with a relaxed store the consumer can observe a stamp before
  /// the payload store that preceded it, and the assertion fires.
  #[test]
  fn a_batch_publish_never_observes_a_payload_before_the_write_that_preceded_its_stamp() {
    loom::model(|| {
      let payloads: &'static [AtomicUsize; 2] = Box::leak(Box::new([AtomicUsize::new(0), AtomicUsize::new(0)]));
      let (producer, mut consumer) = leaked_ends().split();

      let writer = loom::thread::spawn(move || {
        payloads[0].store(WRITTEN, Ordering::Relaxed);
        payloads[1].store(WRITTEN, Ordering::Relaxed);
        drop(producer.claim_batch(2).expect("an empty ring has room for the whole grant"));
      });

      let reader = loom::thread::spawn(move || {
        let drained = consumer.drain();
        for offset in 0..drained.len() {
          assert_eq!(
            payloads[offset].load(Ordering::Relaxed),
            WRITTEN,
            "drained record {offset} did not carry the write that preceded its stamp"
          );
        }
      });

      writer.join().expect("no panic");
      reader.join().expect("no panic");
    });
  }

  /// The consumer never drains further than a batch grant published.
  ///
  /// One producer claiming two sequences publishes two stamps from one drop;
  /// whichever order the scan observes them in, across as many drains as it
  /// takes, the total can never exceed the grant's length.
  #[test]
  fn the_consumer_never_drains_further_than_a_batch_grant_published() {
    loom::model(|| {
      let (producer, mut consumer) = leaked_ends().split();

      let writer = loom::thread::spawn(move || {
        drop(producer.claim_batch(2).expect("an empty ring has room for the whole grant"));
      });

      let reader = loom::thread::spawn(move || {
        let first = consumer.drain().len();
        let second = consumer.drain().len();
        assert!(first + second <= 2, "drained {} of at most 2", first + second);
      });

      writer.join().expect("no panic");
      reader.join().expect("no panic");
    });
  }
}
