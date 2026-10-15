# ring_gating

Producer gating so it never laps the slowest consumer.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

It is the producer half of the gating mechanism. Given a set of consumer
cursors, it computes how far a producer may advance without landing on a slot
one of them has not finished with. `ring_barrier` is the consumer half and
reads the same minimum from the other side.
[`tests/gating_test.rs`](tests/gating_test.rs) asserts the behaviour, which is
also read by hand against [`tests/manual/readme.md`](tests/manual/readme.md).

The bound is the minimum across the whole set, so one stalled consumer stops the
producer for everyone. That is the correct behaviour, and the reason a stalled
consumer is worth detecting rather than routing around. An *empty* set is not a
consumer at zero. A ring nobody reads has no data anyone can lose, and
collapsing the two would deadlock every ungated ring at its first lap.

`GatingSet::check` returns a `Result` rather than a `bool` because its two
refusals need different handling. `RingError::Full` is back-pressure, and the
caller should retry, because a consumer will move. `RingError::BatchTooLarge`
is a configuration error that no consumer's progress can fix, so a retry loop
must stop.

The crate has no atomic load, no loop, no `unsafe` and no destructor. The fold
over the cursors is `ring_cursor::slowest` and the free-slot arithmetic is
`ring_seqno::free_slots`. What is left here is which question to ask and which
refusal to return, and both are decisions rather than code.

## Decisions

- [`GatingSet` stays a set of consumer cursors, although every shipping ring builds it with one consumer](docs/decisions/001_gating_set_stays_a_set_of_consumers.md)

## Known limitations

- Nothing checks that `ring_gating::GatingSet::check` and
  `ring_claim::Claimer::claim`, which open-codes the same two guards, refuse the
  same inputs the same way. A third refusal added to `check` would diverge
  silently.

## Run it

```sh
cargo nextest run -p ring_gating --all-features
cargo test --doc -p ring_gating --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `gating_test.rs` and the manual plan under `manual/` |
