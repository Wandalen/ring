//! `ring_spsc` — one producer, one consumer, one array between them.
//!
//! This file carries the reached-test for `docs/feature/171_spsc_ring_api.md`,
//! stated in `ring/bench_harness/docs/acceptance/001_feature_reached_tests.md`
//! as one sentence with four clauses: one producer and one consumer exchange
//! 100 000 items with **byte-parity** between what was written and what was
//! read, **in order**, with **zero loss** and **no lock in the path**.
//!
//! ## The four clauses are four different failures
//!
//! Byte-parity, order and loss are each independently violable, which is why
//! the reached-test asserts all three of a single run rather than one of them
//! three times:
//!
//! - A ring that folds a sequence to the wrong slot delivers 100 000 records,
//!   in order, none lost, and every one of them the wrong payload.
//! - A ring whose consumer computes its bound from a stale cursor delivers
//!   correct payloads in correct order and silently stops short.
//! - A ring that lets the producer lap the consumer delivers correct payloads
//!   in order with the right *count* — and some of them twice, with the
//!   overwritten ones gone.
//!
//! The fourth clause is structural rather than behavioural: no test run can
//! demonstrate the absence of a lock, so it is asserted at the source in
//! `tests/manual/readme.md`'s S2 and by the crate's own dependency list — no
//! `std::sync::Mutex`, no `RwLock`, and no compare-exchange on either path.
//!
//! ## What the doc examples do not cover
//!
//! Every method has a doc example, and the doc tests run — but each uses a
//! capacity of 1 to 8 and pushes a handful of items, so none of them ever
//! wraps the ring more than once, fills it, or crosses a thread boundary under
//! real contention. Wrap, saturation and contention are where a ring's
//! interesting bugs live, and they are what this file is for.
//!
//! ## What an ordinary run cannot cover either
//!
//! The reached-test exchanges 100 000 items on a real machine, and it passes
//! against a ring whose publish used `Relaxed` where it should use `Release` —
//! measured, not assumed (→ `tests/manual/readme.md` S9). Scale is not
//! coverage.
//!
//! Note the reason is *not* that the host is strongly ordered. It is aarch64
//! (Neoverse-N1), weakly ordered — `rustc -vV | grep host`. The window is
//! simply narrow enough that sampling it 100 000 times does not open it. The `#[ cfg( loom ) ] mod exhaustive` at the bottom
//! of this file runs a deliberately tiny case — one push, one drain — against a
//! memory model weaker than any real hardware, and checks *every* interleaving
//! of it. Neither subsumes the other: one has scale without coverage, the other
//! coverage without scale.
//!
//! ```text
//! RUSTFLAGS="--cfg loom" cargo test -p ring_spsc --test spsc_test
//! ```

// ── the handshake at a scale loom cannot enumerate ─────────────────────────
//
// Gated `not( loom )` because loom's atomics panic when touched outside a
// `loom::model`, and every test below constructs a `Ring` — whose cursors are
// those atomics under `--cfg loom`. The two halves of this file are therefore
// never compiled together, which is also why neither can quietly stand in for
// the other.
#[cfg(not(loom))]
mod threaded {
    use core::sync::atomic::{AtomicUsize, Ordering};

    use ring_slot::{BytesSlot, TypedSlot};
    use ring_spsc::{Batch, Consumer, Producer, Reservation, Ring};
    use ring_types::{Capacity, RingError, Seq};

    /// A capacity, or a panic naming the value that was refused.
    fn cap(slots: usize) -> Capacity {
        Capacity::new(slots).unwrap_or_else(|_| panic!("{slots} is not a valid capacity"))
    }

    // ── the reached-test ───────────────────────────────────────────────────────

    /// `docs/feature/171_spsc_ring_api.md` — the claiming test.
    ///
    /// 100 000 records of eight bytes each, produced on one thread and drained on
    /// another, checked for all three behavioural clauses at once.
    ///
    /// Each record's payload is derived from its own index, so byte-parity is
    /// checked against a value the consumer computes independently rather than
    /// against a copy the producer handed over — a shared `Vec` of expected
    /// payloads would pass just as readily if both ends agreed on the wrong thing.
    #[test]
    fn one_producer_and_one_consumer_exchange_one_hundred_thousand_items() {
        const ITEMS: u64 = 100_000;
        const CAPACITY: usize = 1_024;

        fn payload(index: u64) -> [u8; 8] {
            // Mixed rather than a plain little-endian index, so that a fold error
            // landing on a neighbouring slot produces bytes that differ in every
            // position rather than in one.
            (index.wrapping_mul(0x9E37_79B9_7F4A_7C15)).to_le_bytes()
        }

        let mut ring: Ring<BytesSlot<8>> = Ring::new(cap(CAPACITY));
        let (mut producer, mut consumer) = ring.split();

        let received = std::thread::scope(|scope| {
            let producing = scope.spawn(move || {
                let mut next = 0;

                while next < ITEMS {
                    // No lock and no park: a full ring is reported, and the producer
                    // yields rather than blocking on the consumer's progress.
                    match producer.claim() {
                        Ok(mut slot) => {
                            slot.write(&payload(next))
                                .expect("eight bytes into an eight-byte slot");
                            next += 1;
                        },
                        Err(RingError::Full) => std::thread::yield_now(),
                        Err(other) => panic!("a producer can only ever be Full, got {other:?}"),
                    }
                }
            });

            let mut received = Vec::with_capacity(ITEMS as usize);

            // **The exit condition is "the producer has finished and the ring is
            // empty", not "I have 100 000 records."** The difference is what a broken
            // ring looks like when this test runs: a consumer waiting for a count it
            // will never reach hangs, and a hang in CI reads as a slow machine rather
            // than as a defect. Measured — a mutation committing one sequence past
            // what it read made this loop spin forever under the count-based form, and
            // fail in 30ms with a readable count mismatch under this one.
            while (received.len() as u64) < ITEMS {
                // Sampled *before* the drain, and that order is the whole correctness of
                // this exit. If the producer has already terminated at this point, the
                // drain below sees everything it will ever publish. Sampling after the
                // drain would race: the producer could publish between the two and the
                // loop would break having missed it, turning a passing ring into a
                // flaky count mismatch.
                let producer_done = producing.is_finished();

                let batch = consumer.drain();

                if batch.is_empty() {
                    if producer_done {
                        // Nothing more can arrive. Whatever is missing is missing.
                        break;
                    }

                    std::thread::yield_now();
                    continue;
                }

                // Copied out inside the batch's life, which is what the borrow shape is
                // for: the slots are reusable the instant the batch drops.
                for slot in batch.iter() {
                    received
                        .push(<[u8; 8]>::try_from(slot.read()).expect("a full eight-byte record"));
                }
            }

            producing.join().expect("the producer never panics");

            received
        });

        // Zero loss, and nothing extra.
        assert_eq!(received.len() as u64, ITEMS, "count");

        // Byte-parity and order, together — position `index` must hold exactly the
        // payload of `index`, so a reordering and a corruption both fail here.
        for (index, actual) in received.iter().enumerate() {
            assert_eq!(*actual, payload(index as u64), "record {index}");
        }
    }

    // ── the invariant the unsafe rests on ──────────────────────────────────────

    /// Decision 123 ruling 4 — the shape asserted, not only described.
    ///
    /// The soundness argument in `Ring::slot_mut` says "the caller must be the
    /// producer end", and it is worth nothing if a second producer is
    /// constructible. Three properties make that unrepresentable, and all three are
    /// negative — which is why they need a test: nothing in the code says them.
    #[test]
    fn both_ends_are_send_and_neither_is_sync() {
        fn assert_send<T: Send>() {}

        assert_send::<Producer<'_, TypedSlot<u32>>>();
        assert_send::<Consumer<'_, TypedSlot<u32>>>();
        assert_send::<Ring<TypedSlot<u32>>>();

        // The negative half — `!Sync`, `!Clone`, no second split, no batch outliving
        // its commit — is asserted by four `compile_fail` doc tests in the crate's
        // module documentation, under "What the type system refuses". They live there
        // rather than here because rustdoc collects doc tests from the library target
        // only: the same blocks written in this file are never compiled, and would be
        // four checks that silently check nothing.
    }

    // ── construction ───────────────────────────────────────────────────────────

    #[test]
    fn a_new_ring_is_empty_and_fully_free() {
        let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(8));

        assert_eq!(ring.capacity().get(), 8);
        assert!(ring.on_distinct_lines());

        let (producer, consumer) = ring.split();

        assert_eq!(producer.position(), Seq::ZERO);
        assert_eq!(consumer.position(), Seq::ZERO);
        assert_eq!(producer.free_capacity(), 8);
        assert_eq!(consumer.available(), 0);
        assert!(!producer.is_full());
        assert!(consumer.is_empty());
    }

    #[test]
    fn with_config_takes_the_capacity_and_ignores_the_rest() {
        use ring_config::RingConfig;
        use ring_types::{OverflowPolicy, WaitKind};

        // The wait strategy and overflow policy describe what a *caller* does; this
        // crate never waits and never drops, so a config carrying either must produce
        // the same ring as one that does not.
        let plain = RingConfig::new(16).unwrap();
        let decorated = RingConfig::new(16)
            .unwrap()
            .with_wait(WaitKind::Spin)
            .with_overflow(OverflowPolicy::DropNewest);

        let from_plain: Ring<TypedSlot<u8>> = Ring::with_config(&plain);
        let from_decorated: Ring<TypedSlot<u8>> = Ring::with_config(&decorated);

        assert_eq!(from_plain.capacity(), from_decorated.capacity());
        assert_eq!(from_plain.capacity().get(), 16);
    }

    #[test]
    fn debug_reports_positions_and_never_reads_a_slot() {
        // Formatting the slots would read every one of them, including any the
        // producer is writing — so the `Debug` impl deliberately does not. This
        // asserts the shape of the output, which is the only observable difference.
        let mut ring: Ring<TypedSlot<u32>> = Ring::new(cap(4));
        let (mut producer, _consumer) = ring.split();

        producer.try_push(7).unwrap();
        producer.try_push(8).unwrap();

        let rendered = format!("{ring:?}");

        assert!(rendered.contains("capacity: 4"), "{rendered}");
        assert!(rendered.contains("produced: Seq(2)"), "{rendered}");
        assert!(rendered.contains("consumed: Seq(0)"), "{rendered}");
        assert!(!rendered.contains('7'), "a payload leaked into Debug: {rendered}");
    }

    // ── the producer's surface ─────────────────────────────────────────────────

    #[test]
    fn free_capacity_is_actionable_rather_than_advisory() {
        // The SPSC-specific contract from `docs/api/001_producer_surface.md`: with
        // one producer nothing can take the reported space between the check and the
        // push, so a reported `n` guarantees exactly `n` successes and then a
        // failure. On a multi-producer ring the same call is a hint, which is the
        // pitfall instance's whole subject.
        const CAPACITY: usize = 8;

        let mut ring: Ring<TypedSlot<u32>> = Ring::new(cap(CAPACITY));
        let (mut producer, mut consumer) = ring.split();

        for already in 0..CAPACITY {
            let reported = producer.free_capacity();
            assert_eq!(reported, CAPACITY - already, "after {already} pushes");

            producer.try_push(already as u32).expect("the report promised this one");
        }

        assert_eq!(producer.free_capacity(), 0);
        assert_eq!(producer.try_push(99), Err(99), "and exactly one more fails");

        // The report tracks the consumer too, not only the producer.
        drop(consumer.drain_up_to(3));
        assert_eq!(producer.free_capacity(), 3);
    }

    #[test]
    fn a_full_ring_reports_rather_than_blocks() {
        let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(2));
        let (mut producer, mut consumer) = ring.split();

        producer.try_push(1).unwrap();
        producer.try_push(2).unwrap();

        assert!(producer.is_full());
        assert_eq!(producer.claim().err(), Some(RingError::Full));
        assert_eq!(
            producer.push_with(|slot: &mut TypedSlot<u8>| slot.set(3)).err(),
            Some(RingError::Full)
        );

        // And the failure left nothing behind: a claim that returned Err must not
        // have advanced the cursor, or the ring would have a hole.
        assert_eq!(producer.position(), Seq(2));

        drop(consumer.drain());
        assert!(!producer.is_full());
        assert_eq!(producer.position(), Seq(2), "draining does not move the producer");
    }

    #[test]
    fn a_failed_push_returns_the_record_rather_than_swallowing_it() {
        // A caller applying backpressure has nothing to retry with if the record is
        // consumed by the failure — which is why `try_push` returns `Result< (), T >`
        // rather than `Result< (), RingError >`.
        let mut ring: Ring<TypedSlot<String>> = Ring::new(cap(1));
        let (mut producer, _consumer) = ring.split();

        producer.try_push("kept".to_owned()).unwrap();

        let returned = producer.try_push("rejected".to_owned());

        assert_eq!(returned, Err("rejected".to_owned()));
    }

    #[test]
    fn push_with_does_not_call_the_writer_when_the_ring_is_full() {
        // Otherwise a caller whose closure has a side effect — consuming from an
        // upstream queue, say — loses a record on every rejected push.
        let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(1));
        let (mut producer, _consumer) = ring.split();

        let mut calls = 0;

        producer
            .push_with(|slot: &mut TypedSlot<u8>| {
                calls += 1;
                slot.set(1)
            })
            .unwrap();
        assert_eq!(calls, 1);

        let rejected = producer.push_with(|slot: &mut TypedSlot<u8>| {
            calls += 1;
            slot.set(2)
        });

        assert_eq!(rejected.err(), Some(RingError::Full));
        assert_eq!(calls, 1, "the writer ran on a full ring");
    }

    #[test]
    fn a_reservation_publishes_on_drop_even_unwritten() {
        // Deliberate rather than incidental: publishing an empty slot is recoverable
        // — the consumer sees a record it can recognise as empty — whereas *not*
        // publishing wedges the ring for good. The `must_use` message is what warns
        // about the accident.
        let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(4));
        let (mut producer, consumer) = ring.split();

        drop(producer.claim().unwrap());

        assert_eq!(producer.position(), Seq(1));
        assert_eq!(consumer.available(), 1);
    }

    #[test]
    fn a_reservation_reads_back_what_was_written_through_it() {
        // `Deref` and `DerefMut` must reach the same slot; a `Deref` that folded
        // differently would let a caller write one slot and verify another.
        let mut ring: Ring<BytesSlot<4>> = Ring::new(cap(2));
        let (mut producer, _consumer) = ring.split();

        let mut reservation = producer.claim().unwrap();
        reservation.write(b"ab").unwrap();

        assert_eq!(reservation.read(), b"ab");
        assert_eq!(reservation.len(), 2);
        assert_eq!(reservation.sequence(), Seq::ZERO);
    }

    #[test]
    fn sequences_are_issued_consecutively_across_a_wrap() {
        const CAPACITY: usize = 4;
        const LAPS: u64 = 5;

        let mut ring: Ring<TypedSlot<u64>> = Ring::new(cap(CAPACITY));
        let (mut producer, mut consumer) = ring.split();

        let mut expected = 0;

        for _ in 0..LAPS {
            for _ in 0..CAPACITY {
                assert_eq!(producer.claim().unwrap().sequence(), Seq(expected));
                expected += 1;
            }

            drop(consumer.drain());
        }

        assert_eq!(producer.position(), Seq(LAPS * CAPACITY as u64));
    }

    // ── the consumer's surface ─────────────────────────────────────────────────

    #[test]
    fn draining_an_empty_ring_yields_an_empty_batch_and_moves_nothing() {
        let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(4));
        let (_producer, mut consumer) = ring.split();

        let batch = consumer.drain();

        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
        assert_eq!(batch.start(), Seq::ZERO);
        assert!(batch.get(0).is_none());
        assert_eq!(batch.iter().count(), 0);

        drop(batch);
        assert_eq!(consumer.position(), Seq::ZERO, "an empty commit is still a no-op");
    }

    #[test]
    fn a_batch_commits_exactly_its_own_length() {
        let mut ring: Ring<TypedSlot<u32>> = Ring::new(cap(16));
        let (mut producer, mut consumer) = ring.split();

        for value in 0..10 {
            producer.try_push(value).unwrap();
        }

        let batch = consumer.drain_up_to(4);
        assert_eq!(batch.len(), 4);
        assert_eq!(batch.start(), Seq::ZERO);
        drop(batch);

        assert_eq!(consumer.position(), Seq(4));
        assert_eq!(consumer.available(), 6);

        let rest = consumer.drain();
        assert_eq!(rest.len(), 6);
        assert_eq!(rest.start(), Seq(4));
        drop(rest);

        assert_eq!(consumer.position(), Seq(10));
        assert!(consumer.is_empty());
    }

    #[test]
    fn drain_up_to_zero_is_a_legitimate_no_op() {
        // The boundary a `max` of zero produces: an empty batch over a non-empty
        // ring, committing nothing, leaving everything for the next drain.
        let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(4));
        let (mut producer, mut consumer) = ring.split();

        producer.try_push(1).unwrap();

        assert_eq!(consumer.drain_up_to(0).len(), 0);
        assert_eq!(consumer.position(), Seq::ZERO);
        assert_eq!(consumer.available(), 1, "still there");
    }

    /// Fix(weak_len_assert_sweep_1633):
    /// Root Cause: the assertion checked only `.len() == 2`, never which two
    /// values the over-large `drain_up_to( 100 )` request actually yielded.
    /// Why Not Caught: a wrong `max.min( available )` clamp that read the wrong
    /// bound (e.g. capacity instead of what was actually published) could still
    /// coincidentally report a batch of length 2 while starting at the wrong
    /// offset or reading stale slots.
    /// Fix Applied: read each slot via `get(offset).and_then(TypedSlot::get)
    /// .copied()` — the same non-consuming idiom this file's own
    /// `get_and_iter_agree_at_every_offset` test uses a few lines below — and
    /// assert the exact values `[1, 2]`, in order, in addition to the length.
    /// Prevention: prefer content/order assertions over bare length checks for
    /// any collection-returning API, especially one whose whole contract (as
    /// this test's own name states) is about *what* is yielded, not merely
    /// how much.
    /// Pitfall: `TypedSlot::get` is declared `pub const fn`, not `pub fn` — a
    /// plain `grep "pub fn"` sweep for its API silently misses it.
    #[test]
    fn drain_up_to_more_than_available_yields_what_there_is() {
        let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(8));
        let (mut producer, mut consumer) = ring.split();

        producer.try_push(1).unwrap();
        producer.try_push(2).unwrap();

        let batch = consumer.drain_up_to(100);
        assert_eq!(batch.len(), 2);

        let values: Vec<u8> = (0..batch.len())
            .filter_map(|offset| batch.get(offset).and_then(TypedSlot::get).copied())
            .collect();
        assert_eq!(values, vec![1, 2]);
    }

    #[test]
    fn get_and_iter_agree_at_every_offset() {
        let mut ring: Ring<TypedSlot<u32>> = Ring::new(cap(8));
        let (mut producer, mut consumer) = ring.split();

        for value in 0..5 {
            producer.try_push(value * 11).unwrap();
        }

        let batch = consumer.drain();

        let by_get: Vec<u32> = (0..batch.len())
            .filter_map(|offset| batch.get(offset).and_then(TypedSlot::get).copied())
            .collect();
        let by_iter: Vec<u32> = batch.iter().filter_map(TypedSlot::get).copied().collect();

        assert_eq!(by_get, by_iter);
        assert_eq!(by_get, [0, 11, 22, 33, 44]);
        assert!(batch.get(batch.len()).is_none(), "one past the end");
    }

    #[test]
    fn a_batch_spanning_a_wrap_reads_the_right_slots() {
        // The case a small doc example never reaches: a batch whose sequences fold
        // across the end of the array, so `get( 0 )` and `get( len - 1 )` are at
        // opposite ends of the storage.
        const CAPACITY: usize = 4;

        let mut ring: Ring<TypedSlot<u32>> = Ring::new(cap(CAPACITY));
        let (mut producer, mut consumer) = ring.split();

        // Advance both cursors to sequence 3, leaving the next batch to start at
        // slot 3 and wrap to slot 0.
        for value in 0..3 {
            producer.try_push(value).unwrap();
        }
        drop(consumer.drain());

        for value in 100..104 {
            producer.try_push(value).unwrap();
        }

        let batch = consumer.drain();

        assert_eq!(batch.start(), Seq(3));
        assert_eq!(batch.len(), 4);
        assert_eq!(
            batch.iter().filter_map(TypedSlot::get).copied().collect::<Vec<_>>(),
            [100, 101, 102, 103],
            "the wrap reordered or aliased a slot"
        );
    }

    /// A batch hands ownership out, and the slot it came from is left empty.
    ///
    /// `get` returns `&S`, and `TypedSlot::take` needs `&mut S`, so before
    /// `get_mut` existed there was no path from a drained batch to an owned `T` at
    /// all — only to a borrow of one. `ring_mpsc`'s batch had the counterpart from
    /// the start; this crate's did not, and nothing noticed until `ring_core`'s
    /// uniform surface needed to return `Option< T >` rather than `Option< &T >`.
    ///
    /// The wrap is deliberate: taking through `get_mut` must address the same slot
    /// `get` would, and an off-by-one in either direction is invisible on a batch
    /// that does not fold across the array's end.
    #[test]
    fn a_record_taken_through_get_mut_leaves_its_slot_empty_across_a_wrap() {
        const CAPACITY: usize = 4;

        let mut ring: Ring<TypedSlot<u32>> = Ring::new(cap(CAPACITY));
        let (mut producer, mut consumer) = ring.split();

        // Same setup as the wrap test above: start the batch at slot 3.
        for value in 0..3 {
            producer.try_push(value).unwrap();
        }
        drop(consumer.drain());

        for value in 200..204 {
            producer.try_push(value).unwrap();
        }

        let mut batch = consumer.drain();
        assert_eq!(batch.start(), Seq(3), "the batch must fold across the array's end");

        let taken: Vec<u32> = (0..batch.len())
            .filter_map(|offset| batch.get_mut(offset).and_then(TypedSlot::take))
            .collect();

        assert_eq!(taken, [200, 201, 202, 203], "get_mut addressed a different slot than get");

        // Taking is destructive: every slot the batch covered is now empty, which is
        // what makes a second take return nothing rather than a stale duplicate.
        assert!(
            (0..batch.len())
                .all(|offset| batch.get_mut(offset).and_then(TypedSlot::take).is_none()),
            "a record survived being taken, so ownership was copied rather than moved"
        );

        assert!(batch.get_mut(batch.len()).is_none(), "get_mut ran past the batch's end");
    }

    #[test]
    fn available_and_is_empty_agree_at_every_point_of_a_lap() {
        // The exhaustive form: `is_empty` is a compare and `available` is a
        // subtraction, so they are two computations that must never disagree.
        const CAPACITY: usize = 4;

        let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(CAPACITY));
        let (mut producer, mut consumer) = ring.split();

        for lap in 0..3_u8 {
            for pushed in 1..=CAPACITY {
                producer.try_push(lap).unwrap();

                assert_eq!(consumer.available(), pushed);
                assert!(!consumer.is_empty());
                assert_eq!(producer.free_capacity(), CAPACITY - pushed);
                assert_eq!(producer.is_full(), pushed == CAPACITY);
            }

            drop(consumer.drain());

            assert_eq!(consumer.available(), 0);
            assert!(consumer.is_empty());
            assert_eq!(producer.free_capacity(), CAPACITY);
        }
    }

    // ── the two ends together ──────────────────────────────────────────────────

    #[test]
    fn a_ring_smaller_than_the_traffic_still_loses_nothing() {
        // Capacity 2 against 20 000 items forces the producer to block on the
        // consumer on almost every push — the case where a mis-ordered publish or a
        // premature commit shows up as an overwrite rather than as a hang.
        const ITEMS: u32 = 20_000;

        let mut ring: Ring<TypedSlot<u32>> = Ring::new(cap(2));
        let (mut producer, mut consumer) = ring.split();

        let received = std::thread::scope(|scope| {
            let producing = scope.spawn(move || {
                for value in 0..ITEMS {
                    while producer.try_push(value).is_err() {
                        std::thread::yield_now();
                    }
                }
            });

            let mut received: Vec<u32> = Vec::with_capacity(ITEMS as usize);

            // Same producer-finished exit as the reached-test, and for the same reason:
            // a count-based loop turns "lost a record" into a hang.
            while (received.len() as u32) < ITEMS {
                let producer_done = producing.is_finished();
                let batch = consumer.drain();

                if batch.is_empty() {
                    if producer_done {
                        break;
                    }

                    std::thread::yield_now();
                    continue;
                }

                received.extend(batch.iter().filter_map(TypedSlot::get).copied());
            }

            producing.join().expect("the producer never panics");

            received
        });

        assert_eq!(received.len() as u32, ITEMS);
        assert!(received.iter().copied().eq(0..ITEMS), "a record was lost, doubled, or reordered");
    }

    #[test]
    fn a_departed_producer_leaves_the_published_tail_drainable() {
        // `docs/lifecycle/002_producer_consumer_pairing.md`'s cleanup requirement 3:
        // one end going away does not invalidate the other, and neither case panics.
        //
        // "Going away" is modelled as the end being moved onto a thread that then
        // finishes — the actual Q3 → Q4 transition — rather than as a `drop` call.
        // Neither end has a `Drop` impl, deliberately: requirement 3 rules that
        // detecting counterpart death belongs to feature 184 at `ring_shutdown`, not
        // to a destructor, so a `drop( producer )` here would be a no-op dressed up
        // as an event.
        let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(4));
        let (producer, mut consumer) = ring.split();

        std::thread::scope(|scope| {
            scope.spawn(move || {
                let mut producer = producer;
                producer.try_push(1).unwrap();
                producer.try_push(2).unwrap();
            });
        });

        assert_eq!(consumer.available(), 2);

        let batch = consumer.drain();
        assert_eq!(batch.len(), 2);

        // Fix(weak_len_assert_sweep_1633):
        // Root Cause: the length-2 assertion above never checked which two values
        // the departed producer actually left behind, or their order.
        // Why Not Caught: a ring that aliased or reordered the tail across the
        // producer's thread boundary would still report a batch of length 2.
        // Fix Applied: read both slots via the non-consuming `get(offset)
        // .and_then(TypedSlot::get).copied()` idiom and assert the exact pushed
        // sequence `[1, 2]`, in order.
        // Prevention: prefer content/order assertions over bare length checks,
        // especially across a thread-boundary handoff where reordering is the
        // realistic failure mode.
        // Pitfall: `TypedSlot::get` is declared `pub const fn`, not `pub fn` — a
        // plain `grep "pub fn"` sweep for its API silently misses it.
        let values: Vec<u8> = (0..batch.len())
            .filter_map(|offset| batch.get(offset).and_then(TypedSlot::get).copied())
            .collect();
        assert_eq!(values, vec![1, 2]);
        drop(batch);

        assert!(consumer.is_empty());
    }

    #[test]
    fn a_departed_consumer_leaves_the_producer_reporting_full_forever() {
        // The other half of the same requirement — and it is `Full`, not `Closed`. A
        // ring that guessed at counterpart death here would be reporting a condition
        // it cannot actually observe: a consumer that has stopped for a millisecond
        // and one that has stopped for good look identical from this end.
        let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(2));
        let (mut producer, consumer) = ring.split();

        std::thread::scope(|scope| {
            scope.spawn(move || {
                let _ = consumer.available();
            });
        });

        producer.try_push(1).unwrap();
        producer.try_push(2).unwrap();

        assert_eq!(producer.try_push(3), Err(3));
        assert_eq!(producer.claim().err(), Some(RingError::Full));
    }

    #[test]
    fn a_second_pair_may_be_split_once_the_first_is_gone() {
        // The complement of the `compile_fail` cases: `&mut self` forbids two live
        // pairs, and permits a sequential one. A ring that could only ever be split
        // once would be unusable from `ring_factory`'s recycling path.
        let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(4));

        {
            let (mut producer, mut consumer) = ring.split();
            producer.try_push(1).unwrap();

            // Fix(weak_len_assert_sweep_1633):
            // Root Cause: the length-1 assertion never checked which value the
            // drain actually returned.
            // Why Not Caught: a ring that aliased the wrong slot could still
            // report a batch of length 1.
            // Fix Applied: read the one slot via the non-consuming `get(offset)
            // .and_then(TypedSlot::get).copied()` idiom and assert it is exactly
            // the pushed value `1`.
            // Prevention: prefer content assertions over bare length checks even
            // for single-element batches — length alone never proves identity.
            // Pitfall: `TypedSlot::get` is declared `pub const fn`, not `pub fn`
            // — a plain `grep "pub fn"` sweep for its API silently misses it.
            let batch = consumer.drain();
            assert_eq!(batch.len(), 1);
            assert_eq!(batch.get(0).and_then(TypedSlot::get).copied(), Some(1));
        }

        let (producer, consumer) = ring.split();

        assert_eq!(producer.position(), Seq(1), "the ring kept its state");
        assert_eq!(consumer.position(), Seq(1));
    }

    // ── what the storage owes its records ──────────────────────────────────────

    /// A payload that records its own destruction.
    ///
    /// The counter is per-instantiation rather than global so two tests using this
    /// type cannot interfere; each test that wants an isolated count gives it its
    /// own `const` generic tag.
    #[derive(Default)]
    struct Tracked<const TAG: usize>;

    static DROPPED: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];

    impl<const TAG: usize> Drop for Tracked<TAG> {
        fn drop(&mut self) {
            DROPPED[TAG].fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn every_record_written_is_dropped_exactly_once() {
        // `lifecycle/001`'s Cleanup 1 and 2. Neither a leak nor a double free shows
        // up in any other test in this file: a ring that forgot its records would
        // pass all twenty-five, and so would one that dropped them twice on a
        // platform tolerant enough not to abort. The count is the only witness.
        //
        // Two laps, not one. A single lap would not exercise the overwrite, which is
        // where the interesting failure is — `TypedSlot::set` replacing an occupied
        // slot must drop what it replaces, and a ring that leaks there leaks once per
        // wrap for as long as it runs.
        const TAG: usize = 0;
        assert_eq!(DROPPED[TAG].load(Ordering::Relaxed), 0, "a fresh counter");

        {
            let mut ring: Ring<TypedSlot<Tracked<TAG>>> = Ring::new(cap(4));
            let (mut producer, mut consumer) = ring.split();

            for _lap in 0..2 {
                for _slot in 0..4 {
                    producer.push_with(|slot| slot.set(Tracked::<TAG>)).unwrap();
                }
                assert_eq!(consumer.drain().len(), 4);
            }

            // Four of the eight are gone already — the second lap's `set` calls
            // replaced the first lap's records, and replacement is a drop.
            assert_eq!(
                DROPPED[TAG].load(Ordering::Relaxed),
                4,
                "the second lap should have dropped the first lap's four records as it overwrote them",
            );
        }

        // The ring is gone, and with it the four records still sitting in its slots.
        assert_eq!(
            DROPPED[TAG].load(Ordering::Relaxed),
            8,
            "eight records were written, so eight destructors should have run — no more and no fewer",
        );
    }

    #[test]
    fn the_ends_going_out_of_scope_in_either_order_releases_the_storage_once() {
        // `lifecycle/001`'s Cleanup 2 and `lifecycle/002`'s Cleanup 4: end drop order
        // is unspecified, so both orders must be correct rather than one being
        // correct and the other merely untried.
        //
        // An earlier form of this test called `drop( consumer ); drop( producer );`
        // and clippy's `drop_non_drop` rejected it — correctly, and with a point
        // worth keeping. **Neither end has a destructor.** Both are a shared
        // reference plus a zero-sized marker, so "dropping" one is not an event at
        // all; the earlier form asserted an ordering between two things that do not
        // happen. The order is established here by binding order instead — locals
        // drop in reverse declaration order — which is the only mechanism that
        // actually orders them, and the count is what shows the storage is released
        // once regardless.
        const TAG: usize = 1;
        assert_eq!(DROPPED[TAG].load(Ordering::Relaxed), 0, "a fresh counter");

        {
            let mut ring: Ring<TypedSlot<Tracked<TAG>>> = Ring::new(cap(2));
            let pair = ring.split();
            let mut producer = pair.0;
            let _consumer = pair.1; // declared last, so it goes first
            producer.push_with(|slot| slot.set(Tracked::<TAG>)).unwrap();
        }
        assert_eq!(DROPPED[TAG].load(Ordering::Relaxed), 1, "consumer end first");

        {
            let mut ring: Ring<TypedSlot<Tracked<TAG>>> = Ring::new(cap(2));
            let pair = ring.split();
            let _consumer = pair.1;
            let mut producer = pair.0; // and now the producer goes first
            producer.push_with(|slot| slot.set(Tracked::<TAG>)).unwrap();
        }
        assert_eq!(DROPPED[TAG].load(Ordering::Relaxed), 2, "and producer end first, same result");
    }

    // ── the types themselves ───────────────────────────────────────────────────

    #[test]
    fn the_two_orderings_are_the_ones_the_design_names() {
        // `OWN` and `HANDOFF` are public because the asymmetry is the crate's thesis
        // (→ the module docs' "Orderings, and why they are not uniform"), and a
        // constant nothing asserts is a comment.
        assert_eq!(ring_spsc::OWN, Ordering::Relaxed, "an end reads its own cursor");
        assert_eq!(ring_spsc::HANDOFF, Ordering::Release, "and publishes with a release store");
        assert_eq!(ring_cursor::GATING, Ordering::Acquire, "and reads the peer's with an acquire");
    }

    #[test]
    fn every_public_type_is_debuggable() {
        // Not decoration: a test that fails in CI prints these, and a type without
        // `Debug` forces the next reader to add one before they can diagnose
        // anything.
        let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(4));
        assert!(!format!("{ring:?}").is_empty());

        let (mut producer, mut consumer) = ring.split();
        assert!(!format!("{producer:?}").is_empty());
        assert!(!format!("{consumer:?}").is_empty());

        let reservation: Reservation<'_, TypedSlot<u8>> = producer.claim().unwrap();
        assert!(!format!("{reservation:?}").is_empty());
        drop(reservation);

        let batch: Batch<'_, TypedSlot<u8>> = consumer.drain();
        assert!(!format!("{batch:?}").is_empty());
    }
}

// ── every interleaving of one push and one drain ───────────────────────────

/// The exhaustive half of this file's verification.
///
/// `loom` swaps `ring_atomic`'s `AtomicSeq` for an instrumented one (see that
/// crate's module documentation on the seam) and re-runs the closure once per
/// distinct interleaving, so an assertion inside it is an assertion about all
/// of them rather than about the one the scheduler happened to pick.
///
/// This is what `docs/state_machine/001_slot_state_without_holes.md`'s
/// invariant 3, `docs/type/001_producer_cursor.md`'s V6, and
/// `docs/non_functional_requirement/001_correctness_floor_for_the_family.md`'s
/// A3 all name, and it is the only form of test that can catch a missing
/// `Release` on the publish or a missing `Acquire` on the peer read.
///
/// # The payload cannot be the slot, and finding that out was the point
///
/// The obvious model — push a byte into a `TypedSlot`, drain it, assert the
/// byte arrived — **is vacuous, and it was written and measured before this
/// one replaced it.** With `HANDOFF` mutated from `Release` to `Relaxed`, that
/// version passed. Loom instruments its own atomics and nothing else; a slot
/// payload lives in plain memory behind the ring's `UnsafeCell`, so loom does
/// not model the write at all and cannot report it as unobserved. A model that
/// cannot fail is not a weaker check than a real one — it is not a check.
///
/// So the payload here is a separate loom `AtomicUsize`, stored *before* the
/// publish and loaded *after* the drain, exactly as
/// `ring_publish/tests/handshake_test.rs` does. The ring still supplies the
/// happens-before edge under test — the edge is this crate's own cursor pair —
/// but the thing whose visibility is asserted is something loom can see.
#[cfg(loom)]
mod exhaustive {
    use loom::sync::Arc;
    use loom::sync::atomic::{AtomicUsize, Ordering};
    use ring_slot::TypedSlot;
    use ring_spsc::Ring;
    use ring_types::Capacity;

    /// What the producer writes. Any value the cell cannot hold by accident;
    /// zero would be indistinguishable from "never written".
    const WRITTEN: usize = 0xABC;

    /// A ring with a `'static` lifetime, so its two ends can be moved onto
    /// `loom::thread::spawn`'s `'static` closures.
    ///
    /// `split` takes `&mut self` and loom has no scoped threads, so the borrow
    /// has to outlive both threads. Leaking one small ring per execution is the
    /// least contrived way to get that: the ring is two `TypedSlot< u8 >` and a
    /// `CursorPair`, and loom's own per-execution bookkeeping dwarfs it.
    fn leaked_ring(capacity: usize) -> &'static mut Ring<TypedSlot<u8>> {
        let capacity = Capacity::new(capacity).expect("a power of two");
        Box::leak(Box::new(Ring::new(capacity)))
    }

    #[test]
    fn a_published_record_is_never_observed_before_the_write_that_preceded_it() {
        loom::model(|| {
            let (mut producer, mut consumer) = leaked_ring(2).split();
            let payload = Arc::new(AtomicUsize::new(0));

            let producing = loom::thread::spawn({
                let payload = Arc::clone(&payload);
                move || {
                    // Between this store and the cursor store inside `push_with` is the
                    // window the model explores. A publish that does not release lets
                    // the drain below reach its assertion with `payload` still zero.
                    payload.store(WRITTEN, Ordering::Relaxed);
                    producer.push_with(|slot| slot.set(1)).expect("an empty ring admits one");
                }
            });

            let draining = loom::thread::spawn({
                let payload = Arc::clone(&payload);
                move || {
                    // One look, not a loop. Every point at which the look could land is
                    // a separate execution loom already runs, so spinning here would add
                    // unbounded executions without adding one new observation.
                    let batch = consumer.drain();
                    assert!(
                        batch.len() <= 1,
                        "drained {} past the only push ever made",
                        batch.len()
                    );

                    if batch.is_empty() {
                        return;
                    }

                    // The drain was offered this record, so the producer's earlier store
                    // must already be visible. This is the assertion a `Relaxed` publish
                    // fails, and the reason the model exists.
                    assert_eq!(
                        payload.load(Ordering::Relaxed),
                        WRITTEN,
                        "a drained record did not carry the write that preceded its publish",
                    );
                }
            });

            producing.join().expect("the producer thread");
            draining.join().expect("the drain thread");
        });
    }

    #[test]
    fn the_consumer_never_sees_further_than_the_producer_published() {
        loom::model(|| {
            let (mut producer, mut consumer) = leaked_ring(2).split();

            let producing = loom::thread::spawn(move || {
                producer.push_with(|slot| slot.set(1)).expect("the first fits");
                producer.push_with(|slot| slot.set(2)).expect("and so does the second");
            });

            let draining = loom::thread::spawn(move || {
                // Two drains, so the consumer's own cursor is read back after it has
                // advanced — a commit that stored the wrong sequence would show up as
                // a total past two rather than as a single oversized batch.
                let first = consumer.drain().len();
                let second = consumer.drain().len();

                assert!(
                    first + second <= 2,
                    "drained {} records past the two ever published",
                    first + second,
                );
            });

            producing.join().expect("the producer thread");
            draining.join().expect("the drain thread");
        });
    }

    /// Regression test for the `free_capacity` underflow, and the reachability
    /// question `ring_debug/docs/invariant/002` DB35 left unresolved.
    ///
    /// # Root Cause
    ///
    /// `Producer::claim` (`src/lib.rs`) performs two independent, unfenced
    /// `Relaxed` loads of the producer cursor — one inside `is_full`'s
    /// `occupancy()` call, and a separate second read afterward to compute
    /// `seq`. Nothing between them forces the second read to see no more than
    /// the first validated against. Given the one precondition violation
    /// `docs/invariant/001` already documents as unenforceable ("Silent"): two
    /// racing bit-copies of one `Producer`, racer A's `is_full` sees the
    /// producer cursor at 0 (not full), racer B's entire claim-and-publish then
    /// lands (cursor now 1), and only *then* does A's own re-read execute,
    /// observing B's already-published 1 and computing its own `seq` from it.
    /// A's publish then drives the cursor to 2 against a capacity of 1 — D2,
    /// reached through nothing but this crate's own public API.
    ///
    /// # Why Not Caught
    ///
    /// `ring_debug/docs/invariant/002` DB35 reasoned D2 is "unreachable from
    /// any live `ring_core::Ring` a test can build" — a claim scoped to
    /// `ring_core`'s composition, never actually tested against `ring_spsc`'s
    /// own direct API. An `std::thread` stress test tried that direct question
    /// first — 2 to 128 racing bit-copies of one `Producer`, hammering
    /// `try_push` for up to ~11.6 million combined attempts against a starved
    /// consumer — and never reproduced D2. That was evidence but not proof: an
    /// absent OS-scheduling interleaving in one run is not the same as an
    /// impossible one, which is exactly the gap probabilistic racing leaves
    /// open. `loom`'s exhaustive search closes that gap and proves the
    /// stronger claim (unreachability) false — it finds the interleaving above
    /// on the very first run, deterministically, on every run since.
    ///
    /// # Fix Applied
    ///
    /// `Producer::free_capacity` changed from the unguarded
    /// `self.ring.capacity().get() - self.occupancy() as usize` to
    /// `self.ring.capacity().get().saturating_sub( self.occupancy() as usize )`
    /// — see the `Fix(free_capacity_underflow_on_a_precondition_violation)`
    /// comment on that method, a few dozen lines above in this same file.
    ///
    /// # Prevention
    ///
    /// This crate cannot prevent the precondition violation itself —
    /// enforcement is structural, above this crate, per `docs/invariant/001`.
    /// What this test asserts is what the crate *can* still guarantee once D2
    /// is reached anyway: that `free_capacity` degrades to a safe, sane answer
    /// rather than panicking (`-D warnings` dev builds) or wrapping to a
    /// near-`usize::MAX` value a caller could mistake for real headroom
    /// (release builds, since the workspace sets no `overflow-checks`
    /// anywhere). Running under `loom` means every future change is re-checked
    /// against every interleaving of this exact shape, not merely sampled the
    /// way the discarded `std::thread` stress test was.
    ///
    /// # Pitfall
    ///
    /// A probabilistic stress test that fails to reproduce a race is evidence
    /// bounded by its own sample size, never proof of impossibility — treating
    /// "millions of attempts found nothing" as "unreachable" would have left
    /// this exact bug undiscovered, exactly as `ring_debug`'s own DB35
    /// reasoning left it. Where a crate already carries `loom` infrastructure
    /// for its ordering claims, a reachability question about a *documented*
    /// precondition violation belongs there too, not only in an OS-thread
    /// stress test that can only ever report what it happened to observe.
    #[test]
    #[allow(unsafe_code)]
    fn free_capacity_degrades_safely_even_when_a_precondition_violation_reaches_d2() {
        loom::model(|| {
            let (producer, _consumer) = leaked_ring(1).split();

            // SAFETY: a deliberate, contained violation of `docs/invariant/001`'s
            // single-producer precondition, made only to ask `loom` whether the
            // violation's documented consequence (D2 lapping) is reachable through
            // this crate's own API, and to check `free_capacity`'s behaviour if so —
            // not a claim that this bit-copy is sound for any other purpose. Both
            // copies share one `&'static Ring`, which outlives both racer threads.
            let p1 = producer;
            let p2 = unsafe { std::ptr::read(&p1) };

            let first = loom::thread::spawn(move || {
                let mut p1 = p1;
                let _ = p1.push_with(|slot| slot.set(1));
                p1
            });
            let second = loom::thread::spawn(move || {
                let mut p2 = p2;
                let _ = p2.push_with(|slot| slot.set(2));
            });

            let p1_after = first.join().expect("the first racer");
            second.join().expect("the second racer");

            // Whichever position the racers land on — 1 (the safe outcome) or past
            // it (D2, which loom does reach for this interleaving) — `free_capacity`
            // must answer with a small, sane number a caller could act on safely,
            // never panic and never wrap past the ring's true capacity.
            let reported = p1_after.free_capacity();
            assert!(
                reported <= 1,
                "free_capacity() reported {reported} against a capacity of 1 — it must \
         saturate to a value no caller could mistake for real headroom, never \
         wrap past the ring's true capacity.",
            );
        });
    }
}
