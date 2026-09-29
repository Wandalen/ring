# Pitfall: The Empty Barrier Is the Case Nobody Measured

### Scope

**Purpose:** Record the family's best-argued semantic asymmetry alongside the
operational blind spot that sits in exactly the same place, and show why being
right about the meaning of a case is no protection against being wrong about
its cost.

**Responsibility:** The behaviour of `Consumer::available` and
`Consumer::commit_available` when the barrier has no dependencies — semantically,
and as measured.

**In Scope:** `Barrier::frontier`'s `None`; `ring_gating`'s opposite default;
the three crates that test the asymmetry; the allocation measurement that the
same case concealed, and the one that now keeps it on the record deliberately.

**Out of Scope:** The allocation itself, which is
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md)
CN34. The store that used to be unconditional, which is
[`001`](001_commit_available_does_not_call_commit.md) CN44.

---

## The Asymmetry, and How Well It Is Handled

An empty `Barrier` yields nothing available. An empty `GatingSet` yields the
whole capacity. Two dependency sets, both empty, opposite answers.

```sh
cd "$(git rev-parse --show-toplevel)"

# the argument, in ring_barrier's own module documentation
sed -n '/## Why the empty set has no frontier/,/^$/p' ring_barrier/src/lib.rs

# the opposite default, in ring_gating
command grep -m1 -A8 -F '  /// assert_eq!( set.headroom( Seq( 4 ) ), 2, "the consumer released two slots" );' ring_gating/src/lib.rs | tail -n 8

# every test that asserts it
grep 'no_dependencies\|ungated\|Barrier::over( &\[\] )' \
  ring_consume/tests/consume_test.rs \
  ring_gating/tests/gating_test.rs \
  ring_barrier/tests/barrier_test.rs
```

Live output:

```
//! ## Why the empty set has no frontier
//!
//! [`Barrier::frontier`] returns `None` for a barrier with no dependencies, and
//! [`Barrier::available`] then returns zero — the opposite of `ring_gating`,
//! where an empty set means *unbounded*. The asymmetry is not an inconsistency:
//! a producer with nobody reading behind it can write freely, while a consumer
//! with nothing published in front of it has nothing to read. In both cases the
//! empty set means "no constraint from dependencies", and in both cases that
//! resolves to the value a dependency-free participant actually has available.

  /// ```
  #[ must_use ]
  pub fn headroom( &self, producer : Seq ) -> usize
  {
    self.slowest().map_or( self.capacity.get(), | slowest |
    {
      ring_seqno::free_slots( producer, slowest, self.capacity )
    } )
ring_consume/tests/consume_test.rs:fn a_consumer_with_no_dependencies_never_has_anything_available()
ring_consume/tests/consume_test.rs:  let consumer = Consumer::new( &position, Barrier::over( &[] ) );
ring_gating/tests/gating_test.rs:fn an_ungated_ring_has_a_full_capacity_of_headroom()
ring_gating/tests/gating_test.rs:  // every ungated ring would deadlock.
ring_gating/tests/gating_test.rs:  let ungated = GatingSet::new( cap( 4 ), 0 );
ring_gating/tests/gating_test.rs:  assert_eq!( ungated.slowest(), None );
ring_gating/tests/gating_test.rs:  assert_eq!( ungated.limit(), None );
ring_gating/tests/gating_test.rs:    assert_eq!( ungated.headroom( Seq( producer ) ), 4, "at producer {producer}" );
ring_gating/tests/gating_test.rs:    assert!( ungated.admits( Seq( producer ), 4 ) );
ring_gating/tests/gating_test.rs:fn an_ungated_ring_and_a_consumer_at_zero_disagree_after_one_lap()
ring_gating/tests/gating_test.rs:  let ungated = GatingSet::new( cap( 4 ), 0 );
ring_gating/tests/gating_test.rs:  assert_eq!( ungated.headroom( Seq::ZERO ), gated_at_zero.headroom( Seq::ZERO ), "same while empty" );
ring_gating/tests/gating_test.rs:  assert_ne!( ungated.headroom( Seq( 4 ) ), gated_at_zero.headroom( Seq( 4 ) ), "and different after a lap" );
ring_barrier/tests/barrier_test.rs:  assert!( Barrier::over( &[] ).admits( Seq::ZERO, 0 ), "even with no dependencies at all" );
ring_barrier/tests/barrier_test.rs:  let barrier = Barrier::over( &[] );
ring_barrier/tests/barrier_test.rs:    Barrier::over( &[] ).wait_for( Seq::ZERO, 1, WaitKind::None, 4 ),
ring_barrier/tests/barrier_test.rs:  let empty = Barrier::over( &[] );
```

`ring_barrier`'s module documentation argues it directly:

> [`Barrier::frontier`] returns `None` for a barrier with no dependencies, and
> [`Barrier::available`] then returns zero — the opposite of `ring_gating`,
> where an empty set means *unbounded*. The asymmetry is not an inconsistency:
> a producer with nobody reading behind it can write freely, while a consumer
> with nothing published in front of it has nothing to read. In both cases the
> empty set means "no constraint from dependencies", and in both cases that
> resolves to the value a dependency-free participant actually has available.

And `ring_gating:219` is the code the argument describes:

```rust
self.slowest().map_or( self.capacity.get(), | slowest | ...
```

`map_or` with the full capacity as the default — empty means unbounded. Against
`ring_consume:342`:

```rust
.map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );
```

`map_or` with zero as the default — empty means nothing. The same combinator,
the same shape, opposite defaults, and a paragraph explaining why.

### CN45 — The Asymmetry Is Argued in One Crate and Asserted in Three

This is recorded as a positive finding, which is unusual for a pitfall
document, because it is the shape the rest of the family's near-misses would
have taken if they had been handled this way.

| Where | What it does |
|-------|--------------|
| `ring_barrier/src/lib.rs` module doc | argues the asymmetry, names both sides, gives the unifying reading |
| `ring_gating/tests/gating_test.rs`, at `// The distinction the module documentation argues` | asserts the unbounded default, and asserts that the two states a `Seq::ZERO` implementation would conflate stay distinct |
| `ring_barrier/tests/barrier_test.rs` | asserts `frontier()` is `None` and `available()` is zero |
| `ring_consume/tests/consume_test.rs`, at `// the asymmetry with \`ring_gating\`` | asserts it through the operation a caller actually reaches, with a comment pointing back at the argument |

Four artifacts across three crates, and the one furthest from the decision —
this crate's test — carries a comment naming where the reasoning lives:

```rust
// An empty barrier means nothing has been published, not "everything" —
// the asymmetry with `ring_gating` that `ring_barrier`'s own suite asserts,
// carried through to the operation a caller actually uses.
```

That is the pattern the corpus keeps finding absent elsewhere: a decision
argued once, at the crate that owns it, and referenced rather than restated at
every crate that inherits it. `ring_claim`'s corpus recorded four disciplines
that are followed exceptionlessly and written down nowhere. This one is
followed and written down.

**Cost:** none. Recorded as the counter-example.

---

### CN46 — The Same Case, Measured, Is the One That Reports Zero

Semantically the empty barrier is the best-handled case in the crate.
Operationally it is the one case that measured nothing, and that is exactly why
the allocation on the read path went unrecorded until it was measured
deliberately.

From the probe as it stood in
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md)
— frozen, because the allocation it caught has since been removed and every row
below now reads zero:

```
one dependency, 300 readable:
  Consumer::available()            1000
  Consumer::commit_available()     1000
empty barrier, frontier() is None:
  Consumer::available()            0
  Consumer::commit_available()     0
```

The mechanism: `Barrier::frontier` calls `ring_cursor::slowest`, which collected
cursor positions into a `Vec`. With no dependencies, the slice is empty, and a
zero-length `Vec` never touches the allocator — Rust's `Vec` allocates lazily.
So the empty case allocated nothing, truthfully, about a configuration no
working ring is ever in.

The `Vec` is gone (`b7e075ca`), which does not retire this finding — it sharpens
it. The two empty-barrier rows now read zero on both sides of that change, so
they are the two rows in the replacement measurement that can distinguish
nothing at all. A suite made only of rows like them would have reported success
before the fix, after the fix, and at every point in between.

The trap this sets:

**A test written against an empty barrier measures zero and is right.** Every
assertion it makes about allocation holds. It simply holds about the one input
where the code path terminates before reaching the allocating line.

**The empty case is the natural one to reach for.** It needs no publisher, no
second cursor, no setup — `Barrier::over( &[] )` and a default cursor. Of the
21 tests in `consume_test.rs`, the one that uses it is the one testing the
empty-set semantics, which is correct. A test asking "does this allocate?" would
very plausibly have been written the same cheap way. `tests/allocation_test.rs`
was written the expensive way for that reason — a published cursor at 4096, a
consumer over it, four rows measured against a barrier that actually has a
dependency — and the two cheap rows are kept beside them, labelled in their own
assertion messages as proving nothing.

**Being right about the semantics gave no protection.** Four artifacts across
three crates reason carefully about what the empty barrier *means*. Not one of
them asks what any barrier *costs*. The two questions are independent, and
careful attention to the first is not evidence of attention to the second.

That is the finding worth carrying out of this document, and it generalises
past this crate: the case a design argues about most is not thereby the case
its measurements cover. `ring_claim` CL55 was the same shape — an allocation
claim verified over one file while stated over seven crates, with the ungated
set (the empty case again) reading zero.

**Cost:** reachable as a methodology cost rather than a runtime one. Nothing
here is incorrect. What was missing was any measurement whose input resembles a
running ring, and the crate's most carefully-reasoned case is the one that would
have hidden such a measurement if one had been added carelessly. That gap is now
filled; the trap it describes is not, because nothing stops the next such
measurement from being written against the empty case again.

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| pitfall | [001](001_commit_available_does_not_call_commit.md) | the other cost an idle poll pays |
| non_functional_requirement | [001](../non_functional_requirement/001_what_the_read_path_costs.md) | the measurement CN46 explains the absence of |
| integration | [002](../integration/002_eight_methods_and_the_one_that_is_called.md) | `frontier`, the one `Barrier` method this crate calls |
| algorithm | [001](../algorithm/001_position_frontier_pending.md) | the `map_or( 0, … )` that implements the empty default |
| invariant | [001](../invariant/001_never_reads_past_what_was_published.md) | why zero is the safe default for a consumer |

### Sources

| What | Where |
|------|-------|
| The argument | `ring_barrier/src/lib.rs`, module doc, "Why the empty set has no frontier" |
| The consumer's default | `ring_consume/src/lib.rs:342` |
| The producer's opposite default | `ring_gating/src/lib.rs:224` |
| The test carrying it through | `ring_consume/tests/consume_test.rs:143-154` |
| The two states kept distinct | `ring_gating/tests/gating_test.rs:185-207` |

### Tests

| Claim | Verified by |
|-------|-------------|
| Empty barrier yields nothing available | `consume_test.rs:152` |
| Empty gating set yields full capacity | `ring_gating/src/lib.rs:224` and its own suite |
| The asymmetry is argued, not incidental | the module doc section quoted above |
| The empty case allocated zero, and still does | `no_read_of_the_available_range_allocates`, its last two rows |
| A barrier with a dependency allocates zero now, and did not then | the same test's first four rows, against the frozen probe above |
