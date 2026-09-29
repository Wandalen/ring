# Invariant: The Bound Is the Minimum and Only the Minimum

### Scope

- **Purpose**: State the crate's central invariant, decompose it into checkable clauses, and assess what tests each clause.
- **Responsibility**: Give the clauses, map each to its test, name the one clause with thin coverage, and show the implementation that would pass most of the suite.
- **In Scope**: The rule `headroom` enforces.
- **Out of Scope**: The empty-set case, which is not a minimum at all — see [`decisions/001`](../decisions/001_capacity_for_an_empty_set.md).

### The Invariant

> For a non-empty set, `headroom( p )` depends on exactly one cursor: the one
> holding the smallest sequence. No other consumer's position changes the answer,
> and a producer admitted by the gate never reaches a slot the slowest consumer
> has not finished with.

The crate's module documentation gives the reason in one sentence:

> A ring has one copy of each slot. A producer that laps *any* consumer
> overwrites data that consumer has not read, so the bound is the minimum across
> the whole gating set — not the average, not the median, and not the consumer
> that happens to be asking.

### The Clauses and What Checks Each

| # | Clause | Checked by | Strength |
|---|--------|------------|:--------:|
| C1 | The answer is a minimum, not a first element | `the_slowest_consumer_sets_the_bound_regardless_of_position_in_the_set` | Three positions, one capacity |
| C2 | Faster consumers buy nothing | `one_stalled_consumer_stops_the_producer_for_everyone` | One case, sharp |
| C3 | The boundary is exclusive at exactly one lap | `the_lap_boundary_is_exclusive_on_both_sides`, and the acceptance test | Both sides, both directions |
| C4 | The gate holds across a full lap under a stall | `a_stalled_consumer_stops_the_producer_at_exactly_one_lap` | The acceptance clause |
| C5 | Batched claims obey the same bound | `a_producer_never_passes_the_limit_over_a_full_lap_with_batches` | Five batches of five over 16 slots |
| C6 | The fold is not restated here | manual check M1 — `grep` for `.iter()`, `cursors[`, `.min(`, `.fold(` | Structural |

C4 is the one the crate's own test file singles out, and its reasoning is the
best statement of why the rest are not enough:

> A gate that is never under pressure is a gate that never gates. Every
> sequential test below can be passed by a `headroom` that returns `capacity`
> unconditionally — an empty ring has room for anything, and most tests start with
> an empty ring.

### The Implementation That Passes Most of the Suite

Two wrong implementations are worth naming, because each survives a large part of
the file:

```rust
// Wrong #1 — reads the first cursor rather than the minimum.
pub fn headroom( &self, producer : Seq ) -> usize
{
  self.cursors.first().map_or( self.capacity.get(), | c |
    ring_seqno::free_slots( producer, c.load( GATING ), self.capacity ) )
}
```

| Test | Verdict against Wrong #1 |
|------|:------------------------:|
| Every single-consumer test in the file | ✅ passes |
| `one_stalled_consumer_stops_the_producer_for_everyone` | ❌ caught — the stall is at index 1 |
| `the_slowest_consumer_sets_the_bound_regardless_of_position_in_the_set` | ❌ caught at `slow_index` 1 and 2 |
| `a_stalled_consumer_stops_the_producer_at_exactly_one_lap` | ✅ **passes** — the stalled consumer is index 0 |

The acceptance test — the one the file's own header calls "the test, not the
setup" — does not catch a first-element read, because it stalls index 0 and races
index 1. C1's coverage rests entirely on the two multi-consumer tests, and only
one of them varies the index.

```rust
// Wrong #2 — an inclusive boundary.
count <= self.headroom( producer ) + 1
```

Caught by C3 and C5 only. Every test that fills a ring from empty and stops
passes it, which is exactly what the test file's second header paragraph warns
about:

> an inclusive boundary passes every "the ring fills up" test and corrupts
> exactly one slot per lap under load — the failure that is hardest to reproduce
> and easiest to write.

### The Thin Clause

**C1 is the invariant's core and has the least coverage.** It is checked at three
consumer positions in a single three-consumer set at capacity 8, all with the
same slow position (`2`) and the same fast position (`100`):

```rust
// tests/gating_test.rs:133-147
for slow_index in 0..3
{
  let mut positions = [ 100u64; 3 ];
  positions[ slow_index ] = 2;
  let set = set_at( 8, &positions );

  assert_eq!( set.slowest(), Some( Seq( 2 ) ), "slow consumer at index {slow_index}" );
  assert_eq!( set.headroom( Seq( 8 ) ), 2 );
}
```

Three assertions of one shape. An implementation returning `cursors.iter().map(…)
.min()` and one returning `cursors.last()` — for a set whose slow consumer is
never last except in one iteration — are distinguished, but narrowly.

The gap is cheap to close, and the fix is a sweep rather than another example:

```rust
// not present
for slow_index in 0..4
{
  for slow_position in [ 0u64, 1, 7, 8 ]
  {
    let mut positions = [ 64u64; 4 ];
    positions[ slow_index ] = slow_position;
    let set = set_at( 8, &positions );
    assert_eq!( set.slowest(), Some( Seq( slow_position ) ) );
  }
}
```

16 combinations for four lines. Not added here — a test change belongs to a run
with its own verification — but recorded as the highest-value addition to this
crate's suite.

**The mitigating fact:** C6 makes C1 largely someone else's problem. The fold is
`ring_cursor::slowest`, which is shared with `ring_barrier` and tested there too,
and M1 asserts mechanically that this crate performs no iteration of its own. So
C1 can only break here by the crate reacquiring a fold — which M1 catches — rather
than by the fold being wrong.

That is the argument for the current coverage being adequate rather than
excellent, and it is worth stating: **the invariant is defended structurally
rather than exhaustively.** If M1 is ever deleted, C1's three assertions become
the only defence.

### GT23 — Three Tests, One That Varies the Index

```
62:fn a_stalled_consumer_stops_the_producer_at_exactly_one_lap()
134:fn the_slowest_consumer_sets_the_bound_regardless_of_position_in_the_set()
150:fn one_stalled_consumer_stops_the_producer_for_everyone()
```

A fold that returned the first element, or the last, would satisfy any test that
puts the slow consumer in a fixed position. Only one of the three moves it.

**Finding.** It has three tests, and only one of them varies the slow consumer's index — so only `the_slowest_consumer_sets_the_bound_regardless_of_position_in_the_set` would catch a fold that returned the first or last element rather than the least

---

### GT24 — The Rule Is Enforced One Crate Away

```
199:    ring_cursor::slowest( &self.cursors )     <- this crate's whole contribution
ring_cursor/src/lib.rs:120:pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
```

This crate's job is to hand over the whole slice rather than a subset. Whether
what comes back is a minimum is decided elsewhere, and asserted nowhere here.

**Finding.** In `ring_cursor::slowest`, in another crate. This crate's contribution is passing the whole slice; nothing here asserts the fold is a minimum, so a change to it reaches this crate as a behavioural test failure rather than a compile error

---


### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_headroom_in_two_delegations.md](../algorithm/001_headroom_in_two_delegations.md) | The computation the invariant constrains |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_set_that_cannot_grow.md](../data_structure/001_the_set_that_cannot_grow.md) | The multi-consumer path this invariant governs, and its lack of callers |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_capacity_for_an_empty_set.md](../decisions/001_capacity_for_an_empty_set.md) | The case where there is no minimum to take |

### Invariants

| File | Relationship |
|------|--------------|
| [002_this_crate_names_no_ordering.md](002_this_crate_names_no_ordering.md) | The structural check that keeps C6 true |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_producer_walking_a_lap_against_a_stall.md](../lifecycle/002_the_producer_walking_a_lap_against_a_stall.md) | C4 walked step by step |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:13-20` | The invariant, stated in prose |
| `ring_gating/src/lib.rs:197-228` | `slowest` and `headroom` — the two lines that implement it |
| `ring_cursor/src/lib.rs:120-123` | The fold C6 delegates to |
| `tests/manual/readme.md` § M1 | The structural check, with its expected empty output |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:61-85` | C4 — the acceptance clause, and the one that misses a first-element read |
| `tests/gating_test.rs:100-109` | C3 — both sides of the boundary |
| `tests/gating_test.rs:111-131` | C5 — the same bound under batching |
| `tests/gating_test.rs:133-147` | C1 — three positions, the thin clause |
| `tests/gating_test.rs:149-157` | C2 — two fast consumers buy nothing |
