# Pitfall: Implementing `may_claim` With `laps_between`

### Scope

- **Purpose**: Record the deduplication a competent reviewer proposes on first reading this crate, and show what it costs.
- **Responsibility**: Give the change, show it passes everything, quantify the cost, and identify where in the family that cost would land.
- **In Scope**: Collapsing the three equivalent boundary readings into one.
- **Out of Scope**: The width hazard that keeps them from being equivalent everywhere — see [`002`](002_reading_free_slots_on_a_narrow_target.md).

### The Change

The crate has three functions that answer the same boundary question
([`algorithm/001`](../algorithm/001_four_readings_of_one_subtraction.md)):

```rust
may_claim( p, c, n )     ≡  d < n
laps_between( c, p, n )  ≡  d / n           // == 0 exactly when d < n
free_slots( p, c, n )    ≡  n - d, sat.     //  > 0 exactly when d < n
```

Three implementations of one predicate is exactly the shape a reviewer flags. The
obvious repair:

```rust
#[ must_use ]
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
{
  laps_between( consumer, producer, capacity ) == 0
}
```

Four words shorter, one fewer independent expression, and the boundary now stated
in exactly one place.

### It Passes Everything

| Check | Result |
|-------|--------|
| `cargo test -p ring_seqno` | ✅ — every assertion holds; the two are equivalent for all inputs |
| `cargo test -p ring_seqno --doc` | ✅ — `may_claim`'s doctest is unchanged and still passes |
| `cargo clippy -- -D warnings` | ✅ — nothing to complain about |
| `free_slots_agrees_with_may_claim_across_two_laps` | ✅ — this is the test that would catch a *wrong* rewrite, and the rewrite is right |
| The manual plan's M2 | ✅ — the doc still states the exclusive boundary |
| A reviewer reading the diff | ✅ — it is strictly less code saying the same thing |

There is no check in the repository that fails. That is what makes it a pitfall
rather than a bug.

### What It Costs

**An integer division replaces an integer comparison.**

| Operation | Typical `x86_64` latency |
|-----------|--------------------------|
| `cmp` — 64-bit compare | ~1 cycle, fully pipelined |
| `div` — 64-bit unsigned division | ~20-40 cycles, poorly pipelined, blocks the divider |

Division is among the slowest integer instructions on every mainstream
architecture, and unlike most it is not pipelined — successive divisions serialise.

The capacity is a power of two, so a division *could* be a shift. But the
compiler cannot know that: `Capacity::get()` returns a plain `usize`, and the
power-of-two guarantee lives in a constructor two crates away
(`ring_types/src/capacity.rs:46`), invisible at this call site. It emits a real
`div`.

### Where the Cost Lands

`may_claim` is not incidental. Follow it down:

| Tier | Site |
|------|------|
| 1 | `ring_seqno::may_claim` |
| 2 | `ring_cursor:417-420` — `CursorPair::may_claim`, two atomic loads then this |
| 3 | `ring_wait:241` — `wait_until( kind, spins, \|\| pair.may_claim() )` |
| 3 | `ring_shutdown:620` — `pair.may_claim()` |

`ring_wait:241` is the one that matters:

```rust
wait_until( kind, spins, || pair.may_claim() ).map_err( | _ | RingError::Full )
```

That closure is a **spin-wait predicate**. It is evaluated in a tight loop for as
long as the ring stays full — thousands or millions of times per blocked
producer. Putting a `div` inside it, to save four words in a crate nobody reads
during a stall, is a bad trade in the one place the trade is measurable.

**The same reasoning applies to `free_slots`, and is why `may_claim` exists at
all.** `ring_cursor`'s source argues it directly:

> Exactly [`free_slots`] being non-zero, expressed as the question a caller
> actually asks. Kept as its own method because the two readings answer
> different questions.

See [`pattern/001`](../pattern/001_the_predicate_beside_its_quantity.md).

### The Shape Already Exists One Crate Over

This is not hypothetical. **Finding F10 in `ring_cursor`'s corpus, restated here
because this is the crate whose functions it wraps**: `ring_batch::claim_gated`
takes an ordering parameter and does not use it for most of its work.

```rust
// ring_batch/src/lib.rs:306-314
pub fn claim_gated< P : SeqCell, C : SeqCell >
( producer : &P, consumer : &C, count : usize, capacity : Capacity, order : Ordering )
-> Result< BatchClaim, RingError >
{
  if count > capacity.get() { return Err( RingError::BatchTooLarge { … } ); }

  let at     = producer.load( Ordering::Acquire );   // ← not `order`
  let behind = consumer.load( Ordering::Acquire );   // ← not `order`
  if ( free_slots( at, behind, capacity ) as usize ) < count { return Err( RingError::Full ); }

  Ok( claim( producer, count, order ) )              // ← `order` used, once
}
```

The signature offers a caller control over the ordering; two of the three atomic
operations ignore it. A generalisation that does not generalise — the same
category of change as the rewrite above: locally reasonable, correct, and
quietly wrong about what it is providing.

### What Would Make the Rewrite Safe

If the deduplication is wanted, invert it — implement the *expensive* reading in
terms of the cheap one, never the reverse:

```rust
// keep may_claim as the primitive comparison, and if a lap count is needed:
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
{
  earlier.distance_to( later ) / capacity.get() as u64   // unchanged — the division belongs here
}
```

`laps_between` has no callers in the family
([`workaround/002`](../workaround/002_laps_between_has_no_caller.md)), so its
division is never executed outside tests. Leaving it as the only site that
divides is exactly right: the crate pays for a division only when someone asks
for a lap count, which is never on the hot path.

A one-line test would pin the equivalence without importing the cost —
[`algorithm/001`](../algorithm/001_four_readings_of_one_subtraction.md) § One Edge
of the Lattice Has No Assertion gives it.

### SQ43 — The Warned-Against Shape, One Crate Over

The parameter is accepted and then ignored twice:

```
ring_batch::claim_gated( … , order : Ordering )
  operation 1   Acquire   hardcoded
  operation 2   Acquire   hardcoded
  operation 3   order     honoured
```

**Finding.** `ring_batch::claim_gated` takes an `order : Ordering` and hardcodes `Acquire` for two of its three atomic operations — the shape this instance warns about, already present one crate over.

---

### SQ44 — A Doc That Promises Saturation Over a Plain Addition

The body and the manifest together decide the behaviour, and neither saturates:

```
ring_types/src/id.rs:45   pub const fn next( self ) -> Self
                     46     Self( self.0 + 1 )

command grep -rn overflow-checks Cargo.toml */Cargo.toml   (no matches)
```

**Finding.** `Seq::next`'s doc claims release-mode saturation; the body is `Self( self.0 + 1 )` and no `overflow-checks` override exists, so release wraps.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_four_readings_of_one_subtraction.md](../algorithm/001_four_readings_of_one_subtraction.md) | The equivalence that invites the change |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_capacity_readings.md](../item/001_the_three_capacity_readings.md) | The three functions, and `laps_between`'s caller count |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_predicate_beside_its_quantity.md](../pattern/001_the_predicate_beside_its_quantity.md) | Why a predicate is kept beside the quantity it derives from |

### Pitfalls

| File | Relationship |
|------|--------------|
| [002_reading_free_slots_on_a_narrow_target.md](002_reading_free_slots_on_a_narrow_target.md) | The other hazard in these same three functions |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/002_laps_between_has_no_caller.md](../workaround/002_laps_between_has_no_caller.md) | Why the division is never executed in production |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:50-53, 73-76` | The two functions the change would merge |
| `ring_types/src/capacity.rs:46` | The power-of-two guarantee the compiler cannot see |
| `ring_wait/src/lib.rs:241` | The spin-wait predicate the division would land in |
| `ring_shutdown/src/lib.rs:620` | The other `may_claim` caller |
| `ring_batch/src/lib.rs:306-329` | The same shape, already present |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:71-88` | Passes before and after — it checks correctness, not cost |
| `tests/seq_test.rs:58-67` | `may_claim`'s boundary, likewise unaffected |
| `tests/manual/readme.md` M2 | Also passes — it reads the doc, not the body |
