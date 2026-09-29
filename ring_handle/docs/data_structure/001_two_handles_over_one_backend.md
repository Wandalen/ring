# Data Structure: Two Handles Over One Backend

### Scope

- **Purpose**: Specify what each handle actually holds — one field and nothing else — and work out the three candidate shapes for that field, since the choice determines `Send`, the split's cost, and whether the crate can keep its no-RMW promise.
- **Responsibility**: The fields, the sharing shape, the operations over them, and the state deliberately not carried.
- **In Scope**: `Producer`'s and `Consumer`'s representation; the shared backend reference.
- **Out of Scope**: The ring's own fields, which are `ring_core`'s and its backends'; the split procedure (→ [Splitting a Ring Into Two Ends](../algorithm/001_splitting_a_ring_into_two_ends.md)).

### Abstract

**Each handle is one field wide.** `Producer` holds a reference to the shared
backend; `Consumer` holds the same. They differ in their type, and in nothing
else — no flag distinguishes them at runtime, because the distinction is the
type itself.

The structure is therefore almost content-free, and the whole design question
is what that single field *is*. Three shapes are viable and they differ in
consequences that reach three other crates.

**No per-handle state of any kind, at this crate's own level.** This is the
structural expression of
[Delegating an Operation to the Backend](../algorithm/002_delegating_to_the_backend.md)'s
constraint: a handle with a field has a field that can drift, and a handle with
a buffer has a `Drop` that can lose data. One level down this stops being true:
the wrapped `ring_core::Producer` carries an `OverflowPolicy` field that
`ring_core::Consumer` does not, which is real per-handle state and is why
`Producer` measures eight bytes wider than `Consumer` below (→ HD11).

### Structure

```text
Producer< T >                    Consumer< T >
┌─────────────────┐              ┌─────────────────┐
│ backend: <shape>│──────┐  ┌────│ backend: <shape>│
└─────────────────┘      ▼  ▼    └─────────────────┘
                    ┌──────────────┐
                    │  ring_core   │   one ring, two referents
                    │   backend    │
                    └──────────────┘
```

| Field | Type | Held by | Purpose |
|-------|------|---------|---------|
| `backend` | one of the three shapes below | Both handles | The only route from a handle to the ring |
| — | — | — | **No other field exists on either type** |

#### The three candidate shapes for `backend`

| # | Shape | Handle size | `Send` requires | Cost at split | Cost per operation | Ring lifetime |
|---|-------|------------|-----------------|---------------|--------------------|---------------|
| D1 | `Arc< Backend >` | 8 bytes | `Backend: Send + Sync` | One atomic increment per handle | One pointer deref | Ends when both handles drop |
| D2 | `&'a Backend` | 8 bytes | `Backend: Sync` | None | One pointer deref | Bounded by `'a`; the ring must outlive both |
| D3 | `NonNull< Backend >` + an ownership token | 8–16 bytes | Unsafe assertion | None | One pointer deref | Manual — the crate asserts it |

**D1 is the default and its cost is at the boundaries, not in the path.** The
atomic refcount touch happens at clone and at drop, not per publish, so it does
not violate [`ring_spsc`'s no-RMW invariant](../../../ring_spsc/docs/invariant/002_no_lock_in_the_path.md)
— which governs the publish path, not construction. **The distinction matters
and is easy to lose:** a handle that cloned itself per operation would perform
an RMW per publish while every signature stayed the same.

**D2 removes the atomic entirely and imposes a lifetime on the exported
surface.** That is the trade in full: `Producer<'a, T>` is a public type with a
lifetime parameter, and every consumer that stores one in a struct now carries
that lifetime too. For a crate on the five-name export list
(→ [On the Export Surface](../integration/002_on_the_export_surface.md)) this is
a substantial ergonomic tax paid by every user, in exchange for an atomic
operation that occurs twice per ring.

**D3 buys D2's cost profile without D2's lifetime, and pays for it in `unsafe`.**
It is listed because it is what a hand-rolled version of this crate usually
becomes, not because it is recommended — the safety obligation ("the ring
outlives both handles") is exactly the obligation D2 asks the compiler to check.

#### Settled: D2, and the other two were never reachable

The table above was written as a three-way trade. Implementation reduced it to
one option, for a reason no amount of weighing the costs would have surfaced:

| # | Verdict | What rules it out |
|---|---|---|
| D1 | **Impossible** | `ring_core`'s operations take `&mut self`. An `Arc< Backend >` yields `&Backend`, so the handles could not call them at all. D1 is not expensive here — it does not typecheck |
| D2 | **Selected** | The only shape that gives `&mut` access to one backend from two disjoint handles, which is exactly what `Ring::ends` already produces |
| D3 | **Forbidden** | The `NonNull` deref is `unsafe`, and gate `g6_unsafe.sh` confines `unsafe` to `ring_atomic`, `ring_store`, `ring_slot`, `ring_align`. This crate is not on that list |

**So the lifetime parameter on the exported type is a consequence, not a
choice**, and the ergonomic tax the D2 paragraph above prices out is paid
regardless. The paragraph is kept because the tax is real and a reader budgeting
for it should see the reasoning; what changed is that there was never an
alternative to buy instead.

**The handle-size column was also wrong, in a way worth keeping visible.** All
three rows say 8 bytes — one pointer. Measured:

| Type | Size | Why |
|---|---|---|
| `Producer< '_, u32 >` | **24 bytes** | An enum discriminant selecting the backend, the reference, and an `OverflowPolicy` |
| `Consumer< '_, u32 >` | **16 bytes** | Discriminant and reference; no policy — refusal is the producer's concern |
| `Drain< '_, '_, u32 >` | 16 bytes | A `&mut Consumer` and a `usize` bound |
| `Split< u32 >` | 384 bytes | It owns the whole `Ring`, which is the point of taking it by value |

The estimate came from imagining a hand-rolled handle rather than measuring the
one `ring_core` actually has. `tests/handle_test.rs`'s
`the_wrapper_costs_nothing` asserts the property that survives — **this crate's
handle is exactly the size of the one it wraps** — which is the real claim
("the newtype adds no field") stated so that it stays true when `ring_core`'s
own layout changes.

#### State deliberately absent

| Absent | Why | Where it lives instead |
|--------|-----|------------------------|
| A closed flag | A second copy can disagree with the authoritative one | [`ring_shutdown`](../../../ring_shutdown/readme.md) |
| Counters | An RMW on the hot path | [`ring_stats`](../../../ring_stats/readme.md) |
| A local batch buffer | Makes `Drop` lossy and visibility depend on ownership | [`ring_batch`](../../../ring_batch/readme.md) |
| A capability flag | The type is the capability | — nowhere; it is the design |
| A thread id | Would make the cardinality invariant checkable — and only at runtime, only sometimes | Enforced structurally instead (→ [Capability Follows the Handle](../invariant/001_capability_follows_the_handle.md)) |

**The last row is the interesting refusal.** Storing the creating thread's id
and asserting on it would catch
[`ring_spsc`](../../../ring_spsc/docs/invariant/001_exactly_one_producer_one_consumer.md)'s
sequential-handoff violation, which the type system does not. It is refused
because it is a `debug_assert`-grade check that costs a field and a comparison
on every operation in release builds where it does nothing — but the refusal is
a trade, not a free win, and it leaves that violation undetected.

### Operations

| Operation | Complexity | Touches | Notes |
|-----------|-----------|---------|-------|
| Split a ring into a pair | O(1) | Allocates once under D1; nothing under D2/D3 | → [Splitting a Ring Into Two Ends](../algorithm/001_splitting_a_ring_into_two_ends.md) |
| Publish through `Producer` | O(1) + backend | One pointer deref, then the backend | Adds nothing measurable |
| Drain through `Consumer` | O(1) + backend | Same | Same |
| Move a handle to another thread | O(1) | Nothing — it is a move | The property this crate's row asserts |
| Drop one handle | O(1) | Refcount decrement under D1; nothing under D2/D3 | The ring survives while the other handle lives (D1) |
| Drop both | O(1) | Ring dropped under D1 | → [Split, Move and Drop](../lifecycle/001_split_move_and_drop.md) |
| Reach the backend directly | **Not an operation** | — | Its absence is [Capability Follows the Handle](../invariant/001_capability_follows_the_handle.md)'s V4 |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_splitting_a_ring_into_two_ends.md](../algorithm/001_splitting_a_ring_into_two_ends.md) | The procedure that populates the field, and step 2's version of the D1/D2/D3 choice |
| [../algorithm/002_delegating_to_the_backend.md](../algorithm/002_delegating_to_the_backend.md) | Why the structure carries no state beyond the one field |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_on_the_export_surface.md](../integration/002_on_the_export_surface.md) | Why D2's lifetime parameter is expensive here specifically |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | Why the two types differ only in type and not in a field |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_send_without_sync.md](../non_functional_requirement/002_send_without_sync.md) | The `Send` column of the D1/D2/D3 table, stated as a criterion |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_producer.md](../type/001_producer.md) | The publishing half of this structure |
| [../type/002_consumer.md](../type/002_consumer.md) | The draining half |

### Sources

| File | Relationship |
|------|--------------|
| [`../invariant/001_capability_follows_the_handle.md`](../invariant/001_capability_follows_the_handle.md) | "Without sharing a mutable reference" — the property all three shapes must provide |
| [`../integration/002_on_the_export_surface.md`](../integration/002_on_the_export_surface.md) | The five-crate Contract that makes D2's lifetime an external cost |

### Tests

| File | Relationship |
|------|--------------|
| `tests/handle_test.rs` | `the_wrapper_costs_nothing` — each handle is exactly the size of the `ring_core` handle it wraps, so an added field is visible |

### HD11 — "No Per-Handle State of Any Kind" Is True Here and False One Level Down

The field is not called `backend` and it does not hold one:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what the two handles hold --'
for s in Producer Consumer; do
  sed -n "/^pub struct $s/,/^}/p" ring_handle/src/lib.rs
done
echo '  -- and what the thing they hold holds --'
sed -n '/^pub struct Producer/,/^}/p' ring_core/src/lib.rs
printf '  fields: ring_core::Producer=%s ring_core::Consumer=%s\n' \
  "$( sed -n '/^pub struct Producer/,/^}/p' ring_core/src/lib.rs | command grep -cE '^  [a-z_]+ :' )" \
  "$( sed -n '/^pub struct Consumer/,/^}/p' ring_core/src/lib.rs | command grep -cE '^  [a-z_]+ :' )"
```

Live output:

```
  -- what the two handles hold --
pub struct Producer< 'a, T >
{
  inner : ring_core::Producer< 'a, T >,
}
pub struct Consumer< 'a, T >
{
  inner : ring_core::Consumer< 'a, T >,
}
  -- and what the thing they hold holds --
pub struct Producer< 'a, T >
{
  inner : ProducerInner< 'a, T >,
  overflow : OverflowPolicy,
}
  fields: ring_core::Producer=2 ring_core::Consumer=1
```

`inner : ring_core::Producer< 'a, T >` — one indirection short of the backend.
And `ring_core::Producer` carries two fields: a `ProducerInner` and an
`overflow : OverflowPolicy`.

**The overflow policy is per-handle state, and it explains the width asymmetry
this crate reports as a fact without a cause.**
[`item/001`](../item/001_five_nouns_four_of_them_the_same_width.md) measures
`Producer` at 24 bytes and `Consumer` at 16 and treats both as "the same as what
they wrap". They are — and the eight-byte difference between them is the
`OverflowPolicy` the producing side carries and the draining side does not.

The abstract's claim survives literally: *this* crate adds no field. What it
implies — that a handle is content-free — stops being true at the first
indirection, and the structure diagram's "one ring, two referents" is drawn one
level shallower than the code.

**Disposition:** applied — the abstract's headline claim is now scoped to
"this crate's own level" and names the exception one level down: the wrapped
`ring_core::Producer`'s `OverflowPolicy` field, already shown in this
section's own Live output, is real per-handle state and the source of the
`Producer`/`Consumer` size asymmetry `item/001` measures.
Now prints: `overflow : OverflowPolicy`

### HD12 — The Ergonomic Tax D2 Was Priced Against Is Paid by Nobody

D2's cost was "every consumer that stores one in a struct now carries that
lifetime too". Measure the consumers:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- crates importing ring_handle --'
for c in ring_*; do
  n="$( basename "$c" )"
  [ "$n" = ring_handle ] && continue
  command grep -rqE '^ *(pub )?use ring_handle::' "$c/src" 2>/dev/null && echo "  $n"
done
echo '  -- and which ring_handle types they name --'
command grep -rhoE '\b(Split|Producer|Consumer|Ends|Drain)< ' \
  ring_factory/src ring_registry/src | sort | uniq -c
```

Live output:

```
  -- crates importing ring_handle --
  ring_factory
  ring_registry
  -- and which ring_handle types they name --
     12 Split< 
```

Two consumers, and every one of their signatures names `Split< T >` — the
lifetime-free owning value. Neither stores a `Producer` or a `Consumer` at all.

**The trade was scored against a usage pattern that did not happen.** D2's
lifetime is real and it is confined to the borrow between `ends()` and the end
of the scope; the value that crosses crate boundaries, sits in
`ring_registry`'s `HashMap`, and comes back from `ring_factory::build` is
`Split< T >`, which has no lifetime parameter.

`Split` did not exist when this table was written, which is why the cost column
reads as it does. The correction is worth recording because the *conclusion*
(D2) was right for a reason the analysis never reached: not that the tax was
worth paying, but that a later type absorbed it entirely.
