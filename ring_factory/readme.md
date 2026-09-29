# ring_factory

Ring construction from configuration.

Depends on [`ring_config`](../ring_config/readme.md), [`ring_core`](../ring_core/readme.md), [`ring_handle`](../ring_handle/readme.md), [`ring_registry`](../ring_registry/readme.md), [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

Originally scoped for five dependency edges. Two of them — `ring_stats` and
`ring_tls` — are gone: `RingConfig` has no field that would configure counters
or staging, so there was nothing for a build to do with either, and inventing
surface to justify a manifest line is backwards. Two others were added, because
this crate's own signatures could not be written without naming their types.
→ [`docs/integration/001`](docs/integration/001_declared_edges_and_the_reached_closure.md).

## What it does

One of the five names on the family's export Contract, and the only one that is
a *verb*. `ring_handle` and `ring_tls` are things a consumer holds, `ring_types`
is vocabulary, `ring_flush` is a decision they make. This crate is the door:
the SPSC and MPSC backends, and the registry, are reached *through* it rather
than by importing `ring_spsc`, `ring_mpsc` or `ring_registry` directly.

```rust
use ring_factory::{ Factory, RingConfig };

let cfg = RingConfig::new( 8 )?;
let mut split = Factory.build::< u32 >( cfg )?;

let ( mut producer, mut consumer ) = split.ends().split();
producer.try_push( 7 )?;
```

| Operation | Effect |
|---|---|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| `build` | A config in, the `Split` that owns the ring out |
| `build_named` | Same, but the ring goes into a registry the caller passes |
| `build_crossbeam` | The crossbeam backend's door — a second one, deliberately |
| `RingConfig`, `Registry` | Re-exported, so a Contract-bound consumer names no other crate |

## `build` returns the owner, not the pair

The specified signature was
`build( cfg ) -> Result< HandlePair< S >, BuildError >`, and **it cannot be
written.** Both handles borrow from an `Ends` that borrows from a `Split`; a
struct holding all three is self-referential.
[`docs/data_structure/002`](docs/data_structure/002_the_handle_pair_as_output.md)
scored this shape's cost as *severe* because it saw only two possible owners —
the factory (stateful, forbidden) or caller-supplied storage (a second argument)
— and the third is the return value. The factory allocates the ring, wraps it,
and hands it over. → [`docs/decisions/001`](docs/decisions/001_the_owner_is_the_return_value.md).

## Two of five config fields are observable through it

A built ring's observable behaviour is expected to match every field of the
config it came from. It matches two:

| Field | Observable | Why not |
|---|---|---|
| `capacity` | yes | — |
| `overflow` | yes | — |
| `producers` | **no** | it selects the backend, and `Split` reaches no backend |
| `wait` | **no** | nothing in the closure honours it |
| `batch` | **no** | `ring_core` was implemented without reading it |

`only_two_of_five_config_fields_are_observable_through_the_factory` asserts each
absence rather than noting it, so any of the three reaching the surface later
breaks a test instead of quietly invalidating a paragraph.
`the_backend_does_change_with_producer_count_where_it_can_still_be_seen` is its
control: `producers` genuinely changes the backend, one level down, which is why
its unobservability is encapsulation rather than a bug.

## What it refuses

Two things, and only one of them is its own. `BuildError::NameTaken` is this
crate's. `BuildError::Unsupported( RingError )` relays `ring_core`'s refusal of
`OverflowPolicy::DropOldest` — eviction contradicts the exactly-once delivery
both in-house backends guarantee — and is never re-decided here, so a fourth
backend that accepts the policy needs no change.

`DropOldest` is not unserviceable, only unserviceable *by those backends*:
`build_crossbeam` accepts it, because `ArrayQueue::force_push` evicts. The two
are separate functions rather than one that routes on the policy, since routing
would make the same config produce different rings under different cargo
features. → [`docs/decisions/002`](docs/decisions/002_two_doors_not_one_that_routes.md).

| File | Responsibility |
|------|-----------------|
| `docs/` | Scope, related crates, and the decisions register — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | `Factory`, `BuildError`, and the two Contract re-exports |
| `tests/` | The behavioural suite and the manual plan — see [tests/manual/readme.md](tests/manual/readme.md) |
