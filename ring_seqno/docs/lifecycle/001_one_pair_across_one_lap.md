# Lifecycle: One Pair Across One Lap

### Scope

- **Purpose**: Walk a producer/consumer pair through a complete cycle of a capacity-4 ring and tabulate what every reading in the crate says at every position.
- **Responsibility**: Give the full table, name the phase transitions and the reading that detects each, and establish that the cycle is a cycle in the readings but not in the positions.
- **In Scope**: All five functions across one lap and back.
- **Out of Scope**: How stale a reading is by the time a caller acts on it — see [`002`](002_the_validity_window_of_an_answer.md).

### The Setup

Capacity 4, consumer parked at `Seq( 0 )`, producer walking forward. Every value
below is reproducible:

```rust
let c = Capacity::new( 4 ).unwrap();
for p in 0..=5u64
{
  let ( producer, consumer ) = ( Seq( p ), Seq( 0 ) );
  println!( "{p} {} {} {} {}",
    may_claim( producer, consumer, c ),
    free_slots( producer, consumer, c ),
    pending( producer, consumer ),
    laps_between( consumer, producer, c ) );
}
```

### The Full Table

| Producer | In flight | `may_claim` | `free_slots` | `pending` | `laps_between` | Phase |
|:--------:|:---------:|:-----------:|:------------:|:---------:|:--------------:|-------|
| `Seq(0)` | 0 | ✅ | 4 | 0 | 0 | **Empty** |
| `Seq(1)` | 1 | ✅ | 3 | 1 | 0 | Filling |
| `Seq(2)` | 2 | ✅ | 2 | 2 | 0 | Filling |
| `Seq(3)` | 3 | ✅ | 1 | 3 | 0 | Filling — last slot |
| `Seq(4)` | 4 | ❌ | 0 | 4 | **1** | **Full** |
| `Seq(5)` | 5 | ❌ | 0 | 5 | 1 | Full — over-claimed |

Then the consumer advances one, producer still at 4:

| Consumer | In flight | `may_claim` | `free_slots` | `pending` | `laps_between` | Phase |
|:--------:|:---------:|:-----------:|:------------:|:---------:|:--------------:|-------|
| `Seq(1)` | 3 | ✅ | 1 | 3 | 0 | **Draining** — one slot released |
| `Seq(4)` | 0 | ✅ | 4 | 0 | 0 | **Empty again** |

The last row is identical to the first in every column, at different positions.
That is the crate's whole reason for existing, and § *The Cycle Is Not a Cycle*
below says why it matters.

Rows 1, 4, 5 and 6 of the first table are asserted directly at
`tests/seq_test.rs:57-67` and `:92-100`; the whole grid of the first table's
`may_claim`/`free_slots` pair is swept across two laps at `:71-88`.

### The Four Transitions

| Transition | Trigger | Which reading detects it | Which reading does not |
|------------|---------|--------------------------|------------------------|
| Empty → Filling | producer advances from parity | `pending` 0→1 | `may_claim` stays ✅ — it cannot see the difference between empty and half full |
| Filling → Full | in-flight reaches capacity | `may_claim` ✅→❌, `free_slots` 1→0, `laps_between` 0→1 | `pending` just keeps counting up; nothing about it marks the boundary |
| Full → Over-claimed | producer advances past a full lap | **none** | every reading. `free_slots` saturates at 0, `may_claim` was already ❌, `laps_between` stays 1 until sequence 8 |
| Draining → Empty | consumer catches the producer | `pending` →0, `free_slots` →capacity | `may_claim` was ✅ throughout draining |

**Row 3 is the important one.** Between `Seq(4)` and `Seq(5)` the ring moves from
"exactly full" to "a producer has claimed a slot it should not have", and no
reading in this crate changes. `free_slots` saturates and `laps_between` is a
floor division, so both flatten. The state is not merely undetected — it is
*indistinguishable* from the legal one.

That is by design, not oversight: `may_claim` refuses at `Seq(4)`, so a producer
obeying the gate never reaches `Seq(5)`. The readings do not have to detect a
state the gate prevents. What follows is that they cannot help diagnose it either,
which is why `ring_debug` exists and why it reads cursors directly rather than
through these functions:

> Reads both cursors once, `Acquire`, and compares them directly rather than
> through `ring_seqno` — whose saturating arithmetic is what makes the first of the
> two invisible.

See [`decisions/002`](../decisions/002_saturating_rather_than_signed.md) §
*The Double Saturation Has a Consequence Worth Naming*.

### The Cycle Is Not a Cycle

Row 6 of table one and row 2 of table two carry identical readings — `may_claim`
✅, `free_slots` 4, `pending` 0, `laps_between` 0 — at positions `(0, 0)` and
`(4, 4)`. The ring returned to its starting state; the sequences did not.

The mechanism is that four of the five functions read **only the difference**:

| Function | Depends on | Property |
|----------|------------|----------|
| `may_claim( p, c, cap )` | `p - c` | translation-invariant |
| `free_slots( p, c, cap )` | `p - c` | translation-invariant |
| `pending( p, c )` | `p - c` | translation-invariant |
| `laps_between( e, l, cap )` | `l - e` | translation-invariant |
| `slowest( &[ .. ] )` | the values themselves | **equivariant**: `slowest( xs + k ) == slowest( xs ) + k` |

Add any constant `k` to both positions and the first four answers are unchanged.
`slowest` is the exception, and necessarily so — it returns a position, not a
span, and a position that moved by `k` must come back moved by `k`.

This is asserted rather than left implicit:

```rust
// tests/seq_test.rs:38-44
fn laps_are_relative_not_absolute()
{
  assert_eq!( laps_between( Seq( 1_000_000 ), Seq( 1_000_008 ), c ), 1 );
  assert_eq!( laps_between( Seq( 1_000_001 ), Seq( 1_000_008 ), c ), 0 );
}
```

with the doc comment giving the reason directly: *"the case that matters, since a
long-lived ring's cursors are never near zero."*

**And it is exactly what folding would destroy.** `tests/seq_test.rs:134-148`
makes the point at 99 laps:

```rust
// Folded, both are slot 0 — identical, and the gate would see "caught up".
assert_eq!( producer.0 % 8, consumer.0 % 8 );
// Unfolded, the producer is 99 laps ahead and must be refused.
assert_eq!( laps_between( consumer, producer, c ), 99 );
assert!( !may_claim( producer, consumer, c ) );
```

Translation-invariance in the *difference* is what the crate keeps; periodicity in
the *position* is what it refuses. Those sound similar and are opposite: the first
means the readings do not care where the ring is, the second would mean they
cannot tell how far apart it is.

### The Lifecycle This Crate Never Sees

Three phases of a real ring's life produce no reading here at all:

| Phase | Owner | Why `ring_seqno` is silent |
|-------|-------|--------------------------|
| Construction | `ring_config`, `ring_factory` | `Capacity::new` validates once, in `ring_types`; this crate assumes the result |
| Shutdown | `ring_shutdown` | Draining to a quiescent state is a protocol over the same readings, not a new reading |
| Sequence exhaustion | nobody | `Seq` is `u64`; at 1M msg/s the space lasts ~584,000 years, and `Seq::next`'s documented behaviour at the boundary is wrong (**Finding SQ44**, [`pitfall/001`](../pitfall/001_implementing_may_claim_with_laps_between.md) § *The `next` Doc*) |

The third is the only one that is a gap rather than a division of labour, and it
is unreachable.

### SQ31 — Four Invariant, One Equivariant

Shift both positions by the same amount and four answers do not move:

```
laps_between( a+k, b+k, c ) == laps_between( a, b, c )   invariant under translation
may_claim   ( a+k, b+k, c ) == may_claim   ( a, b, c )   invariant
free_slots  ( a+k, b+k, c ) == free_slots  ( a, b, c )   invariant
pending     ( a+k, b+k    ) == pending     ( a, b    )   invariant
slowest     ( [x+k …]     ) == slowest( [x …] ) + k      EQUIVARIANT
```

**Finding.** Every reading is a function of the difference alone; `slowest` alone is equivariant rather than invariant.

---

### SQ32 — One Example Out of Step With the Others

Four doctests, three capacities held constant and one not:

```
laps_between  Capacity::new( 8 )   3 assertions
may_claim     Capacity::new( 4 )   3 assertions
free_slots    Capacity::new( 4 )   3 assertions
pending       (no capacity)         2 assertions
slowest       (no capacity)         2 assertions
```

**Finding.** Three of the four pair functions demonstrate themselves at capacity 4 and `laps_between` at 8, so the one function whose doc claims it "is the reading that decides" is also the one whose example cannot be read line-for-line against its neighbours.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_four_readings_of_one_subtraction.md](../algorithm/001_four_readings_of_one_subtraction.md) | Why the four share a difference |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_saturating_rather_than_signed.md](../decisions/002_saturating_rather_than_signed.md) | The transition no reading detects, and why saturating is still right |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_sequence_is_never_folded_here.md](../invariant/001_the_sequence_is_never_folded_here.md) | The invariant that makes § *The Cycle Is Not a Cycle* hold |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_capacity_readings.md](../item/001_the_three_capacity_readings.md) | The `Seq(3)`→`Seq(4)` transition as a per-function contract |

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_the_validity_window_of_an_answer.md](002_the_validity_window_of_an_answer.md) | What happens to every cell of these tables once time is allowed to pass |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:50-136` | The five readings tabulated |
| `ring_debug/src/lib.rs:250-252` | Why the diagnostic tier bypasses these functions |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:57-67` | The Filling → Full transition, both sides |
| `tests/seq_test.rs:71-88` | The whole first table's `may_claim`/`free_slots` columns, twice over |
| `tests/seq_test.rs:38-44` | Translation-invariance |
| `tests/seq_test.rs:92-101` | `free_slots` across the full range, including saturation |
| `tests/seq_test.rs:134-148` | Folding would collapse the cycle; unfolded, it does not |
