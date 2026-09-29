# Data Structure: The Three-Way Storage Enum

### Scope

- **Purpose**: Describe the state this crate owns, and account for the fact that its arity is a compile-time property rather than a fixed number.
- **Responsibility**: The four parallel three-way enums, what each variant holds, the two asymmetries between the in-house arms and the crossbeam arm, and the ownership consequences.
- **In Scope**: `Ring< T >`, `Storage< T >`, `Ends`/`EndsInner`, `Producer`/`ProducerInner`, `Consumer`/`ConsumerInner`.
- **Out of Scope**: Why an enum rather than a trait (→ [`algorithm/001`](../algorithm/001_backend_dispatch_and_the_refusal_seam.md)); each backend's own layout (→ its crate).

### Abstract

Four parallel three-way enums — storage, ends, producer, consumer — each wrapped
in a public struct that carries whatever is uniform. The arity is a compile-time
property rather than a fixed number: three variants with the `crossbeam` feature
on, two without, and every asymmetry recorded below follows from the third arm
storing values where the other two store slots.

### Structure

The crate is a public struct wrapping a private enum, four times over. Each
public type carries whatever is uniform, and its `Inner` enum carries whatever
is per-backend:

```rust
pub struct Ring< T > { storage : Storage< T >, overflow : OverflowPolicy }

enum Storage< T >
{
  Spsc( ring_spsc::Ring< TypedSlot< T > > ),
  Mpsc( ring_mpsc::Ring< TypedSlot< T > > ),
  #[ cfg( feature = "crossbeam" ) ]
  Crossbeam( crossbeam_queue::ArrayQueue< T > ),
}
```

Three variants with the feature on, **two without** — `Storage` is not one type
with a conditional arm, it is two different types in two different builds. Every
consequence below follows from that.

`EndsInner`, `ProducerInner`, and `ConsumerInner` repeat the same three-way
split at the borrow level.

#### The policy is uniform; the storage is not

`OverflowPolicy` is stored **beside** the enum, never inside a variant — on
`Ring`, copied into `Ends` and then into each `Producer`. So a backend arm never
has to know which policy is in force, and the policy is resolved in exactly one
shared place after dispatch (→ [`algorithm/001`](../algorithm/001_backend_dispatch_and_the_refusal_seam.md)
step 5).

`Consumer` carries no policy at all — it holds only its `Inner`. Overflow is a
publish-side concept, and giving the consumer a copy would be a second source of
truth for a fact it never reads.

#### Asymmetry 1: slots versus values

| Arm | Element type |
|---|---|
| `Spsc` | `ring_spsc::Ring< TypedSlot< T > >` |
| `Mpsc` | `ring_mpsc::Ring< TypedSlot< T > >` |
| `Crossbeam` | `ArrayQueue< T >` |

**The in-house rings store `TypedSlot< T >`; crossbeam stores `T` bare.** The
two families are built on different premises: the in-house rings preallocate
their storage and a publish *writes into* a slot the producer reserved, so a
record must live in something that exists before it does. `ArrayQueue` owns
values directly and hands them back by move.

This is the structural cause of the drain asymmetry:
`try_recv` on the in-house arms must `TypedSlot::take` the record out of the
slot after the batch yields it, while the crossbeam arm's `pop()` already
returns an owned `T`. It also fixes `ring_slot` as a dependency
(→ [`integration/001`](../integration/001_family_dependency_seam.md)) that a
crossbeam-only build would not otherwise need.

#### Asymmetry 2: exclusive split versus shared borrow

`Ends::split` produces the two handles, and the borrow it hands each one differs
in kind:

| Arm | Producer gets | Consumer gets |
|---|---|---|
| `Spsc` | one half of `ring.split()` | the other half |
| `Mpsc` | one half of `ends.split()` | the other half |
| `Crossbeam` | `&'a ArrayQueue< T >` | **the same** `&'a ArrayQueue< T >` |

The in-house arms partition exclusive access between the two ends — that
partition is what their safety arguments rest on. The crossbeam arm hands both
ends a *shared* reference to one queue, because `ArrayQueue`'s own operations
take `&self` and it is internally synchronised for any number of callers on
either side.

Two consequences follow, and both are visible in the surface:

- `try_clone` can hand out an unbounded number of producers at crossbeam by
  simply copying the shared reference, whereas MPSC must ask its own ring and
  SPSC must refuse (→ [`type/002`](../type/002_producer_cardinality.md)).
- `EndsInner::Spsc` holds `&'a mut ring_spsc::Ring< _ >` directly rather than a
  `ring_spsc::Ends`, because **`ring_spsc` has no `Ends` type** — its `Ring`
  splits directly. `ring_mpsc` does have one. That is a real divergence between
  two sibling crates, absorbed here rather than pushed onto callers, and it is
  the second gap this composition point surfaced (the first being
  `ring_spsc::Batch::get_mut`).

### Operations

Every operation on these four types is one `match` over the private `Inner`,
followed by whatever uniform work applies — after the match, never inside an arm:

| On | Operation | Dispatches to |
|---|---|---|
| `Ring< T >` | `new`, `with_config`, `ends`, `capacity`, `len`, `is_empty` | The `Storage` arm chosen at construction |
| `Ends< '_, T >` | `split` | The matching `EndsInner` arm, yielding a producer/consumer pair |
| `Producer< '_, T >` | `try_push`, `try_push_batch`, `try_clone`, `free_capacity`, `is_full` | `ProducerInner`, then the shared overflow resolution |
| `Consumer< '_, T >` | `try_recv`, `try_recv_batch`, `len`, `is_empty` | `ConsumerInner` — no policy to apply, so no post-match step |

**No operation is defined on a variant.** The enums are private and have no
inherent methods; everything a caller can reach is a method on the public wrapper
that owns the uniform state. That is what keeps the policy resolution in one
place instead of three.

### Ownership and Lifetimes

`Ring< T >` owns the storage outright; `ends()` borrows it mutably and `split()`
divides that borrow. So:

- Neither handle can outlive the ring, enforced by the borrow checker rather
  than a runtime check.
- **The two handles cannot be moved to different threads through `ends()`
  alone** — they borrow one owner. Cross-thread use goes through a shared `Ring`
  and `try_clone`, which is what `four_threads_publishing_through_clones_lose_nothing`
  exercises.
- Records still in storage when the `Ring` drops are dropped exactly once — the
  in-house rings' own `Drop` for two arms, upstream's for the third. Asserted
  rather than assumed: "exactly once" is a property composition can break even
  when both halves are individually correct.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_backend_dispatch_and_the_refusal_seam.md](../algorithm/001_backend_dispatch_and_the_refusal_seam.md) | The `match` over these variants, and the argument against a trait |
| [../algorithm/002_uniform_drain_over_three_shapes.md](../algorithm/002_uniform_drain_over_three_shapes.md) | Asymmetry 1, as it appears on the drain path |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | `ring_slot`, required by asymmetry 1 |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_construction_and_backend_selection.md](../lifecycle/001_construction_and_backend_selection.md) | How a variant is chosen, and when the choice is fixed |
| [../lifecycle/002_ends_split_and_handle_lifetimes.md](../lifecycle/002_ends_split_and_handle_lifetimes.md) | The borrow chain above, as a lifecycle |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_backend.md](../type/001_backend.md) | The public `Backend` enum these private ones are selected by |
| [../type/002_producer_cardinality.md](../type/002_producer_cardinality.md) | Asymmetry 2, as it reaches the surface |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `records_left_in_a_dropped_ring_are_released_exactly_once` — the drop obligation |
| `tests/core_test.rs` | `every_public_type_is_debuggable` — that `Debug` survives the cfg split on all four enums |
| `tests/core_test.rs` | `cloned_producers_share_one_ring`, `four_threads_publishing_through_clones_lose_nothing` — asymmetry 2 |
| `tests/core_test.rs` | `a_heap_payload_round_trips_its_contents` — that asymmetry 1 moves the record rather than copying its bits |

### CO9 — The Third Storage Variant Exists Sixteen Times in the File

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
printf 'cfg gates:      '; grep -c 'cfg( feature = "crossbeam" )' src/lib.rs
printf 'file lines:     '; wc -l < src/lib.rs
printf 'gate spellings: '; grep -oE '#\[ cfg\([^]]*\) \]' src/lib.rs | sort -u | wc -l
```

Live output:

```
cfg gates:      16
file lines:     653
gate spellings: 1
```

One gate per variant declaration and one per match arm, across five enums and
their uses. **A single spelling throughout**, which is what makes the count
meaningful — a second spelling would mean two conditions that could drift apart.

This is the price of the interim backend stated as a number
(→ [`../workaround/001`](../workaround/001_crossbeam_queue_as_interim_backend.md)).
It is also the argument for removing it once `ring_mpsc` is finished: sixteen
gates is sixteen places where the `--all-features` build and the default build
differ.

**Disposition:** declined — this instance's own text names the removal
condition — "once `ring_mpsc` is finished" — and points to
`workaround/001_crossbeam_queue_as_interim_backend.md` as the record of why
the interim backend exists; the fix is deleting that backend on its own
documented condition, not a source or doc change in this crate to make now.

### CO10 — The Storage Enum Owns; the Other Four Borrow

`Storage< T >` has no lifetime; `EndsInner< 'a, T >`, `ProducerInner< 'a, T >`
and `ConsumerInner< 'a, T >` all do. That difference is the whole reason there
are four enums rather than one
(→ [`../data_structure/002`](../data_structure/002_four_parallel_enums_over_one_backend_choice.md)):
the same three-way choice has to be expressed once per ownership shape, and Rust
has no way to abstract over the shape.
