# Type: Producer Cardinality as a Return Value

### Scope

- **Purpose**: Describe how this crate represents "how many producers may exist" — which is not a type, and that is the decision worth documenting.
- **Responsibility**: The three cardinalities, why `Producer` is not `Clone`, what `try_clone`'s `Option` actually reports, and the one thing the representation cannot express.
- **In Scope**: `Producer::try_clone`, `RingConfig::is_multi_producer`, and the absence of a cardinality type.
- **Out of Scope**: How the extra producer is constructed (→ [`data_structure/001`](../data_structure/001_three_way_storage_enum.md), asymmetry 2); thread placement (→ [`lifecycle/002`](../lifecycle/002_ends_split_and_handle_lifetimes.md)).

### Definition

| Backend | Producers | Reported by |
|---|---|---|
| SPSC | exactly 1 | `try_clone() -> None` |
| MPSC | any number | `try_clone() -> Some(_)`, `ring_mpsc::Producer` being `Copy` |
| crossbeam | any number | `try_clone() -> Some(_)`, copying a shared `&ArrayQueue` |

#### There is no `Cardinality` type, on purpose

The obvious design is an enum — `Single` / `Multi` — reported by a method. This
crate does not have one, and the reason is that **a cardinality type is a fact a
caller must then act on, while `try_clone` is the action itself.**

Consider the two shapes at a call site:

```rust
// with a type: two steps, and the second can contradict the first
if ring.cardinality() == Cardinality::Multi { let p2 = producer.clone(); }

// without: one step, and it cannot contradict anything
if let Some( p2 ) = producer.try_clone() { … }
```

The first is a check followed by an unchecked operation — the classic shape
where the two drift apart. The second cannot drift, because the check *is* the
operation.

**`Producer` is therefore deliberately not `Clone`.** `Clone::clone` cannot
fail, so implementing it would require either panicking at SPSC or silently
handing back a second handle to a ring whose entire safety argument is that
there is one. `try_clone` is the fallible spelling, and the name is chosen to
read as "this may refuse" rather than as a `Clone` variant.

### Validation

`None` means **this backend permits no second producer** — not "not right now",
not "try again". The answer is fixed for the ring's life, because the backend is
(→ [`lifecycle/001`](../lifecycle/001_construction_and_backend_selection.md)).

That permanence is what makes `try_clone` usable as a *discriminator* rather
than only as a constructor. It is the one machine-checkable way to learn whether
`free_capacity`'s binding contract applies — a fact the `free_capacity`
signature cannot express and that
[`pitfall/001`](../pitfall/001_free_capacity_carries_two_contracts.md) exists
because of.

#### What it cannot express

**A bound.** `Some(_)` means "more than one", never "at most four". A caller who
configured `with_producers( 4 )` and then clones five times gets five producers,
and nothing here objects.

That is honest rather than lax: `is_multi_producer()` is a *boolean* on the
config, so four and forty select the same backend. The count is a hint about
workload shape used to pick a ring, not a quota anything enforces. A caller
needing an enforced bound needs a different mechanism, and this crate declines
to imply it has one.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | `try_clone` in the context of the whole publish surface |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_three_way_storage_enum.md](../data_structure/001_three_way_storage_enum.md) | Asymmetry 2 — why the clone is free at two backends and impossible at the third |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_construction_and_backend_selection.md](../lifecycle/001_construction_and_backend_selection.md) | Where the cardinality is fixed |
| [../lifecycle/002_ends_split_and_handle_lifetimes.md](../lifecycle/002_ends_split_and_handle_lifetimes.md) | Why cardinality and thread placement are separate questions |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_free_capacity_carries_two_contracts.md](../pitfall/001_free_capacity_carries_two_contracts.md) | The trap `try_clone` is the discriminator for |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `try_clone_refuses_at_spsc_and_permits_elsewhere` — the three cardinalities |
| `tests/core_test.rs` | `cloned_producers_share_one_ring` — that a clone is a handle, not a ring |
| `tests/core_test.rs` | `four_threads_publishing_through_clones_lose_nothing` — many clones, one ring, nothing lost |

### CO51 — Cardinality Is Answered by a `None` Return Rather Than by a Type

`try_clone` returning `None` at SPSC is the family's only machine-checkable
backend distinction, and it is checkable at runtime. A type-level alternative —
`Ring< T, Single >` and `Ring< T, Multi >` — would make the same distinction at
compile time and would end the uniform surface, because the two would no longer
be one type.

**The runtime answer is the price of the uniform surface**, and it is coherent:
a crate whose purpose is that all three backends share one type cannot also
distinguish them in that type. Recorded so the design reads as a consequence
rather than as an oversight.
