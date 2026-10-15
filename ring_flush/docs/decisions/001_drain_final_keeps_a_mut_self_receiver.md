# `Flusher::drain_final` takes `&mut self`, so once-only is a convention and a refused drain is retried by calling again

Status: Deferred

## Context

`ring_flush::Flusher::drain_final` is the teardown phase. It publishes everything staged and ignores the policy,
and its documentation says the owner calls it once. Its signature, `&mut self` in and `FlushOutcome` out, is the
same as `Flusher::drive` and `Flusher::drive_at_barrier`, for which repeated calls are ordinary use.

A consuming receiver would make once-only structural. The retry path is what stops that being a one-line change. A
final drain can return `FlushOutcome::Rejected` when the ring has no room, the records stay staged, and they must
be retried or they are lost when the `Flusher` drops. Rust has no linear types, and this crate retries nothing on
the caller's behalf. Any consuming signature has to hand the flusher back on rejection.

Reuse after a final drain is tested behaviour:

- `a_driver_still_works_after_a_final_drain` shows that appends after it are accepted and the bound policy still
  fires.
- `a_second_final_drain_is_an_empty_trigger` shows that a second call on an empty buffer reports
  `FlushOutcome::TriggeredEmpty`.
- `a_refused_final_drain_keeps_the_records` shows that a `Rejected` drain keeps the records and that calling again
  retries.

## Decision

```rust
pub fn drain_final(&mut self) -> FlushOutcome
```

Once-only holds by name and documentation. A `Rejected` is retried by calling `drain_final` again. After a final
drain the flusher is an ordinary flusher.

This is the default in force, not a ruling against C. It stays because it is the reversible option. A consuming
signature can later be relaxed back to `&mut self` without breaking a caller, while the reverse breaks every teardown
site. What weighs against C today:

- The consuming signature that keeps the retry path (C below) puts a `match` at every teardown site, so the common
  case pays for the rare one.
- C would delete the reuse-after-drain behaviour that the tests above pin and that a caller may already rely on.
- No caller has yet made a second final drain by accident, so the guarantee C buys answers a failure nobody has
  seen.

## Alternatives considered

- **B: `fn drain_final(self) -> FlushOutcome`.** Once-only becomes structural, but a `Rejected` consumes the
  flusher and loses the staged records it reports on.
- **C: `fn drain_final(self) -> Result<Drained, (Self, FlushOutcome)>`.** Once-only becomes structural, and the
  error arm returns the flusher for a retry. Every teardown site grows a `match`, and the tested reuse after a final
  drain goes away. `Drained` would be a new type.

## Consequences

- `drain_final` is documented as once-only, but `&mut self` permits any number of calls, and two tests
  (`a_second_final_drain_is_an_empty_trigger`, `a_refused_final_drain_keeps_the_records`) now pin repeated calls as
  contract. The once-only rule is a convention that can erode.
- A `Flusher` dropped after a `Rejected` loses the staged records without any signal. The only caller outside this
  crate, `ring_bench::run_tls_over_ring`, reads only `Flushed` from its final drain and does not retry a `Rejected`.
- Moving to C later breaks every teardown site and removes tested behaviour, so the switch gets more expensive with
  each new caller.
- Revisit when a real caller makes an accidental second final drain, or loses records by dropping a `Flusher`
  after a `Rejected`.
