# Decision: Zero for a Barrier Over Nothing

### Scope

- **Purpose**: Record why a barrier with no dependencies reports nothing readable, when the crate on the other side of the same fold reports *unbounded* for the same input.
- **Responsibility**: State the decision, the four alternatives, the argument that separates them, and the test that pins it.
- **In Scope**: The default arm of `available`'s `map_or`, and the `Option` in `frontier`'s return type.
- **Out of Scope**: The chain that reaches the `map_or` — see [`algorithm/001`](../algorithm/001_the_frontier_in_two_delegations.md).

### The Decision

```rust
// ring_barrier/src/lib.rs:216-219
pub fn available( &self, from : Seq ) -> u64
{
  self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
}
```

**An empty barrier has no frontier, and a consumer behind an empty barrier may
read nothing.**

### The Two Answers to One Question

The same `&[]`, read from both `Barrier` and `GatingSet`, in one assertion:

```rust
// tests/barrier_test.rs:255-262
let empty = GatingSet::new( cap( 8 ), 0 );

assert_eq!( empty.headroom( Seq( 8 ) ), 8, "a producer with nobody behind it may write" );
assert_eq!(
  Barrier::over( empty.cursors() ).available( Seq::ZERO ),
  0,
  "a consumer with nobody ahead of it may not read"
);
```

| | Empty set means | Resolves to |
|--|-----------------|-------------|
| `ring_gating::headroom` | No consumer constrains the producer | `capacity` — write freely |
| `Barrier::available` | No dependency has produced anything | `0` — nothing to read |

Read as two rules that is an inconsistency. Read as one rule it is not: **both
crates resolve "no constraint from dependencies" to what a dependency-free
participant actually has available.** A producer with nobody reading behind it
owns the whole ring. A consumer with nothing published in front of it owns
nothing. The asymmetry is in the participants, not in the rule.

### Why the Fold Cannot Decide This

`ring_cursor::slowest` returns `Option< Seq >` and refuses to supply an identity
of its own:

```rust
// ring_cursor::slowest in ring_cursor/src/lib.rs
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  cursors.iter().map( | c | c.load( GATING ) ).min()
}
```

That `Option` is exactly what makes the fold shareable. A `slowest` that
returned `Seq::ZERO` for an empty slice would have handed `ring_gating` a
producer permanently at zero headroom — every ungated ring deadlocked at its
first lap. A `slowest` that returned `Seq::MAX` would have handed this crate a
consumer authorized to read the entire sequence space. There is no default that
is right for both callers, so the fold returns the absence and each caller
resolves it.

This is the same argument `ring_gating`'s manual check M5 makes from the other
side: discarding the `Option` there does not merely lose information locally, it
silently adopts *this* crate's answer.

### The Alternatives

| Written | Empty barrier reports | Verdict |
|---------|----------------------|---------|
| `map_or( 0, … )` | Nothing readable | **Chosen** |
| `map_or( u64::MAX, … )` | Everything readable | A consumer reading uninitialized slots |
| `map_or( capacity, … )` | One lap readable | Requires a capacity this crate does not have — [`invariant/002`](../invariant/002_capacity_never_enters_the_arithmetic.md) |
| `unwrap_or( Seq::ZERO )` then subtract | Nothing readable — *by accident* | Right answer, wrong reason; see below |

The fourth is the dangerous one, because it produces the correct number today.
`from.distance_to( Seq::ZERO )` saturates to `0` for every `from >= 0`, so
`unwrap_or( Seq::ZERO )` and `map_or( 0, … )` are observationally identical for
`available`. They differ in what they say `frontier()` is: one reports `None`,
the other would report `Some( Seq::ZERO )` — collapsing *no dependencies* into
*dependencies that have produced nothing yet*. Those are different states with
the same current answer, and a caller distinguishing them (a chained consumer
deciding whether it is misconfigured or merely early) would be told the wrong
one.

The `Option` survives to the public surface for that reason, and
`an_empty_barrier_has_no_frontier_and_nothing_available` asserts both halves —
`frontier() == None` *and* `available( … ) == 0` across four starting positions.

### The Cost Being Accepted

An empty barrier is a silently useless barrier. `wait_for` on one returns
`Err( Empty )` after burning its whole spin budget, and nothing anywhere reports
that the cause is a misconfiguration rather than a slow producer:

```rust
// tests/barrier_test.rs:268-271
assert_eq!(
  Barrier::over( &[] ).wait_for( Seq::ZERO, 1, WaitKind::None, 4 ),
  Err( RingError::Empty )
);
```

Refusing to construct one instead — `over` returning `Option< Self >`, or a
separate `try_over` — would catch the misconfiguration at the boundary. It was
not done, and the reason is that the empty slice is a legitimate *transient*: a
consumer assembling its dependencies incrementally, or a `GatingSet` created
with `consumers = 0` and populated later, both pass through an empty barrier
without being wrong. Making the empty case a construction error would move a
runtime condition into a type-level one that the family's own composition
violates.

That leaves the ambiguity as a real, accepted cost, and it is where
[`pitfall/001`](../pitfall/001_the_two_empty_answers_look_like_a_bug.md) starts.

### BR30 — Three Methods, Three Different Answers for the Empty Barrier

The empty set is resolved three ways inside one crate. `frontier` returns
`None`. `available` maps that to `0`. `wait_for` maps it to
`Err( RingError::Empty )`.

Each is right for its own return type and the module documentation defends the
choice at length against `ring_gating`'s opposite convention. What none of them
says is that a caller moving between the three gets a different shape of answer
for the same condition — an `Option`, a number indistinguishable from *caught
up*, and an error indistinguishable from *budget exhausted* (BR24). The one
input class the crate discusses most is the one where its three readings agree
least.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -A2 "pub fn frontier\|pub fn available" ring_barrier/src/lib.rs | grep -E "slowest|map_or"
grep "ok_or" ring_barrier/src/lib.rs
# the tests name the disagreement without resolving it
grep "fn an_empty_barrier\|fn waiting_on_an_empty" ring_barrier/tests/barrier_test.rs
```

Live output:

```
    ring_cursor::slowest( self.dependencies )
    self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
    self.frontier().ok_or( RingError::Empty )
fn an_empty_barrier_has_no_frontier_and_nothing_available()
fn an_empty_barrier_and_an_empty_gating_set_answer_oppositely()
fn waiting_on_an_empty_barrier_fails_rather_than_hanging()
```

### Decisions

| File | Relationship |
|------|--------------|
| [002_a_slice_rather_than_an_aggregate.md](002_a_slice_rather_than_an_aggregate.md) | The other decision the shared fold forced |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_frontier_in_two_delegations.md](../algorithm/001_the_frontier_in_two_delegations.md) | Where the `map_or` sits in the chain |
| [../algorithm/002_wait_for_asks_twice.md](../algorithm/002_wait_for_asks_twice.md) | The empty case reached through the wait |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | The `Option` in `frontier`'s signature |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_slices_three_provenances.md](../data_structure/002_the_slices_three_provenances.md) | The four `&[]` sites |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_dependency_that_is_not_ring_seqno.md](../integration/002_the_dependency_that_is_not_ring_seqno.md) | Why the fold is shared and the resolution is not |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_capacity_never_enters_the_arithmetic.md](../invariant/002_capacity_never_enters_the_arithmetic.md) | Why the third alternative is not available here |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_two_empty_answers_look_like_a_bug.md](../pitfall/001_the_two_empty_answers_look_like_a_bug.md) | The cost this decision accepts |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:46-54` | The module's own statement of the asymmetry |
| `ring_barrier/src/lib.rs:216-219` | The `map_or` |
| `ring_cursor/src/lib.rs:120-123` | The fold that declines to choose |
| `ring_gating/tests/manual/readme.md` § M5 | The same argument from the producer side |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:234-246` | `None` and `0`, across four starting positions |
| `tests/barrier_test.rs:248-263` | Both crates' answers in one assertion |
| `tests/barrier_test.rs:265-272` | The wait on an empty barrier |
| `tests/barrier_test.rs:223-230` | `admits( ZERO, 0 )` on an empty barrier — the one true answer the empty case gives |
