# ring_flush

Flush policies deciding when thread-local staging reaches the ring.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

## What it is

It is on the family's export Contract, and it is the one Contract crate that is
a **decision** rather than a thing a consumer holds. `ring_factory` makes
things, `ring_handle` and `ring_tls` are things, `ring_types` is vocabulary.
Importing this crate acquires no capability; it accepts three obligations.

`ring_tls` stages records without ever publishing them, deliberately. This
crate supplies the trigger it withheld, so publication happens at a moment
somebody chose rather than at whatever moment a buffer happens to fill.

| Policy | Fires when | Owned by |
|---|---|---|
| `OnFull` | The buffer cannot accept another record | The buffer's capacity |
| `OnBarrier` | The driver is *told* a barrier was reached | The consumer's schedule |
| `OnBatch( n )` | `n` records are staged, read as `buffer.len() >= n` rather than counted | This crate |

The asymmetry follows from ownership. Only `OnBatch` names a condition this
crate owns outright, which is why it is the only variant with a parameter and
the only one that can be misconfigured. It is **not** the only one with state,
because none of them carry state. "Records staged since the last flush" is the
buffer's own occupancy, so no policy keeps a counter.

`Flusher::new` binds a `TlsBuffer`, taken by value, to a producer and one
policy. Records go in through `Flusher::append`, which never publishes. The
caller publishes by calling `drive`, or `drive_at_barrier` when a barrier is
reached, and calls `drain_final` at teardown. `with_log` opts into a
`FlushLog`, and a driver that never calls it allocates nothing for the log.

## The three obligations, none enforceable here

1. **Drive it.** Nothing self-fires: no thread, no timer, no callback, no
   `Drop` impl.
2. **Announce barriers truthfully.** `drive_at_barrier` asserts a fact this
   crate cannot check; every consequence of a wrong announcement is the
   caller's.
3. **Retry a rejected final drain.** Rust has no linear types, so a `Flusher`
   can be dropped with records staged and they are gone.

The third is the one that loses data when neglected.

## Decisions

- [`Flusher::drain_final` takes `&mut self`, so once-only is a convention and a refused drain is retried by calling again](docs/decisions/001_drain_final_keeps_a_mut_self_receiver.md)

## Known limitations

- `OnBarrier` cannot observe a barrier, and no `[dependencies]` line would
  change that. What is missing is the barrier *instance* the consumer is gated
  on and a notification when it advances. The crate cannot prevent the
  misconfiguration. It makes it **observable**, through a
  `FlushOutcome::NotTriggered` that never becomes anything else.
- `ring_flush::FlushPolicy`, `FlushCause`, `FlushOutcome` and `ConfigError` are
  exhaustive by default, not by a recorded choice. A new variant breaks every
  exhaustive `match` outside the crate, and once the first such `match` exists,
  adding `#[non_exhaustive]` is itself a breaking change.
- `ring_flush::Flusher::new` takes a `ring_core::Producer`, which no Contract
  crate hands out. A caller holding a `ring_factory`-built
  `ring_handle::Producer` cannot construct a `Flusher` and must depend on
  `ring_core` directly, as `ring_bench` does. See
  [`ring_handle`'s narrowing-layer decision](../ring_handle/docs/decisions/001_handles_are_a_narrowing_layer_over_ring_core.md).

## Run it

```sh
cargo nextest run -p ring_flush --all-features
cargo test --doc -p ring_flush --all-features
```

| File | Responsibility |
|------|-----------------|
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | The three policies, the driver, and the flush log |
| `tests/flush_test.rs` | This crate's claiming test: each policy fires at its own trigger and at no other, checked against the flush log |
| `tests/append_cost_test.rs` | The append path's cost claims: exact where they are behavioural, an explicit proxy where an allocator would be needed |
| `tests/manual/readme.md` | Readings the suite cannot make, listed in [the plan](tests/manual/readme.md) |
