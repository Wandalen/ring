//! The primary producer's own verification: the cached-cursor fast path,
//! the exclusivity contract with ordinary producers, and the loom model for
//! the guessed exchange.

#[cfg(loom)]
mod common;

#[cfg(not(loom))]
mod threaded {
  use ring_mpsc::Ring;
  use ring_slot::TypedSlot;
  use ring_types::{Capacity, RingError, Seq};

  fn cap(slots: usize) -> Capacity {
    Capacity::new(slots).expect("a power-of-two capacity")
  }

  /// The primary handle mints from the shared claim cursor, coexists with
  /// ordinary `Copy` producers, and its grants are disjoint from theirs —
  /// the compare-exchange arbitrates, exactly as it does between copies.
  #[test]
  fn a_primary_producer_and_a_copy_producer_grant_disjoint_sequences() {
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(4));
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
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(2));
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
    let mut ring: Ring<TypedSlot<u8>> = Ring::new(cap(4));
    let mut ends = ring.ends();
    let (mut producer, mut consumer) = ends.split();
    let mut primary = producer.primary();

    primary.try_push(1).expect("room"); // sequence 0, published

    std::mem::forget(primary.claim().expect("room")); // sequence 1: cursor moved, no stamp

    primary.try_push(2).expect("room"); // sequence 2, published past the hole

    assert_eq!(consumer.drain().len(), 1, "only the record before the hole drains",);
    assert_eq!(consumer.drain().len(), 0, "the hole never fills");
  }
}

#[cfg(loom)]
mod exhaustive {
  use loom::sync::atomic::{AtomicUsize, Ordering};
  use ring_atomic::SeqCell;
  use ring_types::Seq;

  use crate::common::leaked_ends;

  /// The primary's guessed exchange never double-grants against an ordinary
  /// producer: the compare-exchange arbitrates, and every published record is
  /// drained exactly once.
  ///
  /// Red under the mutation that replaces the primary's compare-exchange with
  /// a plain store: both writers grant sequence zero, one publish overwrites
  /// the other's increment, the watermark stops at zero, and the reader
  /// drains one record against two grants.
  ///
  /// What this model cannot check: the `Acquire` on the cached head's
  /// refresh. Loom linearises each step, so a relaxed refresh that returns
  /// the committed value orders identically to an acquired one there, while
  /// on hardware without the edge the overwrite and the take can race — the
  /// same blind spot the payload-proxy pattern narrows only this far.
  #[test]
  fn a_primary_producer_and_a_copy_producer_never_double_grant() {
    loom::model(|| {
      let payloads: &'static [AtomicUsize; 2] = Box::leak(Box::new([AtomicUsize::new(0), AtomicUsize::new(0)]));

      let ends = leaked_ends();
      let ring = ends.ring();
      let (mut producer, mut consumer) = ends.split();
      let mut primary = producer.primary();

      let copy_writer = loom::thread::spawn(move || {
        // Claim first — the sequence decides the slot — then store the
        // payload, then publish. The store sits between the claim and the
        // publish, so the drain's `Acquire` on the cursor sees it.
        let reserved = producer.claim().expect("an empty ring admits one");
        let seq = reserved.sequence();
        payloads[(seq.0 & 1) as usize].store(11, Ordering::Relaxed);
        drop(reserved);
      });

      let primary_writer = loom::thread::spawn(move || {
        let reserved = primary.claim().expect("a two-slot ring admits two");
        let seq = reserved.sequence();
        payloads[(seq.0 & 1) as usize].store(22, Ordering::Relaxed);
        drop(reserved);
      });

      let reader = loom::thread::spawn(move || {
        let first = consumer.drain();
        for offset in 0..first.len() {
          let seq = first.start().advanced_by(offset as u64);
          assert_ne!(
            payloads[(seq.0 & 1) as usize].load(Ordering::Relaxed),
            0,
            "drained record {seq:?} carried no write at all",
          );
        }
        drop(first);

        let second = consumer.drain();
        for offset in 0..second.len() {
          let seq = second.start().advanced_by(offset as u64);
          assert_ne!(
            payloads[(seq.0 & 1) as usize].load(Ordering::Relaxed),
            0,
            "drained record {seq:?} carried no write at all",
          );
        }
        second.len()
      });

      copy_writer.join().expect("no panic");
      primary_writer.join().expect("no panic");
      reader.join().expect("no panic");

      // Both writers claimed a distinct sequence and published it: the
      // stamps carry sequence zero and sequence one. Under a plain store in
      // place of the primary's compare-exchange the two grants collide on
      // sequence zero, sequence one is never published, and its stamp stays
      // `UNSTAMPED`.
      assert_eq!(
        ring.stamps()[0].load(Ordering::Relaxed),
        Seq(0),
        "sequence zero was published by whichever writer won it",
      );
      assert_eq!(
        ring.stamps()[1].load(Ordering::Relaxed),
        Seq(1),
        "sequence one was never published — the two grants collided",
      );
    });
  }
}
