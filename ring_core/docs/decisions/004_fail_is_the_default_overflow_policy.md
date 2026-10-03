# `Fail` is the default overflow policy

Status: Accepted

## Context

A producer on a ring built from a bare `RingConfig::new(n)` lost records on a full ring and was told every push
succeeded.

`OverflowPolicy::default()` was `DropNewest`, and `RingConfig::new` takes the default. On a full ring
`ring_core::Producer::try_push` gets the record back from the backend, classifies it as
`Resolution::DroppedIncoming`, whose `lost_an_item()` is `true`, then reports success and drops it:

```rust
match refused {
  Ok(()) => Ok(()),
  Err(record) => match would_resolve(self.overflow) {
    Resolution::DroppedIncoming => Ok(()),
    Resolution::EvictedOldest | Resolution::Refused => Err(record),
  },
}
```

`try_push_batch` never sees an `Err` there, so it drains the whole iterator and returns `Ok(len)`, counting every
discarded record. The caller has no second channel. `free_capacity` is advisory at MPSC and crossbeam, and
`ring_core` carries no counter, because it adds no atomic of its own. Only a consumer-side count shows the gap, and
it cannot say which records went.

Two measurements recorded in the source:

- `ring_testkit`, capacity 4, eight records pushed: `DropNewest: accepted=8 refused_full=0 received=[0,1,2,3]
  vanished=4`.
- `ring_bench`, its first working version: `Candidate::ContractRing` reported 256 successes into a 16-slot ring and
  kept 16.

The trap was documented as a pitfall on `try_push` and `try_push_batch`, on `OverflowPolicy` itself, in
`ring_handle`, `ring_testkit`, and in the `ring_shutdown` and `ring_poll` tests. The family's own benchmark fell into
it anyway. The people it reaches are the ones who never chose a policy.

[ADR 003](003_the_batch_push_hands_back_the_refused_record.md) made `Fail` lossless for batch pushes and for the
layers that forward them: `ring_handle`, `ring_shutdown::Guarded`, `ring_poll::push_batch_within` and `Tick`. Before
it, `Fail` destroyed one record per refused batch push, so the default could not have moved there.

## Decision

`#[default]` moves from `OverflowPolicy::DropNewest` to `OverflowPolicy::Fail`. Nothing else changes: no signature,
no backend, and `RingConfig::new` still calls `OverflowPolicy::default()`.

A full default ring hands the record back on every backend, `Err(record)` from `try_push` and `Err((n, record))` from
`try_push_batch`, so `Ok` from a default ring means the record is in the ring. `DropNewest` stays available unchanged
as `RingConfig::new(n)?.with_overflow(OverflowPolicy::DropNewest)`. Its `Ok` for a discard is then the policy the
caller asked for.

The defect is loss without consent, and an explicit `DropNewest` is the consent. The off-the-shelf queue `ring_core`
already depends on, behind its `crossbeam` feature, refuses by default as well:
`crossbeam_queue::ArrayQueue::push(&self, value: T) -> Result<(), T>`. Its eviction is a separate method,
`force_push -> Option<T>`, which hands the evicted value back. Outside this repo, `rtrb::Producer::push(&mut self,
value: T) -> Result<(), PushError<T>>` refuses the same way.

## Alternatives considered

- **Return `Ok(Pushed::Stored | Pushed::Dropped)`.** Honest under every policy, but it is a second signature break to
  the exported `ring_handle` after ADR 003, and `.is_ok()` call sites still read a drop as success.
- **Keep `DropNewest`, add a per-producer drop counter and documentation.** The caller has to remember to read the
  counter, next to a trap that the pitfall sections listed above did not prevent.
- **Documentation only.** Already tried.
- **A required policy argument on `RingConfig::new`.** Contradicts `ring_config`'s
  [ADR 001](../../../ring_config/docs/decisions/001_capacity_is_a_constructor_argument_not_a_setter.md): capacity is
  `RingConfig::new`'s argument and its only rejection, and every other field has an infallible setter.
- **Drop `Default` from `OverflowPolicy`.** A configuration has to have a value before anything happens to it, which
  is why `ring_overflow::Resolution` documents that `OverflowPolicy` derives `Default` and `Resolution` does not.
- **Remove `DropNewest`.** At the `ring_core` level it adds nothing over `Fail` plus a caller-side `let _ =`, but the
  variant is named across `ring_stats`, `ring_overflow`, `ring_bench` and `ring_testkit`.

## Consequences

- `ring_types` is on the family's export contract, and the change is silent at compile time. It moves in the
  fail-safe direction: a caller that `.unwrap()`s a push on a full default ring now panics instead of losing data, and
  a caller that discards the result with `let _ =` behaves as before, except that the caller drops the record instead
  of the ring. No `src` code in the workspace unwraps a push outside doc comments.
- No production path starts losing records or panicking. `ring_flush::Flusher::run` checks `free_capacity` first, so
  a sole producer is never refused, and in the contract-violating race with a second producer the reported count is
  now what landed. `ring_shutdown::Refusal::Full` becomes reachable on a default ring; the variant already existed.
  `ring_poll::Tick::progress`, which counts arrivals only, becomes accurate for default rings.
- `ring_bench`'s comparison changes meaning. Its contract-ring and off-the-shelf candidates now refuse like the
  mutex-queue and direct SPSC and MPSC candidates they are compared with, so on a cramped default workload they
  report what they kept. The harness's drop count for a default workload files under `StatsCounts::failed`, whose doc
  says nothing was lost, instead of `StatsCounts::dropped_newest`; the harness discards what is handed back.
- Mutant B5's detector, `a_dropnewest_ring_reports_successes_it_did_not_keep`, is pinned to an explicit `DropNewest`
  workload, so it keeps detecting the counter mapping it guards.
- Revisit when the first `DropNewest` user needs a loss count without counting on the consumer side. A plain field on
  the `&mut self` `Producer` would provide it without breaking `ring_core`'s no-atomic rule.
- `a_ring_from_a_bare_config_refuses_rather_than_drops_on_every_backend` in `tests/core_test.rs`, and
  `overflow_policy_defaults_to_fail` in `ring_types`, check the decision.
