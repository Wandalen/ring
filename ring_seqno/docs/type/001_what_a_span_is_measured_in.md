# Type: What a Span Is Measured In

### Scope

- **Purpose**: Account for the three different return types across four readings that compute the same quantity, and identify what the type system does and does not separate here.
- **Responsibility**: Give each return type with its reason, show where the reasons conflict, and name the two quantities the compiler cannot tell apart.
- **In Scope**: Return types of `laps_between`, `may_claim`, `free_slots` and `pending`.
- **Out of Scope**: `slowest`'s `Option` — see [`002`](002_the_option_that_slowest_returns.md).

### Four Readings, Three Return Types

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^pub fn ' ring_seqno/src/lib.rs
```

Live output:

```
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
pub fn pending( producer : Seq, consumer : Seq ) -> u64
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
```

| Function | Returns | Unit | Bounded by |
|----------|---------|------|------------|
| `pending` | `u64` | publications | nothing — any `u64` |
| `laps_between` | `u64` | laps | nothing |
| `free_slots` | `usize` | slots | `capacity` |
| `may_claim` | `bool` | — | — |

All four derive from one `u64` subtraction
([`algorithm/001`](../algorithm/001_four_readings_of_one_subtraction.md)). Three
different types come out.

### Each Choice Is Locally Right

| Return | Reason |
|--------|--------|
| `pending` → `u64` | An unread count is a *span of sequences*, and a `Seq` is a `u64`. It can legitimately exceed `capacity` — that is `ring_debug`'s D2 violation, detectable precisely because the type permits the value |
| `laps_between` → `u64` | Same domain, divided. Also unbounded |
| `free_slots` → `usize` | A *slot count*, and slots are indexed by `usize`. Every caller compares it against a `usize` — `count` in `ring_batch:323`, a batch width elsewhere |
| `may_claim` → `bool` | A question, not a quantity |

`free_slots`'s `usize` is the one carrying real weight. Its consumers all want a
`usize`:

```rust
// ring_batch/src/lib.rs:323
if ( free_slots( at, behind, capacity ) as usize ) < count      // count : usize

// ring_gating/src/lib.rs:217-222
pub fn headroom( &self, producer : Seq ) -> usize               // returns it onward
```

Returning `u64` here would put a cast at every call site instead of one inside
the function. That is a defensible trade and it is the one taken.

### Where the Reasons Conflict

The trade had a cost the source did not name. `free_slots` used to be the only
reading that **narrowed**:

```rust
let in_flight = consumer.distance_to( producer ) as usize;   // u64 → usize, line 92 [ pre-fix ]
```

`laps_between` and `may_claim` widen instead (`capacity.get() as u64`), so on a
target where `usize` is under 64 bits the three provably-equivalent readings used
to stop agreeing — worked out with a failing input in
[`nfr/002`](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md).

**The return type is what made the narrowing look deliberate.** A reader who
noticed `as usize` on line 92 and checked the signature found `-> usize`,
concluded the cast was there to match, and moved on. It *was* there to match —
and it was in the wrong place. The fix narrows the result rather than the
operand, which is exact because the result is bounded by `capacity`:

```rust
( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
```

Same signature, same call sites, no truncation. The return type was never the
problem.

### Two Quantities the Compiler Cannot Separate

`free_slots` returns `usize`. So does `Capacity::get`. So does `SlotIndex::get`.
Three different things in one type, and two of them share a range:

| Quantity | Type | Range | Meaning |
|----------|------|-------|---------|
| A free-slot **count** | `usize` | `0..=capacity` | how many more may be written |
| A **slot index** | `usize` (via `SlotIndex`) | `0..capacity` | where in the buffer something is |

They overlap on `0..capacity`. So this compiles:

```rust
let n = free_slots( producer, consumer, capacity );   // a count
buffer[ n ]                                            // used as an index
```

No warning, and for most values no panic — just the wrong element. The only
protection is that `ring_index` wraps its result in `SlotIndex`, a newtype. This
crate's count has no such wrapper.

**That is not a defect in `ring_seqno`, and it is worth being precise about why.**
The never-fold invariant ([`invariant/001`](../invariant/001_the_sequence_is_never_folded_here.md))
says no `SlotIndex` is produced here, and that is true of the type name. It is not
true of the representation — a bare `usize` in `0..capacity` is exactly what a
slot index looks like. The invariant holds in the sense it was written and does
not hold in the sense a reader might take from it.

A `FreeSlots( usize )` newtype would close it. That is a family-wide decision
about how many newtypes the vocabulary should carry, and `ring_types` already
declines it once — `Capacity::get` returns a bare `usize` too.

### The Types Agree With `ring_types` Except in One Place

| `ring_types` item | Returns | This crate's counterpart | Returns |
|-------------------|---------|--------------------------|---------|
| `Seq::distance_to` | `u64` | `pending` | `u64` ✅ |
| `Capacity::get` | `usize` | — | — |
| `SlotIndex::get` | `usize` | — | — |

`pending` matching `distance_to` exactly is what makes it a pure rename
([`item/002`](../item/002_the_two_readings_without_a_capacity.md)) — same
arguments, same type, same value, different name and order.

### What the Types Do Not Promise

| # | Not promised | Consequence |
|---|--------------|-------------|
| N1 | That a `u64` span fits in a `usize` | The narrowing above |
| N2 | That a returned count is fresh | These are pure functions over values; the caller loaded the cursors, and they may already have moved |
| N3 | That a count and an index are different | The overlap above |
| N4 | Any direction information | A backward pair returns `0`, not an error — see [`decisions/002`](../decisions/002_saturating_rather_than_signed.md) |

N2 is the one that matters most in practice and is the least visible in the
signature. `free_slots( producer, consumer, capacity )` takes values, so by the
time it returns, the real cursors may hold anything. `CursorPair::free_slots`
loads both at `GATING` and calls straight through — the answer is a snapshot, and
nothing in either signature says so. It is why
`may_claim_and_free_slots_never_disagree` in `ring_cursor`'s suite fixes the
state before reading, and why that test verifies the arithmetic agrees rather
than that two calls under contention would (F14 in `ring_cursor`'s corpus).

### SQ47 — The Same Unit, Two Return Types

Two functions count the same kind of thing and disagree about the type of a count:

```
free_slots( … ) -> usize   a count of slots
pending   ( … ) -> u64     a count of items

both are counts of positions between two Seq values.
```

**Finding.** `free_slots` returns `usize` and `pending` returns `u64` for quantities in the same units, which is what makes the narrowing cast look deliberate.

---

### SQ48 — Two Meanings, One Type, Overlapping Ranges

The ranges overlap almost entirely and the types are identical:

```
free slot count   usize in 0..=capacity
slot index        usize in 0..capacity

A function taking either will accept the other.
```

**Finding.** A free-slot count and a slot index are both `usize` in `0..=capacity`, so the compiler cannot separate them.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_four_readings_of_one_subtraction.md](../algorithm/001_four_readings_of_one_subtraction.md) | The one subtraction behind the three types |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_crate_that_declares_no_type.md](../data_structure/001_the_crate_that_declares_no_type.md) | Why `Seq` and `Capacity` live upstream |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_sequence_is_never_folded_here.md](../invariant/001_the_sequence_is_never_folded_here.md) | The clause that holds by name and not by representation |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_capacity_readings.md](../item/001_the_three_capacity_readings.md) | The cast column, per function |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md) | N1, with a failing input |

### Types

| File | Relationship |
|------|--------------|
| [002_the_option_that_slowest_returns.md](002_the_option_that_slowest_returns.md) | The fifth return type |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:50, 73, 95, 112` | The four signatures |
| `ring_types/src/id.rs:82, 110` | `distance_to` and `SlotIndex::get` |
| `ring_types/src/capacity.rs:60` | `Capacity::get`, also a bare `usize` |
| `ring_batch/src/lib.rs:323` | Where the `usize` return is compared against a `usize` |
| `ring_gating/src/lib.rs:226` | Where it is passed onward unchanged |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:92-101` | `free_slots` across its whole `0..=capacity` range |
| `tests/seq_test.rs:103-109` | `pending` unbounded by capacity |
| — | No test distinguishes a count from an index; the types do not either |
