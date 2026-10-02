# `free_capacity` keeps one `usize` signature on every backend, binding at SPSC and advisory elsewhere

Status: Accepted

## Context

`ring_core::Ring` puts three backends behind one value-shaped API: `ring_spsc`, `ring_mpsc`, and
`crossbeam_queue::ArrayQueue` behind the `crossbeam` feature. The point is that a caller can swap backends without
editing call sites.

`ring_core::Producer::free_capacity` cannot mean the same thing on all three. At SPSC a reported `n` is binding.
Nothing else can take the room, so `n` pushes will succeed. At MPSC and crossbeam another producer can take the room
between the read and the push. `Producer::is_full`, `Consumer::len` and `Consumer::is_empty` carry the same split.

A caller can learn which contract applies from `Producer::try_clone`, which returns `None` at SPSC and `Some`
elsewhere, or from `Ring::backend`. No library code in the family asks. Outside `ring_core`, no `src/` file calls
`Ring::backend`, matches on `Backend`, or calls `try_clone`. The production callers of `free_capacity` and `is_full`
in `ring_debug`, `ring_flush`, `ring_handle` and `ring_shutdown` use the number without discriminating first. They
forward it, report it, or treat it as a hint and let the push decide. `ring_flush::Flusher` also relies on its own
rule that it is its ring's only producer.

## Decision

- `free_capacity`, `is_full`, `Consumer::len` and `Consumer::is_empty` keep one plain signature on every backend.
  Each method's documentation states the split and names the authority, `Producer::try_push` for the producer and
  `Consumer::try_recv` for the consumer.
- `Producer::try_clone` is the discriminator, and there is no cardinality type. `if let Some(p) = producer.try_clone()`
  makes the check and the action one step, so they cannot disagree. `Producer` is not `Clone`, because
  `Clone::clone` cannot refuse.
- `Some` from `try_clone` means "more than one producer is allowed", never a bound. `RingConfig::with_producers(4)`
  selects the MPSC backend, and a fifth clone still succeeds.
- `try_clone` stays although no library uses it to discriminate. It is the only way to get a second producer on an
  MPSC ring, and `four_threads_publishing_through_clones_lose_nothing` and `cloned_producers_share_one_ring` depend
  on it.

## Alternatives considered

- **Two methods, `free_capacity_binding` and `free_capacity_advisory`.** Breaks the uniform API. A caller swapping
  backends would edit call sites, which is what `ring_core` exists to prevent.
- **Return a wrapper type that carries the reading.** Every caller unwraps it, and every production caller wants a
  plain `usize` to compare.
- **Advisory everywhere by contract.** Free, but it gives up a real guarantee that `ring_spsc` provides and a caller
  could rely on.
- **A `Cardinality` enum reported by a method.** The caller checks, then acts in a separate step, and the two can
  drift apart.
- **Cardinality in the type, as `Ring<T, Single>` and `Ring<T, Multi>`.** Answers the question at compile time, and
  ends the single type over all backends that the crate exists to provide.

## Consequences

- Code written against the binding reading breaks when handed a multi-producer ring, with no signature change and no
  compiler error. The only defence is the prose on each of the four methods.
- The SPSC guarantee has no observed user, and neither does the discrimination API. The evidence that this design is
  needed is thin, and nobody has yet paid its cost.
- The producer count in `RingConfig` picks a backend. Nothing enforces it as a limit on producers.
- Revisit when a dependent branches on `Ring::backend` or `try_clone` before reading `free_capacity`. That proves the
  need is real and may justify a typed answer.
- Revisit when `ring_mpsc` can give a binding `free_capacity`. The split disappears and the question retires.
