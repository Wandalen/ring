# `CursorPair`'s gating reads are fixed at `Acquire` and published once as `GATING`, not taken as a parameter

Status: Accepted

## Context

`ring_atomic::SeqCell` takes an `Ordering` on every call, because for a bare cell the caller has a real choice and a
default `SeqCst` would distort every benchmark. `ring_cursor::PaddedCursor` keeps that. Its `SeqCell` impl forwards
whatever ordering the caller names.

The readings on `ring_cursor::CursorPair` (`free_slots`, `pending`, `may_claim`) and the fold `ring_cursor::slowest`
answer a different kind of question, such as "may I claim?". The caller acts on the answer by touching a slot. A
`Relaxed` load there lets a producer act on a stale barrier and overwrite a slot the consumer has not finished with.
There is exactly one correct ordering.

Other crates ask the same question of the same cursors. `ring_claim`, `ring_consume`, `ring_mpsc`, `ring_publish` and
`ring_spsc` all read a cursor to decide whether a slot is safe to touch. `ring_batch::claim_gated` asks it too, but it
is generic over `SeqCell`, depends on `ring_atomic` and not on `ring_cursor`, and never handles a `PaddedCursor`.

## Decision

- `CursorPair`'s readings and `slowest` load at `Ordering::Acquire` and take no ordering argument.
- The value is published once as `pub const ring_cursor::GATING`, so a crate that gates on a cursor imports the
  decision instead of restating it. The crates named above, other than `ring_batch`, import it.
- `GATING` lives in `ring_cursor`, beside the padded cursor it is mostly read through.

## Alternatives considered

- **An ordering parameter, as in `fn free_slots(&self, order: Ordering)`.** Offers a choice with exactly one correct
  answer. Every caller writes `Acquire`, and the one that writes `Relaxed` gets a data race the signature allowed.
- **Default to `Acquire` and allow an override.** The same defect, and a call site that relies on the default does not
  show that a choice exists.
- **`SeqCst` throughout.** Correct and slower. A `ring_bench` run over a padded ring would then measure the fence
  rather than the padding.
- **`Ordering::Acquire` written inline at each site.** The same argument made independently in many places, which is
  how a family ends up with one crate relaxed and the rest not.
- **`GATING` in `ring_atomic`.** Arguably the better home. What it governs is any `SeqCell` read used as a barrier,
  which is `ring_atomic`'s concept. `ring_batch` already depends on `ring_atomic`, so it could import the constant
  there and drop its inline loads without a new dependency. Not done because it moves a public constant and touches
  the imports and manifests of the crates that use it, which is a change with its own tests, not a documentation edit.

## Consequences

- `GATING` is a convention, not a type rule. Every crate that imports it reads cursors through `PaddedCursor`'s
  `SeqCell` impl, which accepts `Relaxed`. A one-word edit at any call site compiles, and no test checks which
  ordering a call site uses. The doctest on `GATING` and the tests in `ring_mpsc` and `ring_spsc` assert only that
  `GATING` equals `Ordering::Acquire`.
- The family states the value independently in several places. `ring_batch::claim_gated` writes `Ordering::Acquire`
  inline for both of its gating loads, with its own copy of the argument. `ring_debug` keeps a private `OBSERVE` for
  its diagnostic reads of a `CursorPair`, argued on its own grounds, though it depends on `ring_cursor` and could
  import `GATING`. `ring_mpsc::OBSERVE` is the consumer's read of a slot stamp, paired with `ring_mpsc::PUBLISH`, a
  different relationship from a cursor read that earns its own name. All of them agree on `Acquire`, and nothing
  compares them.
- Revisit when a new site that gates on a cursor picks `Relaxed`. Nothing detects it today, so it would show up in
  review or in production, not in the suite.
- Revisit when `GATING` moves to `ring_atomic`. `ring_batch::claim_gated` should then import it.
- Revisit when the family targets hardware where `Acquire` and `SeqCst` cost the same. The objection to `SeqCst`
  disappears and the choice stops mattering.
