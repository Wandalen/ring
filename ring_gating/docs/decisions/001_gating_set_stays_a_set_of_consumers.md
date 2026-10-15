# `GatingSet` stays a set of consumer cursors, although every shipping ring builds it with one consumer

Status: Accepted

## Context

`ring_gating::GatingSet` holds a `Vec<PaddedCursor>` and a `Capacity`. Every gating read folds the cursors with
`ring_cursor::slowest`, and an empty set means an ungated ring with full headroom.

The plural shape is not exercised in production:

- The only production construction is in `ring_mpsc::Ring`, with one consumer. `ring_mpsc` reads that cursor back
  with `.cursor(0).expect(..)`, so its single-consumer assumption is a run-time check, not a type.
- `ring_spsc` gates the same three fields (a producer cursor, a consumer cursor, a capacity) through
  `ring_cursor::CursorPair`, with two loads and no heap allocation.
- Sets with more than one consumer, and the empty-set rule, appear only in tests and doctests.

At one consumer the plural costs a slice walk over one element on every gating read, a `Vec` held for the set's
life, an `Option` from the fold that is never `None`, and a default arm in `GatingSet::headroom` that never runs.
The fold itself does not allocate.

## Decision

`GatingSet` stays a set, for three reasons:

- It is specified as a set. A ring with several independent consumers must be gated by the slowest of them.
- The fold is shared. `ring_cursor::slowest` also backs `ring_barrier::Barrier`, which `ring_consume::Consumer`
  holds and whose purpose is folding over several upstream cursors. Like this crate's plural case, a `Barrier` is
  so far built only in tests and doctests.
- A multi-consumer ring is the family's named next step. Replacing the type now and rebuilding it then would move
  the cost, not remove it.

Membership and capacity are fixed at construction. `GatingSet` has no `&mut self` method, and
`ring_claim::Claimer::claim` relies on that when it checks the claim width against `GatingSet::capacity` once,
before its retry loop. A future ring whose consumers join or leave while it runs needs a different type.

## Alternatives considered

- **Move `ring_mpsc` to `CursorPair`, as `ring_spsc` does, and delete this crate.** The single-consumer path would
  lose the slice walk and the held `Vec`. `ring_claim::Claimer` borrows the `GatingSet` and would change with it,
  the ungated shape (`consumers = 0`) has no `CursorPair` equivalent, and `ring_barrier` would lose the sibling it
  shares the fold with.
- **Let consumers join or leave a live set.** Removing a cursor removes a bound, so a producer gated by a departing
  consumer is released at once, which is correct only if that consumer has really finished. Removal also renumbers
  the indices `GatingSet::cursor` hands out. Any `&mut self` method would conflict with every live reader of
  `GatingSet::cursors`, which is the normal state of a running ring.

## Consequences

- The production path pays for plurality it does not use, and the empty-set rule and the multi-consumer fold run
  only under tests.
- `GatingSet::cursor` and `GatingSet::cursors` hand any holder of `&GatingSet` a storable `&PaddedCursor` for any
  index. Nothing ties an index to its consumer, so a wrong store, from the producer side or from another consumer,
  raises `slowest` and makes the gate over-report headroom. Closing this needs an index-scoped capability, which is
  an API change across the `ring_claim`, `ring_mpsc` and `ring_barrier` callers.
- Revisit when a multi-consumer ring lands, which makes the plural shape real, or when the family rules out
  multi-consumer rings, at which point `ring_mpsc` moves to `CursorPair` and this crate goes.
