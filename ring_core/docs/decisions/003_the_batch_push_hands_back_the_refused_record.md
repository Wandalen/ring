# The batch push hands back the record it refused

Status: Accepted

## Context

`ring_core::Producer::try_push_batch` was a loop over the single push that kept only the verdict:

```rust
for record in records.by_ref() {
  if self.try_push(record).is_err() {
    break;
  }
  accepted += 1;
}
```

`Producer::try_push` hands a refused record back as `Err(record)`, and `.is_err()` dropped it. Under
`OverflowPolicy::Fail` every refusal destroyed one record. The returned count left it out, the iterator had already
moved past it, and nothing warned. `ring_handle::Producer::try_push_batch` documented the opposite, that the iterator
is left at the first record that did not fit.

The layers above multiplied it. `ring_poll::push_batch_within` destroyed one more record on every attempt it spent
against a full ring, `ring_poll::Tick` counted those records as `lost` instead of preventing the loss, and
`ring_shutdown::Guarded::try_push_batch` forwarded it unchanged.

Only `Fail` was affected. `DropNewest` never refuses, and `DropOldest` is rejected at construction except on
crossbeam, where it evicts and returns `Ok`. No performance reason for the loss is on record, and `ring_bench` runs
every workload under `DropNewest`, so it never executed the refusal branch.

## Decision

`try_push_batch` returns `Result<usize, (usize, T)>` instead of `usize`, and so does every layer that forwards it,
including the exported `ring_handle::Producer::try_push_batch`.

- `Ok(n)`: nothing was refused, so the iterator ran dry. Under `DropNewest` the result is always `Ok`, and `n`
  includes the records the policy discarded.
- `Err((n, record))`: `n` records went in, then the ring refused `record`. The iterator resumes after it, so `record`
  followed by whatever the iterator still yields is everything the call did not publish, in order.

It is built only on `try_push`'s `Err(record)`, so no backend changes and the behaviour is the same on SPSC, MPSC and
crossbeam. The accepted path is the same loop as before. The refusal path runs at most once per call, allocates
nothing, and takes at most one record past what fits, so a refusal never drains a lazily generated or unbounded
iterator.

`Result` carries `#[must_use]` already. The explicit attribute the old signature had is gone, because clippy's
`double_must_use` rejects it under `-D warnings`. A caller that wants only the count writes `Ok(n) | Err((n, _))`,
which puts the discard where a reviewer can see it.

## Alternatives considered

- **Fix only the documentation.** The loss stays where it is.
- **Reserve a slot before taking a record.** Fails on two backends. An MPSC reservation dropped unwritten publishes
  an empty slot, which happens whenever the iterator turns out to be empty. Crossbeam has no reservation at all.
- **Put the record back through `Peekable::next_if_map`** (stable since 1.94). The record stays inside the wrapper,
  and a temporary wrapper then drops it silently. It would also raise the minimum Rust version and leave the
  budget-loop mutants without a detector.
- **Collect the remainder into the error.** Allocates, and hangs on an unbounded iterator.
- **Return `(usize, Option<T>)`.** A tuple carries no `#[must_use]` of its own, so the warning depends on an
  attribute again, and the shape no longer matches `try_push`.

## Consequences

- `ring_handle::Producer::try_push_batch` changes signature, a breaking change to a crate on the family's export
  contract. The export list itself does not change.
- `Tick::lost` is removed. A loss counter that can no longer be non-zero is a number someone would monitor for
  nothing.
- The mutation gate changes. `s13_tick_lost_subtraction` targets a line that no longer exists and is retired.
  `s7_batch_budget_bound`, `s8_batch_unproductive_break` and `s9_batch_attempt_counter` were detected through the
  records each extra attempt destroyed. A retry now offers the held record again and destroys nothing, so their
  detector becomes a test whose iterator yields one burst per attempt.
- This applies the shape `try_push` and `ring_registry::Registry::register` already use, an error that carries the
  refused value. `ring_tls::TlsBuffer::push` still drops a refused item, for the reasons in
  [its decision](../../../ring_tls/docs/decisions/001_a_refused_push_drops_the_item.md).
- `a_refused_batch_push_drops_no_record_on_every_backend` and
  `a_partial_batch_push_reports_its_count_and_hands_back_the_refused_record` in `tests/core_test.rs` check the
  contract.
