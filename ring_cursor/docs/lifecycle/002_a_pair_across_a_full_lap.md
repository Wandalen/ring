# Lifecycle: A Pair Across a Full Lap

### Scope

- **Purpose**: Walk a `CursorPair` through one complete lap, show what each reading says at every step, and identify the state the readings cannot distinguish from the start.
- **Responsibility**: Give the arithmetic model, tabulate a lap, locate the off-by-one, and derive the consequence of the family's saturating arithmetic.
- **In Scope**: The pair's three readings over a lap; `Seq`'s unbounded monotone model; the two impossible states.
- **Out of Scope**: A single cursor's stages, which is [`lifecycle/001`](001_a_cursor_from_new_to_shared.md).

### The Arithmetic Model

A `Seq` is a `u64` that **does not wrap within any reachable workload** — the
arithmetic itself wraps, and the reason that is survivable is a reachability
argument rather than a property of the operator:

```rust
pub const fn next( self ) -> Self { Self( self.0 + 1 ) }
```

> Panics on overflow in a debug build and wraps to zero in a release build — the
> standard `u64` addition behaviour. A wrapped `Seq` would silently invert every
> gate comparison in the family, which is why the non-wrapping argument has to
> hold: at 10⁹ publications per second a `u64` runs for roughly 584 years, well
> past any reachable workload.

That paragraph is a correction. It previously read "saturates in a release build
… deliberately not wrapping", which this document quoted and built its "never
wraps" phrasing on; release `+` wraps, and the corrected text says so. The
conclusion below is unaffected, because it turns on the wrap point being
unreachable rather than on the operator refusing to reach it.

So a "lap" is not a wrap. Sequences climb forever; what wraps is the **slot
index**, which `ring_index` derives modulo capacity. `SlotIndex`'s own
documentation states the split:

> Two sequences a full lap apart produce the identical `SlotIndex`, which is the
> whole reason [`Seq`] exists as a separate type.

`CursorPair` never sees a `SlotIndex`. It works entirely in the unbounded space,
which is why none of its three readings has a wrap-around case.

### One Lap, Tabulated

A 4-slot ring, producer advancing then consumer following:

| Step | producer | consumer | `distance` | `free_slots` | `pending` | `may_claim` |
|------|---------:|---------:|-----------:|-------------:|----------:|:-----------:|
| fresh | 0 | 0 | 0 | 4 | 0 | ✅ |
| filling | 3 | 0 | 3 | 1 | 3 | ✅ |
| **full** | **4** | **0** | **4** | **0** | **4** | ❌ |
| one released | 4 | 1 | 3 | 1 | 3 | ✅ |
| drained | 4 | 4 | 0 | 4 | 0 | ✅ |
| second lap | 8 | 4 | 4 | 0 | 4 | ❌ |

**The last row is identical to the third in every column**, at twice the absolute
values. That is the lap: the readings are functions of the *difference*, so the
pair returns to a state it has already been in while both cursors keep climbing.

Two rows are tested directly:

```rust
pair.producer().store( Seq( 3 ), Ordering::Release );
assert!( pair.may_claim(), "3 ahead of a 4-slot ring: one slot left" );

pair.producer().store( Seq( 4 ), Ordering::Release );
assert!( !pair.may_claim(), "exactly one lap: no room" );
assert_eq!( pair.free_slots(), 0 );
```

### The Off-By-One Is the Whole Point

At a distance of exactly `capacity` the ring is **full**, not "one more allowed":

```rust
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
{
  consumer.distance_to( producer ) < capacity.get() as u64
}
```

`<`, not `<=`. The test's comment names why:

> at a distance of exactly `capacity` the next claim lands on the slot the
> consumer is currently on.

Both boundary sides are asserted — `3` passes, `4` fails — which is what makes
this a boundary test rather than a value test. Getting it wrong in either
direction produces a ring that either wastes a slot forever or corrupts one
occasionally, and only the second is a bug anyone would notice.

### The State the Readings Cannot See

`distance_to` saturates:

```rust
pub const fn distance_to( self, later : Self ) -> u64 { later.0.saturating_sub( self.0 ) }
```

So a consumer **ahead** of its producer — a state no correct ring reaches —
yields `0`, and every reading downstream reports health:

| Reading | Value when consumer is ahead | Same as |
|---------|------------------------------|---------|
| `pending` | `0` | a fully drained ring |
| `free_slots` | `capacity - 0` = `capacity` | a fresh ring |
| `may_claim` | `0 < capacity` → `true` | a fresh ring |

**The corrupt state is indistinguishable from the initial state, in all three
readings simultaneously.** Not degraded, not suspicious — optimal.

This is not an oversight, and the family says so where it matters.
`ring_debug::Violation::ConsumerAheadOfProducer` carries the diagnosis:

> **The dangerous one.** The family's arithmetic saturates here, so this state
> reads as an empty, healthy ring and `may_claim` returns `true`.

and `ring_debug::check` is documented as reading both cursors and comparing them
**directly** —

> rather than through `ring_seqno` — whose saturating arithmetic is what makes the
> first of the two invisible.

So the detector had to bypass the arithmetic that hides it. That is the design:
the hot path saturates because a branch per gating read is not worth paying for a
state that cannot occur, and a separate opt-in crate re-derives the comparison
for anyone who wants certainty.

### The Other Impossible State Leaves Evidence

`ProducerLappedConsumer` — the producer more than a full lap ahead — is the
mirror case, and it is the easier one:

| | `ConsumerAheadOfProducer` | `ProducerLappedConsumer` |
|---|---|---|
| `pending` | `0` | **exceeds capacity** — no valid state does |
| Detectable from the readings alone? | **No** | Yes |
| `ring_debug` reports first when both hold? | ✅ | — |

`ring_debug` prefers to report the first "because D1 is the one that reads as
healthy and is therefore the one a reader has no other way to learn about". The
preference order encodes exactly the asymmetry above.

### Where the Lap Actually Ends

| # | Bound | Value | Reached after |
|---|-------|------:|---------------|
| B1 | `Seq` overflow | `u64::MAX` | ~584 years at 10⁹ publications/second |
| B2 | `laps_between` precision | none — integer division | never |
| B3 | Anything in this crate | — | this crate has no lap counter |

**B3 is worth stating.** `ring_seqno::laps_between` exists and `ring_cursor` never
calls it — the four `ring_seqno::` calls here are `slowest`, `free_slots`,
`pending`, `may_claim`. A pair does not know which lap it is on and has no reason
to.

B1 is the honest limit and it is handled by panicking in debug rather than
wrapping, which is the right trade: a wrapped `Seq` would make `distance_to`
return a plausible number for a nonsensical state, which is the same failure mode
as the saturation above but permanent.

### CU31 — The Boundary Rule Reaches These Tests as a Failure, Not a Compile Error

`exactly_one_lap_ahead_is_full_and_one_less_is_not` asserts what happens when a
producer is exactly `capacity` ahead. The decision it asserts is made in
`ring_seqno`; this crate contributes two loads.

**Finding.** Change the boundary from `>=` to `>` in `ring_seqno` and this crate
compiles unchanged — the signature is identical — and the test fails. That is the
right failure, and it means the test is guarding another crate's semantics
through this crate's forwarding. Nothing here would catch the change before the
test runs.

---

### CU32 — Complementary Readings, Incompatible Types

Across a full lap, `pending` counts what has been produced and not consumed and
`free_slots` counts what remains. They are complementary — together they are the
capacity — and they return `u64` and `usize`.

**Finding.** A caller asserting the complement casts one of them. The widths come
from `ring_seqno`, where the distinction is real: sequence steps are unbounded and
slot counts are bounded by the ring. This crate forwards both and explains
neither, so the cast looks like sloppiness at the call site rather than a type
boundary being crossed.

---

### Regenerate

Every `rust` block above was hand-transcribed, which is how the `Seq::next`
paragraph this document quoted stayed here after `ring_types` replaced it —
nothing in the corpus could see the quote to notice it had gone stale. This
recipe reproduces all four bodies from live source, so the next divergence is a
gate failure rather than a reader's discovery:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '-- Seq::next, with the overflow paragraph the model rests on --'
command grep -m1 -A16 -F '  /// The next sequence after this one.' ring_types/src/id.rs
echo '-- distance_to, the saturation the invisible state hides behind --'
command grep -m1 -A4 -F '  pub const fn distance_to( self, later : Self ) -> u64' ring_types/src/id.rs
echo '-- may_claim, where the < rather than <= lives --'
command grep -m1 -A3 -F 'pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool' ring_seqno/src/lib.rs
echo '-- and the two boundary rows, as the test asserts them --'
command grep -A6 -F 'assert!( pair.may_claim(), "3 ahead of a 4-slot ring: one slot left" );' ring_cursor/tests/cursor_test.rs
```

Live output:

```
-- Seq::next, with the overflow paragraph the model rests on --
  /// The next sequence after this one.
  ///
  /// Panics on overflow in a debug build and wraps to zero in a release
  /// build — the standard `u64` addition behaviour. A wrapped `Seq` would
  /// silently invert every gate comparison in the family, which is why the
  /// non-wrapping argument has to hold: at 10⁹ publications per second a
  /// `u64` runs for roughly 584 years, well past any reachable workload.
  ///
  /// ```
  /// use ring_types::Seq;
  /// assert_eq!( Seq( 41 ).next(), Seq( 42 ) );
  /// ```
  #[ must_use ]
  pub const fn next( self ) -> Self
  {
    Self( self.0 + 1 )
  }
-- distance_to, the saturation the invisible state hides behind --
  pub const fn distance_to( self, later : Self ) -> u64
  {
    later.0.saturating_sub( self.0 )
  }
}
-- may_claim, where the < rather than <= lives --
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
{
  consumer.distance_to( producer ) < capacity.get() as u64
}
-- and the two boundary rows, as the test asserts them --
  assert!( pair.may_claim(), "3 ahead of a 4-slot ring: one slot left" );

  pair.producer().store( Seq( 4 ), Ordering::Release );
  assert!( !pair.may_claim(), "exactly one lap: no room" );
  assert_eq!( pair.free_slots(), 0 );
}
```

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_three_readings_of_two_cursors.md](../algorithm/002_three_readings_of_two_cursors.md) | The three readings computed, and whether their loads are atomic together |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_cursor_pair.md](../data_structure/002_the_cursor_pair.md) | The structure the lap runs in |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_who_reads_a_cursor.md](../integration/002_who_reads_a_cursor.md) | `ring_debug`, the consumer that exists for the invisible state |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_one_cursor_one_line.md](../invariant/001_one_cursor_one_line.md) | The layout invariant, which the lap does not touch |

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_a_cursor_from_new_to_shared.md](001_a_cursor_from_new_to_shared.md) | L2 — `Seq` overflow, as a gap in a single cursor's stages |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_a_reading_that_consults_one_cursor.md](../pitfall/002_a_reading_that_consults_one_cursor.md) | The other way the fresh state hides a defect |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs` — `laps_between`, `may_claim`, `free_slots`, `pending` | The four readings this crate forwards |
| `ring_types/src/id.rs` — `Seq::next`, `distance_to` | The unreachable wrap, and the saturation that hides the invisible state |
| `ring_types/src/id.rs` — `SlotIndex` | Where lapping actually happens |
| `ring_debug/src/lib.rs` — `Violation`, `check` | The two impossible states, and the check that bypasses the saturation |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs` — `exactly_one_lap_ahead_is_full_and_one_less_is_not` | The full row and the row before it — both sides of the boundary |
| `tests/cursor_test.rs` — `a_consumer_moving_on_reopens_the_ring` | The consumer releasing one slot |
| `tests/cursor_test.rs` — the fresh-pair assertions | The fresh row |
| `tests/cursor_test.rs` — the `pending`/`free_slots` divergence test | `pending` ignoring capacity where `free_slots` does not |
| `ring_debug/tests/debug_test.rs` — the two `Violation` cases | The two impossible states, exercised where they are detectable |
