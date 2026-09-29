//! `Publisher`'s own surface, at the boundaries.
//!
//! Separate from `handshake_test.rs`, which is feature 170's reached-test and
//! exercises this crate only as one of four participants. That test answers
//! "does the handshake hold"; this one answers "does each operation do what its
//! contract says", and the two fail for different reasons — a `try_publish`
//! that accepted a start one past the frontier would still pass the handshake
//! every time only one producer ever publishes.
//!
//! ## What the doc examples do not cover
//!
//! Every method here has a doc example, and doc tests do run. But they are not
//! counted as coverage of the crate, and more importantly they are written to
//! *show the shape* of an operation rather than to probe its edges: none of
//! them publishes a zero-length range, tries a start behind the frontier, or
//! asks `is_published` about the exact boundary sequence. Those are the cases
//! where an off-by-one lives.

// Ordinary tests, so they must be out under `--cfg loom` — the same gate
// `handshake_test.rs` puts on its own ordinary body, and the inverse of the
// `#![ cfg( loom ) ]` its `exhaustive` module carries. `--cfg loom` swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole graph,
// and those panic the moment they are touched outside a `loom::model` closure.
// Without this, `RUSTFLAGS="--cfg loom" cargo test -p ring_publish` fails here
// before it can run the model at all.
#![cfg(not(loom))]

use core::sync::atomic::Ordering;

use ring_cursor::SeqCell;
use ring_publish::Publisher;
use ring_types::Seq;

// ── construction ───────────────────────────────────────────────────────────

#[test]
fn a_new_publisher_has_published_nothing() {
  let publisher = Publisher::new();

  assert_eq!(publisher.published(), Seq::ZERO);
  assert_eq!(publisher.cursor().load(Ordering::Acquire), Seq::ZERO);
  assert!(!publisher.is_published(Seq::ZERO), "nothing is readable yet");
}

#[test]
fn new_and_default_agree() {
  // `new` is `Self::default()` today. Asserting the equivalence rather than
  // trusting it means a future `new` that grew a parameter or a non-zero start
  // has to notice this test rather than silently diverging from `Default`.
  assert_eq!(Publisher::new().published(), Publisher::default().published());
}

#[test]
fn the_cursor_handed_out_is_the_one_publication_moves() {
  // The barrier a consumer builds is over *this* cursor, so a `cursor()` that
  // returned a copy would give every consumer a frontier frozen at zero — a
  // ring that compiles, runs, and never delivers anything.
  let publisher = Publisher::new();
  let cursor = publisher.cursor();

  publisher.publish(Seq::ZERO, 3);

  assert_eq!(cursor.load(Ordering::Acquire), Seq(3), "the borrow saw the advance");
  assert!(
    core::ptr::eq(cursor, publisher.cursor()),
    "and it is the same cursor each time"
  );
}

// ── try_publish ────────────────────────────────────────────────────────────

#[test]
fn try_publish_advances_from_the_exact_frontier() {
  let publisher = Publisher::new();

  assert_eq!(publisher.try_publish(Seq::ZERO, 4), Ok(Seq(4)));
  assert_eq!(publisher.try_publish(Seq(4), 1), Ok(Seq(5)));
  assert_eq!(publisher.published(), Seq(5));
}

#[test]
fn try_publish_refuses_a_start_past_the_frontier_and_reports_where_it_is() {
  // The multi-producer case: B finished before A. The error value is not
  // decoration — it is what B retries against, so a version returning
  // `RingError` or `()` would force B to re-read separately and race again.
  let publisher = Publisher::new();

  assert_eq!(publisher.try_publish(Seq(4), 4), Err(Seq::ZERO));
  assert_eq!(publisher.published(), Seq::ZERO, "and nothing moved");

  assert_eq!(publisher.try_publish(Seq::ZERO, 4), Ok(Seq(4)));
  assert_eq!(publisher.try_publish(Seq(4), 4), Ok(Seq(8)), "now it is B's turn");
}

#[test]
fn try_publish_refuses_a_start_behind_the_frontier() {
  // The direction the doc example never shows, and the more dangerous one: a
  // start *behind* the frontier would move the published cursor backwards,
  // un-publishing slots a consumer may already be reading.
  let publisher = Publisher::new();
  publisher.publish(Seq::ZERO, 8);

  assert_eq!(publisher.try_publish(Seq(4), 2), Err(Seq(8)));
  assert_eq!(publisher.try_publish(Seq::ZERO, 8), Err(Seq(8)), "not even re-publishing");
  assert_eq!(publisher.published(), Seq(8), "the frontier never retreats");
}

#[test]
fn a_zero_length_publication_is_accepted_and_moves_nothing() {
  // Accepted rather than refused, because a producer that claimed nothing has
  // nothing to wait for and no reason to be told to retry — and because
  // `claim_up_to` can legitimately grant a shorter range than asked for.
  let publisher = Publisher::new();
  publisher.publish(Seq::ZERO, 5);

  assert_eq!(publisher.try_publish(Seq(5), 0), Ok(Seq(5)));
  assert_eq!(publisher.published(), Seq(5));
}

// ── publish ────────────────────────────────────────────────────────────────

#[test]
fn publish_returns_the_end_of_what_it_published() {
  let publisher = Publisher::new();

  assert_eq!(publisher.publish(Seq::ZERO, 3), Seq(3));
  assert_eq!(publisher.publish(Seq(3), 1), Seq(4));
  assert_eq!(publisher.publish(Seq(4), 6), Seq(10));
}

#[test]
fn publish_waits_for_its_predecessor_rather_than_reordering() {
  // The whole reason `publish` is a loop. B's range is published only after A's
  // — never before, and never with a gap — so a consumer's frontier moves
  // through 0 → 4 → 8 and is at no point 8 while 0..4 is unwritten.
  let publisher = Publisher::new();

  std::thread::scope(|scope| {
    let waiting = scope.spawn(|| {
      // Starts blocked: the frontier is at 0, and this range begins at 4.
      publisher.publish(Seq(4), 4)
    });

    // Nothing this thread can do makes B's publication land early; the only
    // thing that unblocks it is A's own publication below.
    for _ in 0..1_000 {
      assert!(publisher.published() <= Seq(4), "B published before A");
      std::thread::yield_now();
    }

    assert_eq!(publisher.publish(Seq::ZERO, 4), Seq(4), "A goes first");
    assert_eq!(waiting.join().expect("B never panics"), Seq(8));
  });

  assert_eq!(publisher.published(), Seq(8));
}

#[test]
fn several_producers_publishing_out_of_order_end_contiguous() {
  // Four ranges, handed to threads in an order deliberately unrelated to their
  // sequence order. Whatever order they are scheduled in, the frontier is
  // monotonic and lands exactly at the end.
  const WIDTH: u64 = 25;

  let publisher = Publisher::new();
  let starts = [Seq(3 * WIDTH), Seq(WIDTH), Seq(0), Seq(2 * WIDTH)];

  // Reborrowed so the `move` closure — which it needs, to take `start` by
  // value — captures the reference rather than the publisher itself.
  let publisher = &publisher;

  std::thread::scope(|scope| {
    for start in starts {
      scope.spawn(move || publisher.publish(start, WIDTH as usize));
    }
  });

  assert_eq!(publisher.published(), Seq(4 * WIDTH));
}

// ── is_published ───────────────────────────────────────────────────────────

#[test]
fn is_published_is_exclusive_of_the_frontier() {
  // The boundary the doc example gestures at and this pins down: `published()`
  // is one *past* the last readable sequence, so the frontier itself must
  // answer false. Off by one here hands a consumer a slot no producer has
  // finished writing — the exact failure the loom model catches from the other
  // direction.
  let publisher = Publisher::new();
  publisher.publish(Seq::ZERO, 3);

  assert!(publisher.is_published(Seq::ZERO));
  assert!(publisher.is_published(Seq(1)));
  assert!(publisher.is_published(Seq(2)), "the last readable one");
  assert!(!publisher.is_published(Seq(3)), "the frontier itself is not readable");
  assert!(!publisher.is_published(Seq(4)));
}

#[test]
fn nothing_is_published_before_anything_is() {
  let publisher = Publisher::new();

  for seq in 0..8 {
    assert!(!publisher.is_published(Seq(seq)), "{seq} on an empty publisher");
  }
}

#[test]
fn is_published_agrees_with_published_at_every_point_of_a_run() {
  // The exhaustive form: rather than trusting the two methods to stay
  // consistent, check the whole range on both sides of a moving frontier.
  const REACH: u64 = 16;

  let publisher = Publisher::new();

  for frontier in 0..REACH {
    assert_eq!(publisher.published(), Seq(frontier));

    for candidate in 0..REACH + 4 {
      assert_eq!(
        publisher.is_published(Seq(candidate)),
        candidate < frontier,
        "at frontier {frontier}, asking about {candidate}"
      );
    }

    publisher.publish(Seq(frontier), 1);
  }
}
