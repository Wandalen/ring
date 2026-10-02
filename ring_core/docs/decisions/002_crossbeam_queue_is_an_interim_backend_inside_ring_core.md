# `crossbeam-queue`'s `ArrayQueue` is an interim third backend inside `ring_core`, behind the `crossbeam` feature

Status: Accepted

## Context

Both in-house rings, `ring_spsc` and `ring_mpsc`, work. What they lack is the operating history that would let a
consumer bet a shipping channel on them. Getting that history is `ring_bench`'s job, and consumers should not wait for
the benchmark to finish before they can ship.

`crossbeam_queue::ArrayQueue` already has that history. The question is where to absorb it. Its API is
value-shaped: `push(value)`, `pop() -> Option<value>` and `force_push(value)`, with no reservation and no in-place
write. Both in-house rings publish through a slot reservation.

## Decision

- `ring_core` carries `ArrayQueue` as a third backend, gated by the `crossbeam` feature
  (`crossbeam = ["dep:crossbeam-queue"]`) and reached through `ring_core::Ring::new_crossbeam`. `ring_handle`,
  `ring_factory` (`Factory::build_crossbeam`) and `ring_bench` forward the feature.
- `crossbeam-queue` is declared with `default-features = false, features = ["alloc"]`, so it brings no runtime and no
  global state. `ring_core` adds no `unsafe` of its own; `ArrayQueue`'s `unsafe` stays upstream.
- `ring_core`'s API is value-shaped to fit all three backends. `Producer::try_push` takes a `T` and
  `Consumer::try_recv` returns one.
- Only this backend honours `OverflowPolicy::DropOldest`. `ArrayQueue::force_push` evicts the oldest record, which
  the in-house rings cannot do without breaking exactly-once delivery. `Ring::new` rejects the policy with
  `RingError::PolicyUnsupported` and `Ring::new_crossbeam` accepts it. A construction error beats a push that quietly
  behaves as `DropNewest`.
- The purpose is decoupling schedules, not speed. An in-house ring that consumers trust removes the need for this
  backend whether or not it is faster.

## Alternatives considered

- **Consumers depend on `crossbeam-queue` directly and switch later.** Switching becomes a rewrite at every call site
  instead of a feature flag, and each consumer discovers the API mismatch separately at switch time. `ring_bench`
  could not compare backends through one API either, so its numbers would include the call sites' differences.
- **Ship only the in-house rings.** Every consumer waits on the benchmark verdict.

## Consequences

- A caller who needs to build a record in the ring's own memory must reach past `ring_core` to `ring_spsc` or
  `ring_mpsc` and give up the backend swap.
- The crate is two programs. `Backend`, `Storage`, `EndsInner`, `ProducerInner` and `ConsumerInner` each carry a
  `#[cfg(feature = "crossbeam")]` variant, and every match over them carries a gated arm, so the default build and
  the all-features build differ at each of those sites. Both builds need testing.
- The crossbeam arm of `ring_core::Consumer::try_recv_batch` allocates an intermediate `Vec` on every call
  (`collect`, then `extend`). The in-house arms allocate nothing.
- Deleting this backend deletes the family's only implementation of eviction.
- Revisit, and remove the `crossbeam` feature, when all three hold: `ring_bench` has published a verdict on the
  in-house rings against this backend, no consumer's build enables `crossbeam`, and `OverflowPolicy::DropOldest` has
  no user or has been re-specified as something the in-house rings can honour.
- The third condition can outlive the other two. If a consumer needs eviction, removing the backend becomes a feature
  removal rather than a cleanup. Revisit when that consumer appears.
