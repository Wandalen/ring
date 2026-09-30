# Type: A Seq, a usize, and Three Casts

### Scope

- **Purpose**: Record the crate's integer discipline — which quantity is a `Seq` and which is a `usize` — the three casts that cross between them, and the direction every one of them goes.
- **Responsibility**: State the rule the signatures follow without documenting, locate the reciprocal cast that is missing from this crate, and show what the crate never converts at all.
- **In Scope**: `Seq`, `usize`, and the boundary between them within `ring_claim`.
- **Out of Scope**: The lifetime — see [`type/002`](002_the_lifetime_on_the_claimer.md). `Seq`'s own overflow behaviour is [`lifecycle/002`](../lifecycle/002_the_claimer_over_a_rings_life.md) § CL34.

### The Rule

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs | grep -E 'pub (const )?fn'
```

Live output:

```
  pub const fn new( start : Seq, len : usize ) -> Self
  pub const fn start( self ) -> Seq
  pub const fn end( self ) -> Seq
  pub const fn len( self ) -> usize
  pub const fn is_empty( self ) -> bool
  pub const fn contains( self, seq : Seq ) -> bool
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  pub const fn overlaps( self, other : Self ) -> bool
  pub fn new( consumers : &'a GatingSet ) -> Self
  pub const fn cursor( &self ) -> &PaddedCursor
  pub const fn consumers( &self ) -> &'a GatingSet
  pub fn claimed( &self ) -> Seq
  pub fn headroom( &self ) -> usize
  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >
  pub fn claim_up_to( &self, max : usize ) -> Result< Claim, RingError >
```

Fifteen signatures, and between them one rule with no exceptions:

| Quantity | Type | Appears as |
|----------|------|------------|
| a **position** in the sequence stream | `Seq` | `new( start : Seq, … )`, `start()`, `end()`, `contains( seq : Seq )`, `sequences() -> impl Iterator< Item = Seq >`, `claimed()` |
| a **count** of sequences | `usize` | `new( …, len : usize )`, `len()`, `headroom()`, `claim( count : usize )`, `claim_up_to( max : usize )` |

Positions are 64-bit and absolute; counts are platform-width and relative. The
rule is never stated anywhere in the crate, and it is followed everywhere —
including in the one place it costs something, `Claim::len() -> usize`, where
returning `u64` would have removed a cast and matched the field it is derived
from.

`ring_consume::Available` made the other choice for the same field
(`len : u64`), which is what makes this a rule rather than an inevitability
([`pattern/002`](../pattern/002_the_half_open_range_as_a_value.md) § CL41).

### CL47 — All Three Casts Widen, and the One That Narrows Now Proves Itself Locally

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E ' as (u64|usize|u32|i64)' ring_claim/src/lib.rs
```

Live output:

```
        self.start.advanced_by(self.len as u64)
            let next = current.advanced_by(count as u64);
            let next = current.advanced_by(granted as u64);
```

Three casts in the crate, and they are the complete list:

| Line | Cast | In |
|-----:|------|----|
| 146 | `self.len as u64` | `Claim::end()` |
| 439 | `count as u64` | `claim`'s loop |
| 490 | `granted as u64` | `claim_up_to`'s loop |

All three are `usize → u64`. All three exist for the same reason —
`Seq::advanced_by( self, n : u64 )` takes a `u64`, so every count must widen at
the moment it becomes a position. On every target the family supports this is
lossless and free.

**Zero casts go the other way**, and that is the interesting half, because the
crate plainly needs one. `headroom() -> usize` is a count, and it is derived
from a *distance between two positions* — which is a `u64`:

```rust
pub const fn distance_to( self, later : Self ) -> u64   // ring_types/src/id.rs:82
```

The narrowing happens, just not here. `Claimer::headroom` forwards to
`GatingSet::headroom`, which forwards to `ring_seqno::free_slots`
(`ring_seqno/src/lib.rs:95-99`):

```rust
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
{
  let in_flight = consumer.distance_to( producer );
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
}
```

This used to be the reciprocal of this crate's three casts in the way that
mattered most: a `u64 → usize` narrow, sound only because `ring_claim`'s gate
never lets `in_flight` exceed `capacity` — a fact stated nowhere at the call
site. A refactor moved the narrow to the far side of the `saturating_sub`:
`capacity` widens to `u64` first, `in_flight` is subtracted from *that*, and
only the result — which can never exceed the widened `capacity` — narrows back
to `usize`. The bound comes from `saturating_sub` itself, not from any
caller's discipline, so the cast is lossless regardless of what `in_flight`
actually is, including a value `ring_claim`'s gate should never have produced
in the first place. The proof is now entirely local to this one line; there is
no longer a fact that lives in `ring_claim`, a representability guarantee that
lives in `ring_types::Capacity`, and a lossless conclusion that lives nowhere
— the three-crate chain this section used to describe no longer has anything
to prove.

`saturating_sub` was already worth noting as the one defensive construct: if
`in_flight` ever *did* exceed `capacity`, the result is 0 — the gate closes —
rather than a wrapped enormous headroom. The failure mode is back-pressure,
not a corrupt grant. What the refactor adds is that this same saturating
behaviour is now also *why* the narrowing cast is safe, not just a nicety
running alongside it.

### CL48 — The Crate Names Two of `ring_types`' Six Types and Never Computes a Position

```sh
cd "$(git rev-parse --show-toplevel)"
grep '^use' ring_claim/src/lib.rs
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs | grep -cE 'SlotIndex|%|mask|Capacity'
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
use ring_cursor::{ PaddedCursor, SeqCell, GATING };
use ring_gating::GatingSet;
use ring_types::{ RingError, Seq };
0
```

`ring_types` exports six public types. This crate imports two:

| `ring_types` exports | Used by `ring_claim` |
|----------------------|:--------------------:|
| `Seq` | ✔ |
| `RingError` | ✔ |
| `SlotIndex` | **✘** |
| `Capacity` | ✘ — reached only through `consumers.capacity()`, never named |
| `WaitKind` | ✘ |
| `OverflowPolicy` | ✘ |

And the search for `SlotIndex`, `%`, `mask`, and `Capacity` in the crate body
returns **zero matches**. `ring_claim` never converts a sequence into a position
in a buffer. It has no modulo, calls `Capacity::mask()` never, and constructs a
`SlotIndex` never — despite `SlotIndex` existing one crate down for exactly that
purpose and `Capacity::mask()` existing to make it a single `&`.

This is not an omission. It is the crate's boundary stated in types: **a `Claim`
is a range of sequences, not a range of slots.** The consequences are concrete
and show up throughout the corpus:

| Because there is no slot index | Consequence |
|--------------------------------|-------------|
| a `Claim` is not tied to a buffer | `overlaps` compares claims from *different rings*, and the exhaustive 900-pair test builds both sides from literals with no ring in scope ([`item/001`](../item/001_the_eight_readings_of_a_range.md)) |
| a `Claim` is not tied to a capacity | `Claim::new( Seq( 0 ), usize::MAX )` is constructible and meaningless, which is what makes the public constructor a cost ([`api/001`](../api/001_seventeen_items_and_nothing_that_drops_silently.md)) |
| the crate never reads or writes a slot | it cannot be benchmarked in isolation — a `Claimer` writes nothing ([`non_functional_requirement/002`](../non_functional_requirement/002_what_contention_costs.md)) |
| the wrap point is the sequence's, not the ring's | ~585 years, not one lap ([`lifecycle/002`](../lifecycle/002_the_claimer_over_a_rings_life.md)) |

The same boundary appears in `ring_types` itself, in field visibility:

| Type | Field | Why |
|------|-------|-----|
| `Seq( pub u64 )` | **public** | a label on a number; nothing to enforce |
| `SlotIndex( pub usize )` | **public** | same |
| `Capacity( usize )` | **private** | `new` returns `Result< Self, RingError >` — there is an invariant to hold |

The newtype that enforces something hides its field; the two that only name
something expose theirs. That is a coherent rule, and it is also what makes
[`item/001`](../item/001_the_eight_readings_of_a_range.md) § CL27 possible at
all: `ring_batch` reaches through `Seq`'s public `.0` to win two `const fn`s,
and this crate now does the same in `contains` and `overlaps` — both would be
flatly unavailable if `Seq` were shaped like `Capacity`. The choice that lets
two crates buy `const` is the same choice that lets any crate build a nonsense
`Seq`.

### Types

| File | Relationship |
|------|--------------|
| [002_the_lifetime_on_the_claimer.md](002_the_lifetime_on_the_claimer.md) | The other half of the type — the borrow, not the integers |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_two_loops_that_disagree_at_zero.md](../algorithm/002_two_loops_that_disagree_at_zero.md) | The two loops that depend on casts 406 and 451 |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_eight_readings_of_a_range.md](../item/001_the_eight_readings_of_a_range.md) | Where `Seq`'s public field buys `ring_batch` and this crate two `const fn`s each |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md](../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md) | The `u64` and the `usize` that make sixteen bytes |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_half_open_range_as_a_value.md](../pattern/002_the_half_open_range_as_a_value.md) | The sibling range type that chose `u64` for the same field |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_four_predicates_and_the_one_that_is_called.md](../integration/002_four_predicates_and_the_one_that_is_called.md) | The `GatingSet` call that reaches the narrowing cast |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:66-68` | Two of six `ring_types` exports imported |
| `ring_claim/src/lib.rs:146,439,490` | The three casts, all widening |
| `ring_seqno/src/lib.rs:95-99` | The reciprocal narrowing, and the `saturating_sub` that guards it |
| `ring_types/src/id.rs:25,82,99` | `Seq`'s public field, `distance_to`'s `u64`, `SlotIndex` |
| `ring_types/src/capacity.rs:23,40,75` | The one newtype with a private field, and the `mask()` this crate never calls |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:55` — `a_claim_is_half_open` | `start`/`end` as `Seq`, `len` as `usize`, in one assertion |
| `tests/claim_test.rs:112` — `overlap_is_symmetric_and_detects_every_shared_sequence` | 900 pairs built from literals, with no ring in scope |
| `tests/claim_test.rs:274` — `headroom_tracks_what_claiming_consumed` | The `usize` that came back through the narrowing cast |
| `tests/claim_test.rs:186` — `a_claim_wider_than_the_ring_is_a_configuration_error` | The one place a count is checked against a `Capacity` |
