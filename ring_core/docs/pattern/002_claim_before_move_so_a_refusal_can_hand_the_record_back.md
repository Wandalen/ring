# Pattern: Claim Before Move, So a Refusal Can Hand the Record Back

### Scope

- **Purpose**: Name the pattern that lets `try_push` promise `Result< (), T >` over a backend whose own API cannot return the record, and state the condition under which it is needed.
- **Responsibility**: The problem, the two rules, the applicability condition, and the cost.
- **In Scope**: The MPSC arm of `Producer::try_push`, stated generally enough to reuse.
- **Out of Scope**: The overall dispatch (→ [`../algorithm/001`](../algorithm/001_backend_dispatch_and_the_refusal_seam.md)); the uniform-surface pattern this one sits inside (→ [`001`](001_uniform_surface_over_unequal_backends.md)).

### Problem

A wrapper promises to hand a rejected value back — `Result< (), T >` — over an
inner API that takes the value by move and returns only a unit-shaped error. Once
the inner call has consumed the value, the wrapper has nothing to return, and no
signature change can recover it. `ring_mpsc::Producer::push` is exactly this:
it takes `T` and returns `Result< Seq, RingError >`, so on a full ring the record
is dropped inside it.

### The Two Rules

1. **Split the inner operation into a reservation and a write**, so that the
   condition which can fail is tested before the value is touched.
2. **Perform the reservation first and the move second**, so a refusal happens
   while the wrapper still owns the value.

```rust
// src/lib.rs — the MPSC arm, reduced to the pattern
match producer.claim()
{
  Ok( mut reserved ) => { reserved.set( record ); Ok( () ) }
  Err( _ ) => Err( record ),   // the record was never moved
}
```

### Applicability

Needs all three: the inner API consumes by move; it offers a separate
reservation step; and the wrapper's contract promises the value back. Remove any
one and the pattern is unnecessary — the SPSC arm does not use it, because
`ring_spsc::Producer::try_push` already returns the record itself.

### Cost

**One extra round trip on the success path**, and a `debug_assert` where the type
system cannot reach: `reserved.set` returns the displaced record, which must be
`None` because the slot was just claimed. That obligation is checked in debug and
unchecked in release
(→ [`../workaround/002`](../workaround/002_a_debug_assert_where_the_type_system_cannot_reach.md)).

### Consequence

`try_push`'s signature is uniform across three backends whose own refusal shapes
are three different things — the record itself at SPSC, a `RingError` at MPSC,
and `Result< (), T >` at crossbeam. The pattern is what makes the middle one
conform.

### CO45 — The Pattern Costs a Round Trip and Buys a Signature

`ring_mpsc::Producer::push` would move the record in one call. The arm instead
calls `claim()`, then `set()` on the reservation. On the success path that is
two operations where one would do; on the refusal path it is what makes
`Result< (), T >` expressible at all.

Recorded as a cost rather than a defect because the alternative is not a cheaper
implementation — it is a different signature, and the whole crate exists to
avoid having three of those
(→ [`../pattern/001`](../pattern/001_uniform_surface_over_unequal_backends.md)).

**Disposition:** declined — this instance's own text states it is "recorded
as a cost rather than a defect" since the only alternative is a different,
non-uniform signature the crate exists to avoid; no source or doc fix is
implied beyond what is already recorded in
`pattern/002_claim_before_move_so_a_refusal_can_hand_the_record_back.md`.

### CO46 — The Pattern Applies to One Arm and the Instance Reads as General

`ring_spsc::Producer::try_push` already returns the record. Crossbeam's
`ArrayQueue::push` already returns `Result< (), T >`. Only `ring_mpsc` consumes
by move and refuses with a unit-shaped error, and only that arm uses the pattern.

The pattern instance states its applicability condition — inner API consumes by
move, offers a reservation step, wrapper promises the value back — and this
finding is the measurement behind it: one of three arms qualifies. A pattern
documented from a single instance is worth marking as such, so a future reader
does not read it as the crate's general approach to wrapping.
