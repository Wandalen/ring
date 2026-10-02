# ring_flush

Flush policies deciding when thread-local staging reaches the ring.

Depends on [`ring_tls`](../ring_tls/readme.md), [`ring_core`](../ring_core/readme.md),
[`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list, and [`../readme.md`](../readme.md) describes the family as a
whole.

## What it is

One of the five crates on the family's export Contract, and the only one that
is a **decision** rather than a thing a consumer holds. `ring_factory` makes
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
buffer's own occupancy, so the counter the design assumed was deleted after a
probe found it equal to `buffer.len()` at every evaluation.

## The three obligations, none enforceable here

1. **Drive it.** Nothing self-fires: no thread, no timer, no callback, no
   `Drop` impl.
2. **Announce barriers truthfully.** `drive_at_barrier` asserts a fact this
   crate cannot check; every consequence of a wrong announcement is the
   caller's.
3. **Retry a rejected final drain.** Rust has no linear types, so a `Flusher`
   can be dropped with records staged and they are gone.

The third is the one that loses data when neglected.

## What the implementation settled

| Question the docs left open | Settled as |
|---|---|
| `verb/` | Crate-scoped test/lint/build, described in [verb/readme.md](verb/readme.md) |
| The flush log's compilation boundary | An opt-in `FlushLog` the driver owns, with no cargo feature and no `cfg`, so the acceptance criterion holds under default features and a release build that never opts in allocates nothing |
| How a record reaches the buffer | `Flusher::append`, absent from both API instances. Without it a buffer moved into `Flusher::new` is unreachable |
| `ConfigError::AlreadyBound` | Not built. `new` takes the buffer **by value**, so ownership enforces one-policy-per-buffer and no runtime check is reachable |
| `FlushEntry`'s third field | The `FlushOutcome`, not a count. A count cannot tell `Rejected` from `TriggeredEmpty`, and carrying the outcome makes log/outcome agreement true by construction |
| `OnBatch`'s accumulation counter | **Deleted.** It equalled `buffer.len()` at every trigger evaluation, which a probe aimed at something else established; `append` is now one `push` and nothing more |
| The five-step seal/claim/drain/reset/report sequence | **Three steps.** Checking the ring's capacity *before* touching the buffer makes the rejection path unable to reach the drain, which removes the sealed-and-unreset state two instances were written to describe |
| `drain_all` vs `drain_final` | Not the same operation. `drain_all` is consumer-side (ring → `Vec`), `drain_final` producer-side (buffer → ring); the shared verb was the whole resemblance |
| Whether a driver is reusable after `drain_final` | Yes, and measured rather than left for callers to discover. A second final drain reports `TriggeredEmpty` |

The `ring_tls` API this crate is built on also moved. Before implementation,
`ring_tls` specified `seal`/`drain`/`reset` as three calls, and what was built
was `flush_into`, which fuses claim and drain and empties the buffer whether or
not the records land. That shape cannot satisfy this crate's O3/O4, so
[`TlsBuffer::drain`](../ring_tls/src/lib.rs) was added there. `flush_into`
consequently has no caller in this workstream, and `ring_tls`'s own 19 doc
instances still describe the API that was not built. That crate owes the
documentation debt.

## What is not covered

`OnBarrier` cannot observe a barrier, and no `[dependencies]` line would change
that. What is missing is the barrier *instance* the consumer is gated on and a
notification when it advances. The crate cannot prevent the misconfiguration.
It makes it **observable**, through a `FlushOutcome::NotTriggered` that never
becomes anything else.

## Run it

```sh
cargo nextest run -p ring_flush --all-features   # 34 tests
cargo test --doc -p ring_flush --all-features    # 2 doc tests
GATE_CRATES=ring_flush GATE_FEATURES=176 GATE_STAGE=S6-flush \
  bench_harness/gate/run_all.sh           # the six gates, scoped
```

| File | Responsibility |
|------|-----------------|
| `docs/` | Scope, related crates, and open trade-offs, indexed in [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | The three policies, the driver, and the flush log |
| `tests/flush_test.rs` | This crate's claiming test: M1–M5, each policy positive and negative |
| `tests/append_cost_test.rs` | The append path's cost claims: exact where they are behavioural, an explicit proxy where an allocator would be needed |
| `tests/manual/readme.md` | Readings the suite cannot make, listed in [the plan](tests/manual/readme.md) |
