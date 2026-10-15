# Capacity is `RingConfig::new`'s argument and its only rejection; every other field has an infallible setter

Status: Accepted

## Context

`ring_config::RingConfig` holds five fields: capacity, wait strategy, overflow policy, producer count and batch size.
Callers build one as a single expression, `RingConfig::new(slots)` followed by `with_*` calls.

Each field can be given a bad value, but not every bad value has a right correction. A producer count of zero has
one, because a ring needs at least one producer. A batch wider than the ring has one, because it can never be served,
so the ring's capacity is the most it can mean. A bad capacity has none. Zero cannot be raised to anything the caller
meant, and rounding an arbitrary integer to a power of two silently resizes the ring by up to a factor of two.

`with_batch` reads the capacity to clamp against it. That cross-field read is the only one in the type, and it makes
the order of setter calls matter as soon as capacity can change after construction.

Elsewhere in the family a request wider than the ring is an error. `ring_batch::claim_gated`,
`ring_claim::Claimer::claim` and `ring_gating::GatingSet::check` return `RingError::BatchTooLarge` for
`requested > capacity`, and `ring_slot::BytesSlot::write` returns it for a payload longer than the slot.
`ring_gating`'s module documentation calls it a configuration error, as opposed to `RingError::Full`, which is
back-pressure.

## Decision

- `RingConfig::new(slots) -> Result<Self, RingError>` validates capacity through `ring_types::Capacity::new`. It is the
  crate's only rejection, so the one `?` in a builder chain sits at its head.
- `with_wait`, `with_overflow`, `with_producers` and `with_batch` are infallible `const fn` setters.
  `with_producers` clamps `0` to `1`. `with_batch` clamps into `1..=capacity`. Both correct a value that has an
  obvious right answer instead of forcing a `?` into the middle of the chain.
- There is no `with_capacity`. Capacity is written once, in `new`. That absence is what keeps the four setters
  order-independent.

## Alternatives considered

- **Clamp capacity as well.** Zero has no correction, and rounding to a power of two would hand the caller a ring up to
  twice the size they asked for without telling them.
- **A `with_capacity` setter, with a default capacity in `new`.** It makes `with_batch` depend on call order. Starting
  from capacity 16, `with_capacity(64).with_batch(32)` yields batch 32 while `with_batch(32).with_capacity(64)` yields
  batch 16. Both records satisfy `1..=capacity`, so no invariant check can tell them apart, and the only symptom is a
  ring that batches half as much as the caller meant.
- **Reject a zero producer count or an oversized batch with an error, as the claim-time crates do.** Every chain that
  sets either field would need a `?` in its middle to reject a value that has an obvious correction. At configuration
  time a correction is available, so the setter applies it.

## Consequences

- The family answers `requested > capacity` two ways. The claim-time crates return `RingError::BatchTooLarge` and
  call it a configuration error, while the configuration crate clamps. Nothing in `ring_config` mentions
  `BatchTooLarge`, and nothing in the raising crates says a configured batch was already clamped.
- A clamp is silent. A caller detects one only by comparing what they asked for with `RingConfig::batch` or
  `RingConfig::producers` afterwards.
- The commutation tests do not guard the absence of a capacity setter. `setters_commute` and
  `setters_commute_when_both_clamps_fire` in `ring_config/tests/config_test.rs` name the four existing setters
  explicitly. A fifth setter is not called by either test, and both stay green.
- Revisit when someone proposes a capacity setter. It must either come with a commutation test that includes it, or
  the type must document which call order wins.
- Revisit when a manifest language that describes channels as this record needs to tell its author that a value was
  corrected. A silent clamp would then hide a mistake from the person who wrote it.
