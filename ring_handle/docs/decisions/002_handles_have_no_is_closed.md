# Neither handle has `is_closed()`; close-awareness comes from wrapping a producer in `ring_shutdown::Guarded`

Status: Accepted

## Context

The pre-implementation specification gave both handles an `is_closed()` that reads `ring_shutdown`'s single close
flag rather than a copy of it. [`src/lib.rs`](../../src/lib.rs) says in one paragraph why it is absent. This record
keeps the options and the price.

Reading the flag means depending on `ring_shutdown`, and `ring_shutdown` depends on `ring_wait` for
`ring_shutdown::wait_for_close` and `ring_shutdown::for_space_or_close`, two functions that block a thread outside
the tick. `ring_handle` is on the tick path, where nothing may park. `ring_poll`'s tests assert that no tick-path
crate (`ring_poll`, `ring_handle`, `ring_core`) reaches `ring_wait`, directly or through an intermediate crate. A
`ring_shutdown` edge from this crate would fail the transitive check.

## Decision

Neither `ring_handle::Producer` nor `ring_handle::Consumer` has `is_closed()`. A caller who needs close-awareness
wraps a producer with `ring_shutdown::Shutdown::guard`. The resulting `ring_shutdown::Guarded` checks the flag
before every push and returns the record with the reason when the ring is closed. The handles compose with
`ring_shutdown` instead of depending on it.

## Alternatives considered

- **Add the dependency and widen `ring_poll::PARKING_CRATES`.** The roster is a public constant so that widening it
  is a decision. Widening it to make a convenience method compile is the failure it exists to prevent.
- **Keep a copy of the flag on each handle.** A second copy can disagree with `ring_shutdown`'s single flag.
- **Feature-gate `ring_shutdown`'s two waiting functions.** This is the structurally correct answer, and cargo's
  feature unification defeats it. A workspace build that enables the feature anywhere enables it everywhere, so the
  parking functions become nameable again under exactly the build the checks run.

## Consequences

- `Shutdown::guard` takes a `ring_core::Producer`, not a `ring_handle::Producer`, and no conversion exists in
  either direction. A caller who wants close-awareness gives up all four of this crate's narrowings, including the
  withheld `try_clone`. The missing piece is a trait for "a producer that can `try_push`" that `Guarded` could be
  generic over. `ring_flush::Flusher::new` needs the same trait (see
  [001](001_handles_are_a_narrowing_layer_over_ring_core.md)), so the need is on record more than once, which is
  the point at which inventing the trait stops being speculative.
- The check that keeps this decision safe lives in `ring_poll`'s tests, not here. If this crate's manifest gains
  `ring_shutdown`, `ring_handle`'s own suite stays green. Only
  `the_transitive_reach_is_wider_than_the_manifest_scan_can_see` in `ring_poll` fails, in a run that includes that
  crate.
- Revisit when `ring_shutdown`'s manifest no longer names `ring_wait`, for example because `wait_for_close` and
  `for_space_or_close` move to `ring_barrier`, which depends on `ring_wait` already. `is_closed()` can then be built
  as first specified.
