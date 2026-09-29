# Integration: Three Dependencies and One Dependent

### Scope

- **Purpose**: Place the crate in the family's dependency forest and account for every edge in and out — including the two that exist only in test manifests and the one that should not exist at all.
- **Responsibility**: Give the edges, what each carries, who calls without depending, and the duplicated computation on the one real outgoing edge.
- **In Scope**: `[dependencies]`, `[dev-dependencies]`, and reverse edges.
- **Out of Scope**: Why `ring_wait` is here instead of `ring_seqno` — see [`002`](002_the_dependency_that_is_not_ring_seq.md).

### The Edges

```sh
cd "$(git rev-parse --show-toplevel)"
grep -l 'ring_barrier' ring_*/Cargo.toml
grep -E '^ring_' ring_barrier/Cargo.toml
```

Live output:

```
ring_barrier/Cargo.toml
ring_consume/Cargo.toml
ring_publish/Cargo.toml
ring_types = { path = "../ring_types" }
ring_cursor = { path = "../ring_cursor" }
ring_wait = { path = "../ring_wait" }
ring_gating = { path = "../ring_gating" }
```

| Direction | Crate | Kind | Carries |
|-----------|-------|------|---------|
| out | `ring_types` | dependency | `Seq`, `RingError`, `WaitKind` |
| out | `ring_cursor` | dependency | `PaddedCursor`, `slowest` |
| out | `ring_wait` | dependency | `wait_until` |
| out | `ring_gating` | **dev**-dependency | `GatingSet`, for three tests only — [`workaround/002`](../workaround/002_ring_gating_as_a_dev_dependency.md) |
| in | `ring_consume` | dependency | Holds a `Barrier` as a field |
| in | `ring_publish` | **dev**-dependency | Constructs ten barriers in `handshake_test.rs` |

Three real edges out, one real edge in. Every other relationship in the family
is a test's.

### BR2 — The One Real Dependent Reimplements the Method It Holds

`ring_consume::Consumer` owns a `Barrier` and computes what a barrier computes:

```rust
// ring_consume/src/lib.rs:336-345
pub fn available( &self ) -> Available
{
  let position = self.position();
  let readable = self
    .barrier
    .frontier()
    .map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );

  Available::new( position, readable )
}
```

Unfold the two calls:

| | Expression |
|--|------------|
| `ring_seqno::pending( producer, consumer )` | `consumer.distance_to( producer )` |
| So the closure is | `position.distance_to( frontier )` |
| `Barrier::available( from )` is | `self.frontier().map_or( 0, \| frontier \| from.distance_to( frontier ) )` |
| With `from = position` | **identical, term for term** |

`self.barrier.available( position )` is an exact substitution — same fold, same
default, same subtraction, same type. And it deletes a dependency:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*(///|//!)" ring_consume/src/lib.rs | grep -oE 'ring_seqno::[a-z_]+'
# 95:ring_seqno::pending
```

Live output:

```
ring_seqno::pending
```

**One use, in the one expression that reimplements a method of a crate
`ring_consume` already depends on.** The whole `ring_consume → ring_seqno` edge
exists to spell `Barrier::available` by hand.

| | Today | After substitution |
|--|-------|--------------------|
| `ring_consume` dependencies | `ring_types`, `ring_cursor`, `ring_barrier`, `ring_seqno` | one fewer |
| Copies of the empty-case default | 2 (`0` in each crate) | 1 |
| Behaviour | — | unchanged |

The second row is why this is worth recording rather than filing as tidiness.
The empty-case default is a *decision*
([`decisions/001`](../decisions/001_zero_for_a_barrier_over_nothing.md)),
argued at length in one crate, and silently re-made in another. Nothing checks
the two copies agree — and if `Barrier::available`'s default ever changed, every
barrier-based caller would follow it except this one.

Recorded, not applied: the edit belongs to `ring_consume`, and changing another
crate's dependency graph from this crate's documentation pass would be a change
without its own verification.

### BR17 — The Third Crate That Says "Barrier" Cannot See One

```sh
cd "$(git rev-parse --show-toplevel)"
for f in ring_*/src/*.rs; do
  n=$( grep -vE "^[[:space:]]*(///|//!)" "$f" | grep -c 'Barrier' )
  [ "$n" != 0 ] && printf '%-42s %s\n' "$f" "$n"
done
# ring_barrier/src/lib.rs             2
# ring_consume/src/lib.rs             4
# ring_flush/src/lib.rs               4
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
ring_barrier/src/lib.rs               2
ring_consume/src/lib.rs               4
ring_flush/src/lib.rs                 4
```

The third is `ring_flush`, which declares no edge to this crate and names a
barrier anyway — `FlushPolicy::OnBarrier`, `FlushCause::Barrier`. It is the same
concept, not a homonym: a flush policy that fires when the consumer's barrier
advances. And that crate's own module documentation states the gap:

> **`OnBarrier` cannot observe a barrier**, and adding a dependency edge would
> not change that. What is missing is the barrier *instance* the consumer is
> gated on and a *notification* when it advances, and no arrangement of
> `[dependencies]` lines supplies either.
>
> — `ring_flush/src/lib.rs:26-29`

That is the correct reading and worth keeping visible from this side too. A
`Barrier` is a *view*, 16 bytes, with no identity and no notification channel
([`api/002`](../api/002_the_borrow_is_the_whole_type.md)) — polling it is the
only thing anyone can do with it. So `OnBarrier` is not blocked by a missing
manifest line; it is blocked by this crate not being the kind of thing that can
tell anyone anything. Whatever eventually supplies that will be a new type
somewhere, not an edge to here.

### What Is Not a Dependency

| Absent | Why |
|--------|-----|
| `ring_seqno` | Reached transitively through `ring_cursor`; nothing here needs it directly — [`002`](002_the_dependency_that_is_not_ring_seq.md) |
| `ring_gating` | A barrier is over a slice, from wherever the cursors live — [`decisions/002`](../decisions/002_a_slice_rather_than_an_aggregate.md) |
| `ring_publish` | The direction runs the other way: `ring_publish`'s tests depend on this crate |
| `ring_align` | Reached through `PaddedCursor`; this crate never names an alignment |

The graph stays acyclic in the one place it could plausibly loop: `ring_publish`
holds this crate as a dev-dependency for the handshake test, and this crate
holds `ring_gating` the same way — neither is a real edge, and no real edge
runs back into `ring_barrier` from anything it depends on.

### BR33 — The Crate Has Two Dependents and Only One of Them Links It

Three manifests name `ring_barrier`: its own, `ring_consume`'s, and
`ring_publish`'s. Only `ring_consume` depends on it as a library —
`ring_publish` names it under `[dev-dependencies]`, for the four-operation
handshake test the module documentation cites as the reason the signature is a
slice rather than an aggregate.

So the design decision recorded in this crate's own module doc is justified by a
consumer that is not a consumer: the test that made the aggregate signature
unwireable lives in a crate which, in a release build, does not link this one at
all. The justification is sound and the edge that carries it is invisible to
every dependency graph the family draws.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r "ring_barrier" --include=Cargo.toml ring_*/ | sed 's|ring/||'
# and the test the module doc names as the reason
ls ring_publish/tests/handshake_test.rs
grep -c "Barrier" ring_publish/tests/handshake_test.rs
```

Live output:

```
ring_barrier/Cargo.toml:name = "ring_barrier"
ring_consume/Cargo.toml:ring_barrier = { path = "../ring_barrier" }
ring_publish/Cargo.toml:ring_barrier = { path = "../ring_barrier" }
ring_publish/tests/handshake_test.rs
12
```

### Integrations

| File | Relationship |
|------|--------------|
| [002_the_dependency_that_is_not_ring_seq.md](002_the_dependency_that_is_not_ring_seq.md) | Why the third slot is `ring_wait` |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | The surface `ring_consume` reaches through |
| [../api/002_the_borrow_is_the_whole_type.md](../api/002_the_borrow_is_the_whole_type.md) | Why a barrier can notify nobody |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_slices_three_provenances.md](../data_structure/002_the_slices_three_provenances.md) | `ring_publish` and `ring_gating` as cursor sources |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_zero_for_a_barrier_over_nothing.md](../decisions/001_zero_for_a_barrier_over_nothing.md) | The default `ring_consume` re-makes by hand |
| [../decisions/002_a_slice_rather_than_an_aggregate.md](../decisions/002_a_slice_rather_than_an_aggregate.md) | The edge that was deliberately not added |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_quantity_the_predicate_and_the_wait.md](../pattern/002_the_quantity_the_predicate_and_the_wait.md) | The shape `ring_consume` re-derives the first rung of |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/002_ring_gating_as_a_dev_dependency.md](../workaround/002_ring_gating_as_a_dev_dependency.md) | The test-only edge and its re-promotion hazard |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/Cargo.toml` | Three dependencies and one dev-dependency |
| `ring_consume/Cargo.toml` | The one real reverse edge, and the `ring_seqno` line BR2 is about |
| `ring_consume/src/lib.rs:336-345` | The duplicated computation |
| `ring_seqno/src/lib.rs` | `pending`, and what it unfolds to |
| `ring_flush/src/lib.rs:26-29` | `OnBarrier`'s own account of the gap |

### Tests

| File | Relationship |
|------|--------------|
| `ring_consume/tests/consume_test.rs` | 18 barriers, exercising the reverse edge |
| `ring_publish/tests/handshake_test.rs` | 10 barriers, exercising the dev-only reverse edge |
| `tests/barrier_test.rs:248-263` | The dev-dependency edge, used for what it is for |
