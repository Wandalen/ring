# Item: The Three Capacity Readings

### Scope

- **Purpose**: Inventory `laps_between`, `may_claim` and `free_slots`, and show that their casts do not all go the same way.
- **Responsibility**: Give each function's signature, cast direction, constness and coverage, and account for the one with no callers.
- **In Scope**: The three functions taking a `Capacity`.
- **Out of Scope**: `pending` and `slowest` — see [`002`](002_the_two_readings_without_a_capacity.md).

### The Inventory

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^\/\/\/ assert_eq!\( laps_between\( Seq\( 0 \), Seq\( 17 \), cap \), 2 \);$/{ n1 = NR } n1 && NR >= n1 + 2 && NR <= n1 + 6 { print } /^\/\/\/ assert!\( may_claim\( Seq\( 4 \), Seq\( 1 \), cap \) \);  \/\/ consumer moved on$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 6 { print } /^\/\/\/ assert_eq!\( free_slots\( Seq\( 4 \), Seq\( 0 \), cap \), 0 \);$/{ n3 = NR } n3 && NR >= n3 + 2 && NR <= n3 + 7 { print }' ring_seqno/src/lib.rs
```

Live output:

```
#[ must_use ]
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
{
  earlier.distance_to( later ) / capacity.get() as u64
}
#[ must_use ]
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
{
  consumer.distance_to( producer ) < capacity.get() as u64
}
#[ must_use ]
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
{
  let in_flight = consumer.distance_to( producer );
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
}
```

| Function | Signature | `const`? | Cast | Returns | Line |
|----------|-----------|:--------:|------|---------|-----:|
| `laps_between` | `( earlier : Seq, later : Seq, capacity : Capacity ) -> u64` | ❌ *(could be)* | `usize as u64` — **widening** | `u64` | 50 |
| `may_claim` | `( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool` | ❌ *(could be)* | `usize as u64` — **widening** | `bool` | 73 |
| `free_slots` | `( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize` | ❌ *(could be)* | `usize as u64` widen, then `u64 as usize` narrow (of the bounded result) | `usize` | 95 |

All three are `#[ must_use ]`.

### The Casts Do Not Agree

```sh
cd "$(git rev-parse --show-toplevel)"
grep ' as u64\| as usize' ring_seqno/src/lib.rs
```

Live output:

```
  earlier.distance_to( later ) / capacity.get() as u64
  consumer.distance_to( producer ) < capacity.get() as u64
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
```

**This was once an asymmetry; it no longer is.** Two of the three moved the
*capacity* up to `u64` and compared in 64-bit; the third used to move the
*distance* down to `usize` before subtracting, discarding any bits above the
target's own `usize` width. On a 64-bit target the two widths are identical and
nothing distinguished them; on any narrower target the old narrowing cast in
`free_slots` truncated while the widening casts in `laps_between`/`may_claim`
did not — so the three readings, provably equivalent as arithmetic
([`algorithm/001`](../algorithm/001_four_readings_of_one_subtraction.md)),
stopped agreeing.

**Finding SQ37**, worked out with a concrete input in
[`non_functional_requirement/002`](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md)
and as a reader's trap in
[`pitfall/002`](../pitfall/002_reading_free_slots_on_a_narrow_target.md), is
fixed — see that file's SQ37/SQ38 disposition.

The asymmetry was worth stating as an item property because the fix was an item
change: `free_slots` now computes in `u64` and narrows only its *result*, which
is bounded by `capacity` and therefore always fits — exactly the body the Live
output above already shows.

### All Three Could Be `const fn`

**Finding SQ5.** None of the three carries the annotation, and all three accept
it. This was compiled rather than reasoned about — the bodies copied verbatim
with `fn` replaced by `const fn`, plus `const` items to force evaluation:

```rust
const CAP : Capacity = match Capacity::new( 8 ) { Ok( c ) => c, Err( _ ) => panic!() };
pub const LAPS  : u64   = laps_between( Seq( 0 ), Seq( 16 ), CAP );
pub const CLAIM : bool  = may_claim( Seq( 4 ), Seq( 0 ), CAP );
pub const FREE  : usize = free_slots( Seq( 3 ), Seq( 0 ), CAP );
```

`cargo check` — clean, exit 0. Integer division, comparison, `saturating_sub` and
both casts are all permitted in a `const fn` body, and `Capacity::get` and
`Seq::distance_to` are themselves `const`.

Every function these three call is `const`; none of these three is. See
[`api/001`](../api/001_five_functions_and_no_types.md) § Four of Five Could Be
`const fn` for what that does and does not cost.

### Coverage

| Function | Unit tests | Doctest | Cross-crate callers |
|----------|-----------:|:-------:|---------------------|
| `laps_between` | 5 — `:26`, `:39`, `:52`, `:137`, `:155` | ✅ `:40-48` | **none** |
| `may_claim` | 4 — `:60`, `:74`, `:137`, `:155` | ✅ `:63-71` | `ring_cursor:417` |
| `free_slots` | 3 — `:74` *(the sweep)*, `:95`, `:137` | ✅ `:85-93` | `ring_cursor:368`, `ring_gating:222`, `ring_batch:323` |

```sh
cd "$(git rev-parse --show-toplevel)"
for f in laps_between may_claim free_slots; do
  printf '%-14s %s\n' "$f" "$( grep -rn "ring_seqno::$f\|[^_a-z]$f(" ring_*/src/*.rs \
    | grep -v '^ring_seqno/' | grep -vE ':\s*(///|//!|//)' | wc -l )"
done
```

Live output:

```
laps_between   0
may_claim      4
free_slots     5
```

**`laps_between` has no caller anywhere in the family.** Not in `src/`, not in any
other crate's tests — only this crate's own suite and its doctest.

That is a finding on its own (**SQ27**), and it is sharper than a simple unused
function, because the doc claims otherwise:

> A ring is safe to publish into exactly while the producer is less than one
> full lap ahead of the slowest consumer. `laps_between` is the reading that
> decides it.

The reading that decides it is `may_claim`, which computes `d < n` directly and
never calls `laps_between`. The two are equivalent at the boundary and one of them
does the work. See [`workaround/002`](../workaround/002_laps_between_has_no_caller.md).

**The nearest thing to a caller computes the lap boundary by hand.**
`ring_gating::limit` answers "what sequence is one full lap past the slowest
consumer":

```rust
// ring_gating/src/lib.rs:321-324
pub fn limit( &self ) -> Option< Seq >
{
  self.slowest().map( | s | s.advanced_by( self.capacity.get() as u64 ) )
}
```

That is lap arithmetic against a capacity, written with `advanced_by` and its own
`usize as u64` cast, in a crate that already depends on `ring_seqno`. It is not
`laps_between` — it is the inverse direction, and no function here offers it — but
it is the closest the family comes to wanting this crate's lap vocabulary, and it
did not reach for it.

### `free_slots` Is the One That Carries Weight

Three cross-crate callers, and the two that matter both use it to gate a write:

| Caller | Use |
|--------|-----|
| `ring_gating:222` | `headroom` — how many slots a producer may claim, `None` resolving to full capacity |
| `ring_batch:323` | `if ( free_slots( at, behind, capacity ) as usize ) < count { return Err( RingError::Full ) }` |
| `ring_cursor:368` | `CursorPair::free_slots`, which `ring_spsc` and `ring_wait` read through |

`ring_batch:323`'s cast is worth a line of its own. `free_slots` already returns
`usize`; `as usize` there is a no-op today (**Finding SQ45**). It is harmless,
and it is the exact construct that would silently absorb a *widening* of the
return type to `u64` — the compiler would not warn, and the truncation would
appear in the caller instead of here. The repair the crate actually took went
the other way: `free_slots` kept its `usize` return and moved the narrowing onto
an already-`capacity`-bounded value, so the cast absorbed nothing and is still
there, still loaded for whoever widens next. Nothing reports it — not even
`clippy::unnecessary_cast`, which is warn-by-default and active in that very
file (**Finding SQ54**). A redundant cast is cheap to delete and expensive to
leave.

### SQ26 — The Narrowing One Is the Load-Bearing One

Of the four pair readings, exactly one crosses a width boundary downward:

```
free_slots   u64 -> usize   narrows        called by ring_cursor, ring_gating, ring_batch
may_claim    u64             does not       called by ring_cursor
pending      u64             does not       called by ring_cursor, ring_consume
laps_between u64             does not       called by nothing
```

**Finding.** `free_slots` is the only reading that narrows, and it is the one two other crates decide writes with.

---

### SQ27 — Zero, Across Thirty-Two Crates

The search covers every source and test file in the family outside this crate:

```
command grep -rn laps_between --include=*.rs */  (excluding ring_seqno/)
(no output)
```

**Finding.** `laps_between` has zero callers outside this crate — none in any of the other 32 crates, in production or in test code.

---

### SQ28 — Three Crossings, One Direction That Loses

Every cast in the crate, in source order — the third line now carries two casts,
not one, since the fix widens before it narrows:

```
52:  earlier.distance_to( later ) / capacity.get() as u64            usize -> u64   widening
75:  consumer.distance_to( producer ) < capacity.get() as u64         usize -> u64   widening
98:  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize   usize -> u64 widening, then bounded-result u64 -> usize narrowing
```

**Finding.** The crate crossed a width boundary three times — `capacity.get() as u64` twice, in `laps_between` and `may_claim`, neither of which could lose — and once more in `free_slots`, where the distance was narrowed with `… as usize` *before* capacity was subtracted, which could lose on any target where `usize` is under 64 bits. Of the three crossings, exactly one carried the risk.

**Disposition:** applied — `free_slots` now widens `capacity.get()` to `u64` first and narrows only the already-`capacity`-bounded result afterward (line 98 above), so the crate's one remaining narrowing cast can no longer lose on any target width. This is the same `free_slots` change [`non_functional_requirement/002`](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md)'s SQ37/SQ38 disposition documents in full — one change to `free_slots` closed SQ16, SQ28, SQ37 and SQ38 at once. Now prints:
`( capacity.get() as u64 ).saturating_sub( in_flight ) as usize`

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_four_readings_of_one_subtraction.md](../algorithm/001_four_readings_of_one_subtraction.md) | What the three compute, and their equivalence |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_five_functions_and_no_types.md](../api/001_five_functions_and_no_types.md) | The same three as a surface |
| [../api/002_the_argument_order_split.md](../api/002_the_argument_order_split.md) | Why `laps_between`'s parameters are reversed |

### Items

| File | Relationship |
|------|--------------|
| [002_the_two_readings_without_a_capacity.md](002_the_two_readings_without_a_capacity.md) | The two that perform no cast |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md) | The narrowing cast, with a failing input |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_reading_free_slots_on_a_narrow_target.md](../pitfall/002_reading_free_slots_on_a_narrow_target.md) | Why the disagreement is invisible in review |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/002_laps_between_has_no_caller.md](../workaround/002_laps_between_has_no_caller.md) | The function with no callers, kept |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:29-99` | The three functions and their docs |
| `ring_types/src/capacity.rs:60-63` | `Capacity::get`, returning `usize` — the origin of every cast |
| `ring_gating/src/lib.rs:219-222` | `headroom` |
| `ring_gating/src/lib.rs:321-324` | `limit` — lap arithmetic that did not use `laps_between` |
| `ring_batch/src/lib.rs:323` | The gate, and the redundant cast |
| `ring_cursor/src/lib.rs:351-420` | The three `CursorPair` methods that wrap these |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:71-88` | The sweep pinning `free_slots` to `may_claim` |
| `tests/seq_test.rs:26, 39, 52, 137, 155` | `laps_between`'s five call sites |
| `tests/seq_test.rs:92-101` | `free_slots` at both ends of its range |
| `tests/manual/readme.md` M2 | `may_claim`'s exclusive boundary, read by hand |
