# ring_factory

Ring construction from configuration.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

## What it does

It is on the family's export Contract, and it is the one Contract crate that is
a *verb*. `ring_handle` and `ring_tls` are things a consumer holds, `ring_types`
is vocabulary, `ring_flush` is a decision they make. This crate is the door. A
consumer reaches the SPSC and MPSC backends, and the registry, *through* it
rather than by importing `ring_spsc`, `ring_mpsc` or `ring_registry` directly.

```rust
use ring_factory::{ Factory, RingConfig };

let cfg = RingConfig::new( 8 )?;
let mut split = Factory.build::< u32 >( cfg )?;

let ( mut producer, mut consumer ) = split.ends().split();
producer.try_push( 7 )?;
```

| Operation | Effect |
|---|---|
| `build` | A config in, the `Split` that owns the ring out |
| `build_named` | Same, but the ring goes into a registry the caller passes |
| `build_crossbeam` | A deliberate second door, for the crossbeam backend |
| `RingConfig`, `Registry` | Re-exported, so a Contract-bound consumer names no other crate |

## `build` returns the owner, not the pair

`build` cannot return the producer and consumer. Both handles borrow from an
`Ends` that borrows from a `Split`, so a struct holding all three is
self-referential. The other two possible owners are both bad: the factory
holding the ring makes it stateful, and caller-supplied storage adds a second
argument. So the owner is the return value. The factory allocates the ring,
wraps it, and hands it over, and the caller splits it when ready.

## Two of five config fields are observable through it

A built ring's observable behaviour is expected to match every field of the
config it came from. It matches two:

| Field | Observable | Why not |
|---|---|---|
| `capacity` | yes | n/a |
| `overflow` | yes | n/a |
| `producers` | **no** | it selects the backend, and `Split` reaches no backend |
| `wait` | **no** | nothing in the closure honours it |
| `batch` | **no** | `ring_core` does not read it |

`only_two_of_five_config_fields_are_observable_through_the_factory` asserts each
absence rather than noting it, so any of the three reaching the public API later
breaks a test instead of quietly invalidating a paragraph.
`the_backend_does_change_with_producer_count_where_it_can_still_be_seen` is its
control. `producers` does change the backend, one level down, which is why its
unobservability is encapsulation rather than a bug.

## What it refuses

Two things, and only one of them is its own. `BuildError::NameTaken` is this
crate's. `BuildError::Unsupported( RingError )` relays `ring_core`'s refusal of
`OverflowPolicy::DropOldest`, because eviction contradicts the exactly-once
delivery both in-house backends guarantee. This crate never re-decides that
refusal, so a new backend that accepts the policy needs no change here.

`DropOldest` is unserviceable only *by those backends*. `build_crossbeam`
accepts it, because `ArrayQueue::force_push` evicts. The two are separate
functions rather than one that routes on the policy, since routing would make
the same config produce different rings under different cargo features.

## Run it

```sh
cargo nextest run -p ring_factory --all-features
cargo test --doc -p ring_factory --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `src/lib.rs` | `Factory`, `BuildError`, and the two Contract re-exports |
| `tests/` | The behavioural suite and the manual plan. See [tests/manual/readme.md](tests/manual/readme.md) |
