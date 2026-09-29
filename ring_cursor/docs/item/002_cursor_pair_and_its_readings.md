# Item: `CursorPair` and Its Readings

### Scope

- **Purpose**: Inventory the eight associated functions on `CursorPair`, and show that their `const`ness partitions them exactly by what they touch.
- **Responsibility**: Give each function with its signature, `const`ness, atomic loads, and coverage, and account for the one that is neither `const` nor loading.
- **In Scope**: `CursorPair`'s eight associated functions.
- **Out of Scope**: `PaddedCursor`'s six, which is [`item/001`](001_padded_cursor_and_its_functions.md).

### The Inventory

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^\s*pub (const )?fn ' ring_cursor/src/lib.rs | sed -n '5,13p'
```

Live output:

```
  pub const fn new( capacity : Capacity ) -> Self
  pub fn new( capacity : Capacity ) -> Self
  pub const fn producer( &self ) -> &PaddedCursor
  pub const fn consumer( &self ) -> &PaddedCursor
  pub const fn capacity( &self ) -> Capacity
  pub fn free_slots( &self ) -> usize
  pub fn pending( &self ) -> u64
  pub fn may_claim( &self ) -> bool
  pub fn on_distinct_lines( &self ) -> bool
```

| Function | Signature | `const`? | Atomic loads | Line |
|----------|-----------|:--------:|:------------:|-----:|
| `new` | `( Capacity ) -> Self` | ✅ *(not under `loom`)* | 0 | 270 / 283 |
| `producer` | `( &self ) -> &PaddedCursor` | ✅ | 0 | 311 |
| `consumer` | `( &self ) -> &PaddedCursor` | ✅ | 0 | 333 |
| `capacity` | `( &self ) -> Capacity` | ✅ | 0 | 347 |
| `free_slots` | `( &self ) -> usize` | ❌ | **2** | 368 |
| `pending` | `( &self ) -> u64` | ❌ | **2** | 387 |
| `may_claim` | `( &self ) -> bool` | ❌ | **2** | 417 |
| `on_distinct_lines` | `( &self ) -> bool` | ❌ | 0 | 438 |

### `const`ness Partitions Them Almost Perfectly

| Group | Functions | Why |
|-------|-----------|-----|
| `const` | `new`, `producer`, `consumer`, `capacity` | Structural — they hand back what is already there |
| not `const`, loading | `free_slots`, `pending`, `may_claim` | An atomic load is not a compile-time operation |
| not `const`, **not** loading | `on_distinct_lines` | — |

**`on_distinct_lines` is the odd one, and its reason is inherited.** It calls
`PaddedCursor::addr`, which casts a pointer to a `usize`, and pointer-to-integer
casts are forbidden in `const fn`. Its own callee in `ring_align` *is* `const`:

```rust
// ring_align/src/lib.rs:138
pub const fn on_distinct_lines( a : usize, b : usize ) -> bool
{
  a / CACHE_LINE != b / CACHE_LINE
}
```

So a `const` predicate is made non-`const` by the way its arguments are obtained.
Nothing is wrong — an address genuinely is not known at compile time — but the
signature reads as though the comparison were the expensive part, and it is not.

### The Three Readings Are One Shape

```rust
ring_seqno::free_slots( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
ring_seqno::pending   ( self.producer.load( GATING ), self.consumer.load( GATING ) )
ring_seqno::may_claim ( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
```

Two loads at `GATING`, hand off to `ring_seqno`, return. **Six loads across three
methods** — which is what `tests/manual/readme.md` M4 counts, and why it must use
`grep -o | wc -l` rather than `grep -c`: both loads sit on one line each.

`pending` is the one M4 singles out:

> `pending` is the one to look at twice: it takes no capacity, so a reader may
> assume it takes fewer cursors too.

It takes the same two cursors as the others and one fewer *parameter*, which is
exactly the shape that invites the one-cursor defect in
[`pitfall/002`](../pitfall/002_a_reading_that_consults_one_cursor.md).

### Why `may_claim` Exists Beside `free_slots`

It is `free_slots() != 0` with the loads done once instead of twice, and the
source argues for it explicitly:

> Exactly [`free_slots`] being non-zero, expressed as the question a caller
> actually asks. Kept as its own method because the two readings answer
> different questions.

`may_claim_and_free_slots_never_disagree` is the test that holds them together,
and its limitation is documented in
[`algorithm/002`](../algorithm/002_three_readings_of_two_cursors.md): it fixes the
state before reading, so it verifies the *arithmetic* agrees, not that two
separate calls under contention would.

### Coverage

| Function | Unit tests | Doctest | Cross-crate |
|----------|-----------:|:-------:|-------------|
| `new` | all pair tests | ✅ | `ring_spsc:302` |
| `producer` | many | ✅ | `ring_wait`, `ring_shutdown`, `ring_debug` |
| `consumer` | many | ✅ | same |
| `capacity` | 1 | ✅ | `ring_debug:274` |
| `free_slots` | 6 | ✅ | `ring_spsc` |
| `pending` | 4 | ✅ | `ring_spsc` |
| `may_claim` | 5 | ✅ | `ring_spsc` |
| `on_distinct_lines` | 2 | — | `ring_spsc:364` — the only cross-crate call |

**`on_distinct_lines` is the only one without a doctest**, and the only one whose
cross-crate use is a single delegation. It is also the function whose absence
from `ring_mpsc`'s reach produced the family's forked line predicate — see
[`integration/002`](../integration/002_who_reads_a_cursor.md).

### What the Inventory Does Not Include

| # | Absent | Consequence |
|---|--------|-------------|
| I1 | Any mutator | A pair is advanced through `producer()`/`consumer()` and `SeqCell`, never through the pair |
| I2 | Any constructor taking initial positions | A pair always starts at `(ZERO, ZERO)` — which is what makes [`pitfall/002`](../pitfall/002_a_reading_that_consults_one_cursor.md) invisible in most tests |
| I3 | `laps` or any lap counter | `ring_seqno::laps_between` exists and this crate never calls it |
| I4 | Any validity check | `ring_debug::check` is a separate crate — see [`lifecycle/002`](../lifecycle/002_a_pair_across_a_full_lap.md) |

**I1 is the deliberate one.** The pair reads; the cursors write. That split is
why `writing_one_cursor_leaves_the_other_alone` is a test — it pins that the pair
is not secretly coupling the two.

### CU27 — An Equivalence Stated as Identity Between Two Separate Readings

`may_claim`'s doc comment says it is "exactly `free_slots` being non-zero". The
implementation is its own two-load reading delegating to `ring_seqno::may_claim`,
not a call to `free_slots`.

**Finding.** Two separate readings of the same two cursors are taken at two
different instants, so under concurrent writers they can disagree — `free_slots`
can return zero and a `may_claim` a nanosecond later return true. The
equivalence holds for the arithmetic and not for the readings, and the doc
comment states it as though it held for both.

---

### CU28 — The Crate's Most Consistent Convention Is Documented Nowhere

```
must_use attributes: 13
public items: 17
items that cannot carry one: 4 (the trait impl methods)
```

Every constructor and every reading carries `#[ must_use ]`. The only four
without it are the `SeqCell` methods, where the trait declares it instead.

**Finding.** Thirteen out of thirteen eligible items — no exceptions, no drift —
and no document in this corpus names the convention. It is the most consistently
applied rule in the crate and the one a new item is most likely to omit, because
nothing states it and nothing checks it.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_three_readings_of_two_cursors.md](../algorithm/002_three_readings_of_two_cursors.md) | What the three loading functions compute |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_surface_that_decides.md](../api/002_the_surface_that_decides.md) | The same eight as a surface |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_cursor_pair.md](../data_structure/002_the_cursor_pair.md) | The three fields these operate on |

### Items

| File | Relationship |
|------|--------------|
| [001_padded_cursor_and_its_functions.md](001_padded_cursor_and_its_functions.md) | The six on the component type |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_a_pair_across_a_full_lap.md](../lifecycle/002_a_pair_across_a_full_lap.md) | I2 and I4 — where the readings go blind |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_a_reading_that_consults_one_cursor.md](../pitfall/002_a_reading_that_consults_one_cursor.md) | The defect the six loads exist to prevent |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:247-442` | All eight |
| `ring_align/src/lib.rs:138` | The `const` predicate `on_distinct_lines` cannot inherit |
| `ring_seqno/src/lib.rs:73, 95, 112` | The three functions the readings delegate to |
| `ring_spsc/src/lib.rs:364` | `on_distinct_lines`'s only cross-crate caller |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:177-318` | The three readings, across nine tests |
| `tests/cursor_test.rs:300-318` | All six loads exercised by moving one cursor |
| `tests/cursor_test.rs:319-333` | I1 — writing one cursor leaves the other alone |
| `tests/manual/readme.md` M4 | The load count, and its corrected instrument |
