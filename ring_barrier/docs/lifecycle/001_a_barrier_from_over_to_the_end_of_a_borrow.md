# Lifecycle: A Barrier From `over` to the End of a Borrow

### Scope

- **Purpose**: Walk a barrier's whole life and show that it has one state, no transitions, and no end anyone can observe.
- **Responsibility**: Give the construction, the single state, the allocation ledger, and what "release" would mean if there were one.
- **In Scope**: The `Barrier` value itself.
- **Out of Scope**: The sequence the family runs through it — see [`002`](002_a_consumer_draining_behind_a_producer.md).

### One State

```
              over( &[ PaddedCursor ] )                  end of 'a
   (nothing) ───────────────────────────▶  Borrowing  ──────────────▶  (nothing)
                    const, 0 alloc                       no code runs
```

There is no second state. A barrier cannot be armed, closed, invalidated,
exhausted, or reset. Every method is `&self`, none takes `&mut self`, and the
type carries no flag that any of them could flip.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c '&mut self' ring_barrier/src/lib.rs          # 0
grep -c 'impl.*Drop'  ring_barrier/src/lib.rs        # 0
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
  | grep -cE '\bwhile\b|\bloop\b|\bfor\b'                   # 0
grep -c 'unsafe' ring_barrier/src/lib.rs             # 0
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
  | grep -cE 'Ordering::'                                   # 0
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
0
0
0
0
0
```

No mutation, no destructor, no loop, no `unsafe`, and no ordering named — the
crate performs no atomic operation of its own, the same absence `ring_gating`'s
manual check M2 asserts on the other side.

### The Allocation Ledger

| Event | Allocations | Atomic loads |
|-------|------------:|-------------:|
| `Barrier::over( … )` | **0** | 0 |
| Copying a barrier | 0 | 0 |
| Passing it to a thread | 0 | 0 |
| `len()` / `is_empty()` / `dependencies()` / `cursor( i )` | 0 | 0 |
| `frontier()` | 1 | one per dependency |
| `available()` / `admits()` | 1 | one per dependency |
| `wait_for( …, n spins )` | up to n + 1 | n+1 per dependency |
| Going out of scope | **0** | 0 |

Construction is free and destruction does not exist. **All of the crate's cost
is in reading**, which is the exact inverse of `GatingSet`, whose construction
allocates and whose reads are otherwise the same shape:

| | `Barrier` | `GatingSet` |
|--|-----------|-------------|
| Construct | 0 allocations | 1, for `consumers > 0` |
| Read | 1 allocation | 1 allocation |
| Drop | nothing | frees the `Vec` |
| Copy | free | not `Copy` |

### There Is No Release

`Copy` makes this structural rather than a matter of discipline:

```rust
let borrowed : Option< &PaddedCursor > =
{
  let barrier = Barrier::over( &cursors );
  let c = barrier.cursor( 0 );
  drop( barrier );        // warning: calls to `std::mem::drop` with a value that implements `Copy`
  c
};
```

The compiler declines to pretend. `drop( barrier )` is a no-op it warns about,
and the reference obtained through the barrier survives it
([`api/002`](../api/002_the_borrow_is_the_whole_type.md) § BR8) — because that
reference was never the barrier's to hand back.

So a dependency is never told it has been depended on, and never told it has
stopped being. Compare the family's four destructors:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'impl.*Drop for' ring_*/src/*.rs
# ring_spsc:788  Reservation   ring_spsc:1130  Batch
# ring_mpsc:985  Reserved      ring_mpsc:1257  Batch
```

Live output:

```
ring_mpsc/src/lib.rs:impl< S > Drop for Reserved< '_, S >
ring_mpsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Reservation< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
```

Exactly four `impl Drop` in all 33 crates, and every one is a publish-on-drop
guard — a claimed slot that must be published even if the holder panics.
Nothing in the barrier chain has a destructor at any level: not `Barrier`, not
`PaddedCursor`, not `CacheAligned`, not `AtomicSeq`. There is nothing to
publish, because a barrier only ever reads.

### What the Absent End State Costs

| Wanted | Available | Because |
|--------|-----------|---------|
| "This consumer has finished; stop gating on it" | Nothing here | A barrier is a consumer's view of *its* dependencies, not the producer's view of consumers — that is `GatingSet`'s side, and it has no removal either |
| "Notify me when the frontier advances" | Nothing here | A view has no identity to register — [`integration/001`](../integration/001_three_dependencies_and_one_dependent.md) § BR17 |
| "This barrier is stale" | Nothing here | Every read is fresh; there is no cached state to go stale |

The third row is the one the absence actually buys. A barrier with no lifecycle
has no invalidation problem, which is why `frontier()` can be called any number
of times from any number of threads with no coordination at all — and why the
whole crate needs neither `&mut self` nor `unsafe` to be correct under
concurrency.

### BR40 — The Type Has No Destruction Step and That Is Why It Is `Copy`

A `Barrier` is created by `over`, used, and ends when its lexical scope does.
There is no close, no drop impl, no release — the borrow it holds is the whole
of its state, and ending is the borrow checker's business rather than the
crate's.

`Copy` follows from that: a type with no destruction step and no owned resource
can be duplicated freely, and `ring_consume::Consumer` relies on it, storing a
`Barrier<'a>` by value and handing copies back out of `barrier()`. The producer
half of the same feature cannot do this — `GatingSet` owns its cursors, so it is
neither `Copy` nor duplicable, and a consumer wanting one must borrow it. One
derive line is the difference between the two lifecycles.

```sh
cd "$(git rev-parse --show-toplevel)"
grep "impl Drop\|fn close\|fn release" ring_barrier/src/lib.rs \
  || echo '(no destruction step of any kind)'
# control: the identical expression over a crate that has one
grep "impl.*Drop" ring_claim/src/lib.rs
# and the consumer that stores a Barrier by value
grep -A4 "pub struct Consumer" ring_consume/src/lib.rs
```

Live output:

```
(no destruction step of any kind)
pub struct Consumer<'a> {
    cursor: &'a PaddedCursor,
    barrier: Barrier<'a>,
}
```

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_a_consumer_draining_behind_a_producer.md](002_a_consumer_draining_behind_a_producer.md) | The barrier in use, over 512 items |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_borrow_is_the_whole_type.md](../api/002_the_borrow_is_the_whole_type.md) | `Copy`, and the reference that outlives its barrier |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_field_and_a_sixteen_byte_view.md](../data_structure/001_one_field_and_a_sixteen_byte_view.md) | The field that has nothing to release |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_five_accessors_and_the_wait.md](../item/002_the_five_accessors_and_the_wait.md) | `over` as the only entry point |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_frontier_read_allocates_nothing.md](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md) | The one row of the ledger that is not zero |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_borrowed_view_and_the_owned_set.md](../pattern/001_the_borrowed_view_and_the_owned_set.md) | Why a view has no lifecycle to have |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_the_traits_derived_and_the_traits_absent.md](../type/002_the_traits_derived_and_the_traits_absent.md) | The `Copy` that removes the end state |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:80-107` | The derive and the constructor |
| `ring_spsc/src/lib.rs:788`, `:1130` | Two of the family's four destructors |
| `ring_mpsc/src/lib.rs:985`, `:1257` | The other two |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:402-417` | Two barriers over one set, alive at once |
| `tests/barrier_test.rs:341-370` | A barrier used on two threads, by copy |
| `tests/manual/readme.md` § B2 | The check that keeps the field a borrow |
