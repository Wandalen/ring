# Lifecycle: The Ends Split and Handle Lifetimes

### Scope

- **Purpose**: Specify the borrow chain from a `Ring` to its two handles, and the one thing that chain makes impossible — which is the reason concurrency in this crate looks the way it does.
- **Responsibility**: The three-step path from owner to handles, the lifetime each step imposes, the cross-thread consequence, and the drop obligation at the end.
- **In Scope**: `Ring::ends`, `Ends::split`, `Producer::try_clone`, and the `Ring`'s `Drop`.
- **Out of Scope**: Which backend was selected (→ [`lifecycle/001`](001_construction_and_backend_selection.md)); the enums involved (→ [`data_structure/001`](../data_structure/001_three_way_storage_enum.md)).

### Lifecycle Phases

```
Ring< T >                      owns the storage
  └─ .ends()      &mut self →  Ends< '_, T >
       └─ .split()          →  ( Producer< '_, T >, Consumer< '_, T > )
            └─ .try_clone()  →  Option< Producer< '_, T > >
```

### Phase Transitions

Three steps, each narrowing what is possible:

| Step | Takes | Yields | Fixed at this point |
|---|---|---|---|
| `ends()` | `&mut Ring` | `Ends` | that no second `Ends` can coexist |
| `split()` | `&mut Ends` | one producer, one consumer | the pairing — always exactly one of each |
| `try_clone()` | `&Producer` | maybe another producer | how many producers the backend permits |

**Why the split is a two-step rather than a method on `Ring`.** The `Ends` value
holds the mutable borrow, and the two handles borrow *from it*. Collapsing the
steps would mean returning two values borrowing from a temporary, which does not
outlive the call. The intermediate is not ceremony; it is where the borrow
lives.

### Dependencies

Every step of the chain depends on the one above it *still being alive*, and
that is the whole of the lifetime story — there is no reference counting, no
`Arc`, nothing that outlives its parent:

| This must live | For as long as | Enforced by |
|---|---|---|
| the `Ring` | the `Ends` exists | `ends()` takes `&mut self` |
| the `Ends` | either handle exists | both handles borrow from it |
| the original `Producer` | any clone exists | `try_clone` takes `&self` |

All three are compile-time obligations, so a caller cannot get them wrong at
runtime. What a caller *can* get wrong is the scope they establish them in,
which is the next paragraph.

#### What this chain makes impossible

**The two handles cannot be moved to different threads.** They borrow one
`Ends`, which borrows one `Ring`, and no thread boundary can be crossed while
that chain is live without the ring outliving the threads.

This is not a limitation to work around — it is the correct default, since a
ring whose ends are on different threads must outlive both, and that is a
lifetime the caller has to establish deliberately. The crate's concurrency test
does exactly that: a `Ring` in an outer scope, `ends()`/`split()` inside a
`thread::scope`, and `try_clone` per producer thread.

So **cardinality and thread-placement are separate questions here**, and
`try_clone` answers only the first. The second belongs to the caller's scope
discipline, and the crate deliberately owns none of it.

### Cleanup Requirements

**The handles have no cleanup of their own.** A `Producer`, a `Consumer`, and a
cloned producer are all borrows — dropping one releases a borrow and nothing
else, in any order, and dropping the `Ends` merely returns the ring to a state
where `ends()` can be called again. The one real obligation is the ring's.

When the `Ring` drops, records still in storage must be released exactly once —
not leaked, not freed twice.

Neither half of that is automatic in a composition: this crate does not run the
drop, its backend does, and a composition can still break the property by
handing the same storage to two owners or by leaking one. So it is asserted
rather than assumed, with a heap payload and a drop counter, at every backend:

```
records_left_in_a_dropped_ring_are_released_exactly_once
```

The `TypedSlot` layer makes this less obvious than it sounds at the in-house
backends — a slot may or may not hold a record, and "exactly once" has to hold
for both cases in one ring.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_three_way_storage_enum.md](../data_structure/001_three_way_storage_enum.md) | The four enums this chain walks, and the exclusive-versus-shared borrow at `split()` |

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_construction_and_backend_selection.md](001_construction_and_backend_selection.md) | Where the ring this chain starts from comes from |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_producer_cardinality.md](../type/002_producer_cardinality.md) | What `try_clone` is actually reporting |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `four_threads_publishing_through_clones_lose_nothing` — the scoped-thread shape the chain forces |
| `tests/core_test.rs` | `cloned_producers_share_one_ring` — that a clone is a second handle, not a second ring |
| `tests/core_test.rs` | `records_left_in_a_dropped_ring_are_released_exactly_once` — the drop obligation |

### CO35 — The Two-Step Split Exists for One Backend

`EndsInner`'s MPSC arm holds a `ring_mpsc::Ends` rather than a borrow, because
`ring_mpsc` produces its handles through its own two-step split. The SPSC and
crossbeam arms need no intermediate at all.

So the public shape — `let mut ends = ring.ends(); let ( p, c ) = ends.split();`
— is one backend's requirement imposed on all three, which is the uniform-surface
pattern applied in the direction that costs the caller
(→ [`../pattern/001`](../pattern/001_uniform_surface_over_unequal_backends.md)).
Seven and eight production call sites respectively carry that cost
(→ [`../item/001`](../item/001_thirty_six_items_and_the_four_names_the_family_imports.md)).

### CO36 — The `Ends` Binding Must Outlive Both Handles and Nothing Says So

`ring.ends()` returns a temporary; `split` takes `&'a mut self` from it. Writing
`let ( p, c ) = ring.ends().split();` does not compile, because the `Ends` value
is dropped at the end of the statement while the handles borrow from it.

The fix is one `let`, and every doc example in the crate happens to be written
that way — so the constraint is demonstrated everywhere and stated nowhere. A
caller who hits the borrow error has an example to copy and no sentence
explaining why the intermediate binding is required.
