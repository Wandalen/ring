# Workaround: `laps_between` Has No Caller

### Scope

- **Purpose**: Record that the crate's first export is unused outside itself despite claiming to be the decisive reading, show what each would-be consumer built instead, and give the disposition options.
- **Responsibility**: Establish the zero-caller fact mechanically, quote the two doc comments that both claim the same rule, document the reversed argument order, and assess each way out.
- **In Scope**: Findings **SQ53** and **SQ8**.
- **Out of Scope**: The cost of routing `may_claim` through it — see [`pitfall/001`](../pitfall/001_implementing_may_claim_with_laps_between.md).

### The Fact

```sh
cd "$(git rev-parse --show-toplevel)"
# not one caller outside the crate that defines it
grep -r 'laps_between' ring_*/src/*.rs ring_*/tests/*.rs \
  | grep -v '^ring_seqno/' | grep -vE ':\s*(///|//!|//)' \
  || echo '(no matches — no caller outside ring_seqno)'
# control — the identical expression for may_claim, the reading that is called
grep -r 'may_claim' ring_*/src/*.rs ring_*/tests/*.rs \
  | grep -v '^ring_seqno/' | grep -vE ':\s*(///|//!|//)' | head -4
```

Live output:

```
(no matches — no caller outside ring_seqno)
ring_cursor/src/lib.rs:  pub fn may_claim( &self ) -> bool
ring_cursor/src/lib.rs:    ring_seqno::may_claim( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
ring_shutdown/src/lib.rs:    pair.may_claim()
ring_wait/src/lib.rs:  wait_until( kind, spins, || pair.may_claim() ).map_err( | _ | RingError::Full )
```

Zero call sites outside `ring_seqno` — in source or in tests, across all 33 crates.
Inside the crate it is called only by its own doctest and by
`tests/seq_test.rs`, which exist to exercise it.

### Both Docs Claim the Same Rule

```rust
// ring_seqno/src/lib.rs:29-33
/// How far apart two sequences are, in laps of a given capacity.
///
/// A ring is safe to publish into exactly while the producer is less than one
/// full lap ahead of the slowest consumer. `laps_between` is the reading that
/// decides it.
```

```rust
// ring_seqno/src/lib.rs:55-61
/// Whether `producer` may claim without overwriting a slot `consumer` has not
/// yet reached.
///
/// True exactly while the producer is strictly less than one lap ahead. At
/// exactly one lap the next claim would land on the consumer's current slot,
/// so the boundary is exclusive …
```

Two functions, twenty-six lines apart, describing the same predicate. One of them
says it *is* the reading that decides. The other is the one everybody calls.

The claim is not false — `laps_between( c, p, cap ) == 0` is exactly
`may_claim( p, c, cap )`, and that equivalence is real
([`algorithm/001`](../algorithm/001_four_readings_of_one_subtraction.md)). It is
misleading: a reader arriving at line 32 is told to use the function nobody uses,
in the form that costs a division.

### What Each Consumer Built Instead

| Consumer | Wanted | Wrote | Rather than |
|----------|--------|-------|-------------|
| `ring_gating::limit` | the position one lap ahead of the slowest | `self.slowest().map( \| s \| s.advanced_by( self.capacity.get() as u64 ) )` | anything from this crate |
| `ring_claim`, `ring_wait`, `ring_batch` | "is it under one lap?" | `may_claim` / `headroom` | `laps_between( … ) == 0` |
| `ring_debug::check_seqs` | "is it over one lap?" | `producer.0 - consumer.0 > capacity.get() as u64` | `laps_between( … ) >= 1` |

`ring_gating::limit` is the closest miss. It performs the same multiplication by
capacity that `laps_between` divides by — the inverse operation, in the same
crate that owns the capacity, at `ring_gating/src/lib.rs:321-324`. If any
call site was going to route through a lap-counting helper it was that one, and it
did not, because what it needs is a *position* and `laps_between` returns a
*count*.

That is the whole diagnosis: **the family never wants a lap count.** It wants a
boolean (`may_claim`), a quantity of slots (`free_slots`, `headroom`), a quantity
of items (`pending`, `available`), or a position (`limit`). A number of laps is
none of those. The only plausible consumer is a human reading a diagnostic, and
`ring_debug` chose a comparison over a division for the reason in
[`001`](001_the_diagnostic_that_reimplements_the_readings.md).

### The Argument Order Is Reversed (Finding SQ8)

| Function | Signature | First parameter |
|----------|-----------|-----------------|
| `laps_between` | `( earlier : Seq, later : Seq, capacity )` | **the trailing position** |
| `may_claim` | `( producer : Seq, consumer : Seq, capacity )` | the leading position |
| `free_slots` | `( producer : Seq, consumer : Seq, capacity )` | the leading position |
| `pending` | `( producer : Seq, consumer : Seq )` | the leading position |

Three take `( ahead, behind )`. `laps_between` takes `( behind, ahead )`. Both
orders are individually defensible — `earlier`/`later` reads naturally left to
right, and `producer`/`consumer` matches the direction of data — and the crate
ships both.

The consequence is visible in its own test suite, three lines apart:

```rust
// tests/seq_test.rs:145-147
assert_eq!( laps_between( consumer, producer, c ), 99 );
assert!( !may_claim( producer, consumer, c ) );
assert_eq!( free_slots( producer, consumer, c ), 0 );
```

Line 145 passes `( consumer, producer )`; lines 146 and 147 pass
`( producer, consumer )`. All three are correct. A reader scanning them cannot
tell that from the shape.

**Nothing catches a swap**, because `Seq` is one type for both roles — see
[`data_structure/001`](../data_structure/001_the_crate_that_declares_no_type.md)
§ *What the Absence Costs*. A swapped `laps_between` returns `0`, which is the
"safe to publish" answer; a swapped `may_claim` returns `true`, likewise. Both
failures are permissive.

There is one mitigation, and it is deliberate:

```rust
// tests/seq_test.rs:49-53
/// A backward pair reads zero rather than an enormous number, so a caller that
/// swapped its arguments gets an obviously-wrong answer instead of a plausible
/// one.
fn laps_backward_read_zero()
{
  assert_eq!( laps_between( Seq( 100 ), Seq( 4 ), cap( 8 ) ), 0 );
}
```

"Obviously wrong" is doing heavy lifting there. `0` laps is obviously wrong to a
reader inspecting the value and indistinguishable from correct to a caller
branching on it.

### Disposition

| Option | Effect | Cost | Verdict |
|--------|--------|------|---------|
| **A. Delete it** | Removes SQ53 and SQ8 together; the crate drops to four functions | Loses the only reading that names laps, which this crate's own vocabulary ("lap bug") uses throughout | Tempting, and premature |
| **B. Reorder to `( producer, consumer, capacity )`** | Removes SQ8; all four readings then agree | A breaking signature change with zero external callers — the cheapest moment this will ever be | **Recommended** |
| **C. Correct the doc comment** | Removes the misleading "is the reading that decides"; points at `may_claim` | Three lines | **Recommended, independent of B** |
| **D. Find the caller** | SQ53 resolves by use | None identified — see the table above; the family wants booleans, spans and positions, not counts | Unlikely |
| **E. Leave it** | — | A reader is directed to an unused function in its costly form | Current state |

B and C together cost a signature edit, a doc edit, and four call-site updates
inside this crate's own tests. **B is strictly cheapest now**: the function has no
external callers today, so reordering breaks nothing, and every day it stays is a
day a caller might appear and make the change expensive.

Neither is applied here. Both are source changes to the crate under
documentation, which belongs to a run with its own verification — and C's wording
depends on whether B lands first.

### SQ53 — A Doc That Claims a Role the Function Does Not Hold

The claim and the call graph disagree:

```
ring_seqno/src/lib.rs:31-33
  /// A ring is safe to publish into exactly while the producer is less than one
  /// full lap ahead of the slowest consumer. `laps_between` is the reading that
  /// decides it.

callers of laps_between outside this crate: 0
callers of may_claim   outside this crate: ring_cursor, ring_shutdown, ring_wait
```

**Finding.** `laps_between` has zero callers outside this crate while its own doc claims it "is the reading that decides" publication safety — the role `may_claim` actually fills.

---

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_argument_order_split.md](../api/002_the_argument_order_split.md) | SQ8 in full, across the whole surface |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_four_readings_of_one_subtraction.md](../algorithm/001_four_readings_of_one_subtraction.md) | The equivalence that makes the doc's claim true-but-useless |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_crate_that_declares_no_type.md](../data_structure/001_the_crate_that_declares_no_type.md) | Why no newtype catches a swap |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_capacity_readings.md](../item/001_the_three_capacity_readings.md) | `laps_between`'s own contract and coverage |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_implementing_may_claim_with_laps_between.md](../pitfall/001_implementing_may_claim_with_laps_between.md) | What using it for the boolean would cost |

### Workarounds

| File | Relationship |
|------|--------------|
| [001_the_diagnostic_that_reimplements_the_readings.md](001_the_diagnostic_that_reimplements_the_readings.md) | The consumer that wanted `laps_between >= 1` and wrote a comparison |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:29-53` | The function and the claim |
| `ring_seqno/src/lib.rs:55-76` | `may_claim`, making the same claim and getting the callers |
| `ring_gating/src/lib.rs:321-324` | `limit` — the inverse arithmetic, hand-written |
| `ring_debug/src/lib.rs:303` | The `>= 1` case, written as a comparison |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:25-34` | Lap counting at the boundary |
| `tests/seq_test.rs:38-44` | Laps are relative, not absolute |
| `tests/seq_test.rs:49-53` | The backward-pair mitigation, and its limit |
| `tests/seq_test.rs:145-147` | Both argument conventions, three lines apart |
