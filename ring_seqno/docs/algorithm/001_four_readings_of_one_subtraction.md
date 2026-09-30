# Algorithm: Four Readings of One Subtraction

### Scope

- **Purpose**: Show that `laps_between`, `may_claim`, `free_slots` and `pending` are one subtraction presented four ways, and identify which of their equivalences the test suite actually pins.
- **Responsibility**: Give each function's expression, derive the equivalences between them, and mark the one that has no assertion behind it.
- **In Scope**: The four binary readings of `ring_seqno`.
- **Out of Scope**: `slowest`, which takes a slice — see [`002`](002_the_slowest_fold.md).

### The Four Expressions

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64$/,/^}$/p;/^pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool$/,/^}$/p;/^pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize$/,/^}$/p;/^pub fn pending( producer : Seq, consumer : Seq ) -> u64$/,/^}$/p' ring_seqno/src/lib.rs
```

Live output:

```
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
{
  earlier.distance_to( later ) / capacity.get() as u64
}
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
{
  consumer.distance_to( producer ) < capacity.get() as u64
}
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
{
  let in_flight = consumer.distance_to( producer );
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
}
pub fn pending( producer : Seq, consumer : Seq ) -> u64
{
  consumer.distance_to( producer )
}
```

| Function | Body | Capacity? | Returns |
|----------|------|:---------:|---------|
| `laps_between( earlier, later, capacity )` | `earlier.distance_to( later ) / capacity.get() as u64` | ✅ | `u64` |
| `may_claim( producer, consumer, capacity )` | `consumer.distance_to( producer ) < capacity.get() as u64` | ✅ | `bool` |
| `free_slots( producer, consumer, capacity )` | `( capacity.get() as u64 ).saturating_sub( distance ) as usize` | ✅ | `usize` |
| `pending( producer, consumer )` | `consumer.distance_to( producer )` | ❌ | `u64` |

Every one of them begins by computing the same quantity. Writing `d` for
`consumer.distance_to( producer )` and `n` for `capacity.get()`:

| Function | In terms of `d` and `n` |
|----------|-------------------------|
| `pending` | `d` |
| `laps_between` *(arguments swapped)* | `d / n` |
| `may_claim` | `d < n` |
| `free_slots` | `n - d`, saturating at `0` |

`pending` is `d` with nothing added at all — it is `Seq::distance_to` under
another name and with the arguments in the other order. The other three each
combine `d` with `n` once.

### The Equivalence Lattice

All three capacity readings answer the same boundary question, because `d` is a
non-negative integer and `n` is at least `1`:

| Equivalence | Why |
|-------------|-----|
| `may_claim ⟺ free_slots > 0` | `n - d > 0` ⟺ `d < n`. The saturation at zero is exactly the `d >= n` case |
| `may_claim ⟺ laps_between == 0` | `d / n == 0` ⟺ `d < n`, by integer division |
| therefore `free_slots > 0 ⟺ laps_between == 0` | transitively |

So the crate holds **three independent implementations of one predicate**. That
is not a defect — each returns a different thing and callers want different
things — but it means the three can drift, and only assertions stop them.

### One Edge of the Lattice Has No Assertion

```sh
cd "$(git rev-parse --show-toplevel)"/ring_seqno
grep 'may_claim\|free_slots\|laps_between' tests/seq_test.rs | grep assert
```

Live output:

```
    assert_eq!(laps_between(Seq(0), Seq(0), c), 0);
    assert_eq!(laps_between(Seq(0), Seq(7), c), 0);
    assert_eq!(laps_between(Seq(0), Seq(8), c), 1);
    assert_eq!(laps_between(Seq(0), Seq(15), c), 1);
    assert_eq!(laps_between(Seq(0), Seq(16), c), 2);
    assert_eq!(laps_between(Seq(1_000_000), Seq(1_000_008), c), 1);
    assert_eq!(laps_between(Seq(1_000_001), Seq(1_000_008), c), 0);
    assert_eq!(laps_between(Seq(100), Seq(4), cap(8)), 0);
    assert!(may_claim(Seq(0), Seq(0), c));
    assert!(may_claim(Seq(3), Seq(0), c));
    assert!(!may_claim(Seq(4), Seq(0), c), "one full lap ahead must be refused");
    assert!(!may_claim(Seq(5), Seq(0), c), "beyond a lap must be refused");
    assert!(may_claim(Seq(4), Seq(1), c));
    assert_eq!(free_slots(Seq(0), Seq(0), c), 4);
    assert_eq!(free_slots(Seq(1), Seq(0), c), 3);
    assert_eq!(free_slots(Seq(4), Seq(0), c), 0);
    assert_eq!(free_slots(Seq(100), Seq(0), c), 0, "saturates rather than wrapping");
    assert_eq!(laps_between(consumer, producer, c), 99);
    assert!(!may_claim(producer, consumer, c));
    assert_eq!(free_slots(producer, consumer, c), 0);
```

| Edge | Pinned by | Strength |
|------|-----------|----------|
| `may_claim ⟺ free_slots > 0` | `free_slots_agrees_with_may_claim_across_two_laps` (`seq_test.rs:71-88`) | **Swept** — 8 consumers × 20 producers, both laps, compared pairwise |
| `may_claim ⟺ laps_between == 0` | *nothing* | **Absent** |
| `free_slots > 0 ⟺ laps_between == 0` | *nothing* | Absent (follows from the above) |

`laps_between`'s boundary is pinned only by hand-written literals — `Seq( 7 )`
gives `0` and `Seq( 8 )` gives `1` at capacity 8 (`seq_test.rs:29-33`), and one
value at capacity 4. Those are the right values, but they check `laps_between`
against a human's arithmetic, not against the predicate that shares its boundary
and that the rest of the family actually decides with.

**Finding SQ1.** The suite proves two of the three readings cannot disagree and
leaves the third unbound. A one-line extension of the existing sweep would close
it, since the loop already has both positions in hand:

```rust
// inside free_slots_agrees_with_may_claim_across_two_laps
assert_eq!( laps_between( Seq( consumer ), Seq( producer ), c ) == 0, claimable );
```

It is not added here — a test change belongs to a run of its own.

### `positions_many_laps_apart_stay_comparable` Is the Load-Bearing Test

`seq_test.rs:134-148` is the one that justifies the crate:

```rust
let consumer = Seq( 8 );
let producer = Seq( 800 );
assert_eq!( producer.0 % 8, consumer.0 % 8 );          // folded: identical
assert_eq!( laps_between( consumer, producer, c ), 99 ); // unfolded: 99 laps
assert!( !may_claim( producer, consumer, c ) );
assert_eq!( free_slots( producer, consumer, c ), 0 );
```

The first assertion is the interesting one. It asserts the *failure mode* — that
a folded representation makes these two positions indistinguishable — and then
shows all three readings still separating them. Most tests assert what the code
does; this one first establishes what a plausible alternative implementation
would get wrong.

It also happens to be where the crate's argument-order split is most visible: line
145 passes `( consumer, producer )` and lines 146-147 pass `( producer, consumer )`.
Both are correct against their signatures. See
[`api/002`](../api/002_the_argument_order_split.md).

### The Degenerate Cases

| Input | `pending` | `laps_between` | `may_claim` | `free_slots` |
|-------|:---------:|:--------------:|:-----------:|:------------:|
| Caught up (`d = 0`) | `0` | `0` | `true` | `n` |
| Exactly one lap (`d = n`) | `n` | `1` | **`false`** | `0` |
| Consumer *ahead* (`d` saturates to `0`) | `0` | `0` | `true` | `n` |

The third row is the one to notice. `Seq::distance_to` saturates, so a consumer
ahead of its producer is arithmetically indistinguishable from a ring that has
published nothing — in all four readings simultaneously. That state is
unreachable through correct use and detectable only from outside; `ring_debug`
carries `Violation::ConsumerAheadOfProducer` for exactly this reason. See
[`decisions/002`](../decisions/002_saturating_rather_than_signed.md).

### SQ1 — One Boundary, Three Statements, One Sweep

The three readings all decide the same boundary, and the test suite pins one edge of the triangle:

```
free_slots_agrees_with_may_claim_across_two_laps  8 x 20 = 160 pairs swept
may_claim  <->  laps_between == 0                 no assertion, any file
```

**Finding.** Three predicates state the same boundary and only `may_claim` against `free_slots > 0` is swept; `may_claim` against `laps_between == 0` has no assertion anywhere in the crate.

---

### SQ2 — The Unpinned Edge Is the Reversed One

Writing the untested equivalence down is the whole demonstration — the two sides do not read alike:

```
may_claim( producer, consumer, cap )      == free_slots( producer, consumer, cap ) > 0   swept
may_claim( producer, consumer, cap )      == ( laps_between( consumer, producer, cap ) == 0 )
                                             ^^^^^^^^^^^^^^^^^^ arguments swap here
```

**Finding.** The single unasserted equivalence is also the only one whose statement requires reversing argument order, so the one edge no test pins is the edge a caller is most likely to write backwards.

```sh
cd "$(git rev-parse --show-toplevel)"
CARGO_TARGET_DIR=/tmp/claude-1001/-home-user1-pro-lib-yrd-gamedev-codename-space-sandbox-spike-asteroids-game-src-tmp2-tmp3-tmp4-tmp5/3412a9ab-7335-429d-b589-ca3ce37fb788/scratchpad/cargo_target_ring cargo test -p ring_seqno --all-features laps_between_zero_agrees_with_may_claim_across_two_laps -- --nocapture 2>&1 | command grep -E 'laps_between_zero|test result'
```

Live output:

```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test laps_between_zero_agrees_with_may_claim_across_two_laps ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s
```

**Disposition:** applied — added `laps_between_zero_agrees_with_may_claim_across_two_laps` to `tests/seq_test.rs`, sweeping `may_claim ⟺ laps_between == 0` across the same 8 consumers × 20 producers that `free_slots_agrees_with_may_claim_across_two_laps` already covers for the other edge, in the reversed argument order this equivalence actually requires. Verified via `cargo test -p ring_seqno --all-features`, 2026-09-04 — ring_seqno's 11 unit tests plus 5 doctests all pass. Now prints:
`test laps_between_zero_agrees_with_may_claim_across_two_laps ... ok`

---

### SQ3 — Three Divisions in Thirty-Three Crates

A division is the most expensive arithmetic the family performs, and it performs three:

```
ring_align/src/lib.rs:   a / CACHE_LINE != b / CACHE_LINE
ring_bench/src/lib.rs:   let producer = ( record / workload.records_per_producer() as Record ) as usize;
ring_seqno/src/lib.rs:     earlier.distance_to( later ) / capacity.get() as u64
```

**Finding.** All 33 crates contain exactly three `/` operators outside comments — `ring_align`'s cache-line membership test, `ring_bench::destination_of`'s producer decode, and `laps_between`'s lap count. None of the three is on a hot write or read path: the first runs once per padding check, the second inside a torn-read assertion in a benchmark harness, and the third in the only function in the family with no caller at all.

---

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_five_functions_and_no_types.md](../api/001_five_functions_and_no_types.md) | The same four as a surface |
| [../api/002_the_argument_order_split.md](../api/002_the_argument_order_split.md) | Why line 145 reads backwards from 146 |

### Algorithms

| File | Relationship |
|------|--------------|
| [002_the_slowest_fold.md](002_the_slowest_fold.md) | The fifth function, which is not a subtraction |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_saturating_rather_than_signed.md](../decisions/002_saturating_rather_than_signed.md) | Why the third degenerate row collapses |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_capacity_readings.md](../item/001_the_three_capacity_readings.md) | Signatures, casts and coverage for three of the four |
| [../item/002_the_two_readings_without_a_capacity.md](../item/002_the_two_readings_without_a_capacity.md) | `pending`, and why it adds nothing to `distance_to` |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_implementing_may_claim_with_laps_between.md](../pitfall/001_implementing_may_claim_with_laps_between.md) | The dedup the lattice invites, and what it costs |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:50-115` | All four readings |
| `ring_types/src/id.rs:74-77` | `distance_to` — the subtraction all four share |
| `ring_types/src/capacity.rs:60-63` | `Capacity::get`, the `n` in every expression |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:71-88` | The swept edge of the lattice |
| `tests/seq_test.rs:29-33` | `laps_between`'s boundary, by literal |
| `tests/seq_test.rs:134-148` | The load-bearing test, and the argument-order split |
| `tests/manual/readme.md` M2 | The exclusive boundary, read by hand |
