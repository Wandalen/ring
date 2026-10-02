# `ring_handle` stays a separate narrowing layer over `ring_core`'s ends

Status: Accepted

## Context

`ring_core` already partitions the ring's capabilities. `ring_core::Producer` cannot drain and
`ring_core::Consumer` cannot publish, so the split this crate was first specified to provide exists one crate down.
What `ring_handle` adds is four narrowings, listed in [`src/lib.rs`](../../src/lib.rs): the ring is taken by value,
`try_clone` is withheld, `Consumer::drain` is bounded at call time, and nothing reaches the backend.

`ring_handle` is one of the five crates on the family's public contract (`ring_types`, `ring_handle`, `ring_tls`,
`ring_flush`, `ring_factory`). `ring_core` is not. The question was whether four narrowings justify a crate on that
contract, or whether the handles belong in `ring_core`.

Two facts bear on it:

- `ring_factory`, the constructor real callers use, returns `ring_handle::Split` from `Factory::build` and
  `Factory::build_crossbeam`, and names no `ring_core` end.
- No code outside `ring_core` calls `ring_core::Producer::try_clone`. No caller has asked for the wider ends.

## Decision

Keep the three-way split. `ring_core` owns backend dispatch and stays internal. `ring_handle` holds the public
handle types. `ring_factory` hands them out.

Backend dispatch and the public contract change for different reasons, and this keeps them in different crates. It
is also the option that leaves the others open. Merging or moving the ends later remains possible, while moving the
ends out now would change every crate that names them.

## Alternatives considered

- **Merge into `ring_core`.** Move the narrowings down and delete this crate. `ring_core` would then join the
  public contract and own both backend dispatch and the contract, which change for different reasons.
- **Move the ends out of `ring_core`.** `ring_core` keeps `Ring` and dispatch, and the handle types live only here.
  This is the cleanest separation and the most disruptive. Every crate that names `ring_core::Producer` or
  `ring_core::Consumer` changes, among them `ring_debug`, `ring_flush`, `ring_poll`, `ring_shutdown` and
  `ring_testkit`.

## Consequences

- One concept has two type names, and the layer's methods are one-line forwards.
- A factory-built `ring_handle::Producer` feeds neither `ring_flush::Flusher::new`, which is on the public
  contract, nor `ring_shutdown::Shutdown::guard`. Both take `ring_core::Producer`, and no conversion exists. A
  caller who needs either takes `ring_core`'s ends and gives up all four narrowings. `ring_bench` declares
  `ring_core` for this reason. This is the population wanting `ring_core`'s ends that the decision assumed did not
  exist, and a trait for "a producer that can `try_push`" is the missing piece (see
  [002](002_handles_have_no_is_closed.md)).
- Revisit when a caller needs a second producer, such as an MPSC fan-in through `try_clone`. `ring_bench` records
  the same trigger for its producer ceiling in
  [its benchmark-candidates decision](../../../ring_bench/docs/decisions/001_the_in_house_ring_is_three_candidates.md).
