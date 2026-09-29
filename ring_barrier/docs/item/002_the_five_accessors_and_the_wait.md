# Item: The Five Accessors and the Wait

### Scope

- **Purpose**: Give the per-method contract and coverage for the six methods that are not readings, and account for how little of the surface anything actually uses.
- **Responsibility**: `over`, `dependencies`, `len`, `is_empty`, `cursor`, `wait_for` — contract, callers, coverage, and the one `WaitKind` that never reaches the wait.
- **In Scope**: The shape tier and the waiting tier.
- **Out of Scope**: The reading tier — see [`001`](001_the_three_barrier_readings.md).

### The Five Accessors

| | Contract | `const` | Empty barrier | Test call sites |
|--|----------|:-------:|---------------|----------------:|
| `over` | Wraps a slice; never fails, never allocates | ✔ | Accepts `&[]` | 27 |
| `dependencies` | Returns the slice unchanged | ✔ | `&[]` | **1** |
| `len` | The dependency count | ✔ | `0` | 2 |
| `is_empty` | Whether there are none | ✔ | `true` | 1 |
| `cursor( i )` | The `i`th dependency, for that dependency to advance | — | Always `None` | 5 |

All five are total — no panics, no errors, no allocation. `cursor` is the only
one that can decline, and it declines the same way a slice does.

### BR16 — `dependencies()` Has One Caller in the Family

```sh
cd "$(git rev-parse --show-toplevel)"
for f in ring_*/src/*.rs ring_*/tests/*.rs; do
  n=$( grep -vE "^[[:space:]]*//" "$f" | grep -c '\.dependencies()' )
  [ "$n" != 0 ] && printf '%-46s %s\n' "$f" "$n" || true
done
# ring_barrier/tests/barrier_test.rs      1
```

Live output:

```
ring_barrier/tests/barrier_test.rs        1
```

One call, in `a_barrier_exposes_the_same_cursors_it_was_given`, which exists to
assert the accessor. Nothing else in the family — library or test — asks a
barrier for its dependencies back.

That is defensible for a view type: a caller that has a `Barrier` generally
already has the slice it built it from. It does mean the method's only
justification is round-trip fidelity, which is exactly what its one test
asserts, and that the `&'a` on its return
([`api/002`](../api/002_the_borrow_is_the_whole_type.md) § BR8) is a lifetime
nothing currently exploits.

`cursor` is in the same position but with a stated purpose: the doc says "for
that dependency to advance", which makes it the write path a test uses to drive
a barrier's inputs. All five of its call sites are that.

### BR4 — `wait_for` Has No Caller Outside Its Own Crate's Tests

```sh
cd "$(git rev-parse --show-toplevel)"
for f in ring_*/src/*.rs ring_*/tests/*.rs; do
  n=$( grep -vE "^[[:space:]]*//" "$f" | grep -c '\.wait_for(' )
  [ "$n" != 0 ] && printf '%-46s %s\n' "$f" "$n" || true
done
```

Live output:

```
ring_barrier/tests/allocation_test.rs     2
ring_barrier/tests/barrier_test.rs        8
```

Eight calls across two files, both this crate's own. **The family's only
blocking consumer API is unused by the family**, including by `ring_consume`,
the one crate that holds a barrier — it polls `frontier()` and lets its own
caller decide what to do when nothing is available. The second file is
`allocation_test.rs`, which calls the method to weigh it rather than to wait on
anything, so the count of sites that use the wait *as a wait* is still six.

| | |
|--|--|
| Callers in any `src/` | 0 |
| Callers in any other crate's tests | 0 |
| Cost of the method existing | The whole `ring_wait` dependency — [`integration/002`](../integration/002_the_dependency_that_is_not_ring_seqno.md) |
| Cost of not having it | Every consumer writes its own spin loop, and the four `WaitKind`s stop being reachable from a barrier at all |

Not a defect, and not obviously premature either: this crate's role is
specified to block, and a barrier that could only be polled would push the wait
strategy into every caller — which is precisely the duplication `ring_wait`
exists to prevent. Recorded so that the surface's actual exercise is visible.

### BR12 — Three of the Four `WaitKind`s Reach It

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'wait_for(' ring_barrier/tests/barrier_test.rs
```

Live output:

```
    Barrier::over( &[] ).wait_for( Seq::ZERO, 1, WaitKind::None, 4 ),
    empty.wait_for( Seq::ZERO, 0, WaitKind::None, 4 ),
    Barrier::over( &published ).wait_for( Seq::ZERO, 0, WaitKind::None, 4 ),
    Barrier::over( &deps ).wait_for( Seq::ZERO, 1, WaitKind::None, 1 ),
  assert_eq!( barrier.wait_for( Seq::ZERO, 2, WaitKind::Spin, 8 ), Ok( Seq( 2 ) ) );
  assert_eq!( barrier.wait_for( Seq::ZERO, 3, WaitKind::Spin, 8 ), Err( RingError::Empty ) );
  assert!( barrier.wait_for( Seq::ZERO, 1, WaitKind::None, usize::MAX ).is_err() );
      if let Ok( frontier ) = barrier.wait_for( position, 1, WaitKind::Yield, 10_000 )
```

| `WaitKind` | Through `wait_for` | Between attempts | Lines |
|------------|-------------------:|------------------|-------|
| `None` | 3 | returns immediately | 261, 274, 296 |
| `Spin` | 2 | `spin_loop()` hint, `attempt % 8` times | 285, 286 |
| `Yield` | 1 | `thread::yield_now()` | 324 |
| **`Park`** | **0** | **`thread::sleep( 50µs )`** | — |

`Park` is the only variant that sleeps, and it is the only one that never passes
through this crate. Two things follow that are not visible from any other
variant:

1. **Wall-clock cost.** `pause` sleeps 50µs per attempt for `Park`, so
   `wait_for( …, Park, 10_000 )` is a half-second call. Every other variant's
   budget is bounded by CPU speed; this one is bounded by a constant in another
   crate.
2. **Allocation pressure over time.** Every attempt allocates
   ([`non_functional_requirement/001`](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md)),
   so a parked wait performs ten thousand allocations spread across half a
   second — the same count as a spin wait, at a rate that makes it look free in
   a profile.

Neither is a correctness problem, and `ring_wait`'s own tests cover `Park`'s
behaviour directly. What is missing is any assertion that `wait_for` composes
with it, which is the one composition a real blocking consumer would use.

### `over` — What the Constructor Cannot Refuse

| Input | Result | Right? |
|-------|--------|--------|
| A slice of live cursors | A working barrier | ✔ |
| `&[]` | A barrier that reports nothing forever | Deliberate — [`decisions/001`](../decisions/001_zero_for_a_barrier_over_nothing.md) |
| A slice of cursors nobody advances | A barrier that waits forever | **Undetectable here** |

The third is the wiring mistake `ring_publish/tests/handshake_test.rs` asserts
against, one crate over, because no signature at this boundary can distinguish
"the cursor the producer publishes to" from "a cursor"
([`decisions/002`](../decisions/002_a_slice_rather_than_an_aggregate.md)).

### BR39 — The Accessor With No Caller Is the One the Doctest Advertises Most

`dependencies()` returns the slice the barrier was built over. It appears twice
in the entire family and both are this crate's own — one doctest at `:116`, one
test assertion at `barrier_test.rs:404`, and no consumer anywhere. Its doctest
demonstrates a round trip — build a barrier over two cursors, ask for them back, count two —
that no consumer performs.

The method that consumers actually reach for is `cursor( index )`, which
`ring_consume` uses to let a dependency advance its own position. Both are
accessors over the same field; one is documented as a collection and the other
as a lookup, and the family only ever needed the lookup. The collection form
survives because it is what `Barrier` would expose if it were a container, which
it is not — it is a view, and a view's callers want one element at a time.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r "\.dependencies()" --include=*.rs ring_*/ | sed 's|ring/||'
# against the accessor consumers do reach
grep -r "\.cursor(" --include=*.rs ring_consume/src/ ring_publish/src/ | sed 's|ring/||'
```

Live output:

```
ring_barrier/tests/barrier_test.rs:  assert_eq!( barrier.dependencies().len(), 2 );
ring_barrier/src/lib.rs:  /// assert_eq!( Barrier::over( &cursors ).dependencies().len(), 2 );
ring_consume/src/lib.rs:  /// assert_eq!( consumer.cursor().load( Ordering::Acquire ), Seq::ZERO );
ring_consume/src/lib.rs:  /// assert!( core::ptr::eq( consumer.cursor(), &position ), "the very same cursor" );
ring_publish/src/lib.rs:  /// assert_eq!( publisher.cursor().load( Ordering::Acquire ), Seq::ZERO );
```

### Items

| File | Relationship |
|------|--------------|
| [001_the_three_barrier_readings.md](001_the_three_barrier_readings.md) | The other three methods |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_wait_for_asks_twice.md](../algorithm/002_wait_for_asks_twice.md) | `wait_for`'s structure and its three endings |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | The six in the context of nine |
| [../api/002_the_borrow_is_the_whole_type.md](../api/002_the_borrow_is_the_whole_type.md) | The lifetimes `dependencies` and `cursor` return |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_slices_three_provenances.md](../data_structure/002_the_slices_three_provenances.md) | What `over`'s 63 call sites pass it |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_slice_rather_than_an_aggregate.md](../decisions/002_a_slice_rather_than_an_aggregate.md) | `over`'s parameter type |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_dependency_that_is_not_ring_seqno.md](../integration/002_the_dependency_that_is_not_ring_seqno.md) | The dependency `wait_for` alone justifies |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md](../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md) | `over` as the only entry point |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_a_non_blocking_wait_must_look_exactly_once.md](../non_functional_requirement/002_a_non_blocking_wait_must_look_exactly_once.md) | `WaitKind::None`'s contract through `wait_for` |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:88-167` | The five accessors |
| `ring_barrier/src/lib.rs:243-287` | `wait_for` |
| `ring_wait/src/lib.rs` | `pause`, and what each `WaitKind` does between attempts |
| `ring_types/src/policy.rs:21-35` | The four variants |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:419-434` | `dependencies()`'s one call site |
| `tests/barrier_test.rs:265-339` | `wait_for`'s four assertions, and `None`/`Spin` |
| `tests/barrier_test.rs:341-370` | `Yield`, under a real writer |
