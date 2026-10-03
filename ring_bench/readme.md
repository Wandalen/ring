# ring_bench

Comparative write-path measurements: mutex, ring, and thread-local staging.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

## What it does

The only crate in the family whose output is a *number* rather than a type. It
runs one workload against every candidate write path, and the comparison itself
is the deliverable.

```rust
use ring_bench::{ Comparison, Workload };
use ring_factory::RingConfig;

let workload = Workload::new( RingConfig::new( 1024 ).unwrap() )
  .with_records_per_producer( 256 ).unwrap();

let comparison = Comparison::run( workload );
print!( "{}", comparison.report() );
```

| Type | Responsibility |
|---|---|
| `Workload` | One description of capacity, producers, records, batch, accumulator cells, and `Set`/`Delta` semantics, run identically against every candidate |
| `Candidate` | The write paths under comparison. `OffTheShelf` exists only with the `crossbeam` feature |
| `Outcome` | What one candidate did with one workload: offered, reported, received, timed, and folded into an accumulator table |
| `Comparison` | Every candidate against one workload, plus the ones the producer count excluded |

The crate measures every candidate under both accumulator semantics: `Set`
(overwrite) and `Delta` (commutative sum).

## What measurement established

Each of these came from running the harness and reading a number that
contradicted the expectation.

### `Ok` is not evidence a record was kept

Under `DropNewest`, `try_push` returns `Ok` for a record it discarded. A harness
that counted those `Ok`s would report `contract_ring` as **lossless at 256
records in a 16-slot ring**. It would also report it as *fast*, and correctly,
because discarding is the cheapest thing a queue can do. The default policy,
`Fail`
([`ring_core` ADR 004](../ring_core/docs/decisions/004_fail_is_the_default_overflow_policy.md)),
refuses instead, but a workload can set any policy.

So `Outcome::received` is drained from the ring and is the only count treated
as truth. `reported` keeps what the
API claimed, and the crate publishes the gap as `silently_discarded` instead of
hiding it. **The trap corrupts a verdict, not a record**, and the verdict is
what this harness exists to produce.

### The Contract door caps at one producer a structure that has no cap

`ring_factory::Factory::build` returns a `ring_handle::Split`, whose
`Ends::split` yields exactly one producer and offers no way to ask for a second.
`ring_core::Producer::try_clone` is the operation that would, and `ring_handle`
deliberately does not re-expose it. So `ContractRing` and `OffTheShelf` are both
capped at one producer, and neither cap belongs to the data structure behind it.
`ring_mpsc`'s ring and crossbeam's `ArrayQueue` are both multi-producer.

The candidates must be compared under the same producer counts, and through one
door they cannot be. `DirectMpsc` exists to make the gap measurable.
→ [docs/decisions/001](docs/decisions/001_the_in_house_ring_is_three_candidates.md).

### Two crates on the export Contract do not compose

`ring_factory::Factory::build` hands back a `ring_handle::Split`.
`ring_flush::Flusher` needs a `ring_core::Producer`. Nothing on the Contract
converts one into the other, and `ring_flush` re-exports neither the type nor a
way to build one. So `TlsOverRing` builds its ring through `ring_core` directly,
below the door the Contract names.

## What it never asserts

**No test asserts which candidate is fastest.** An assertion about wall-clock
ordering is a flaky test on any machine with other processes on it, and a flaky
test inside a benchmark harness discredits the measurement the harness exists
to produce. Instead the suite asserts record accounting: offered, reported,
received, dropped. Those counts are deterministic, and they are what makes a
timing number mean anything.

`Comparison::fastest` therefore filters on losslessness *before* comparing
times, and returns `None` when no candidate kept the whole workload. The
quickest way to finish a write phase is to refuse every record.

## Decisions

- [The in-house ring is three benchmark candidates, one per door, each with its own producer ceiling](docs/decisions/001_the_in_house_ring_is_three_candidates.md)

## Known limitations

- No test constrains the timings, so a candidate that becomes ten times slower
  breaks no test. Only a person reading the report would notice.
- `Comparison::report` prints `write ns` for every candidate with no marker on
  the ones that lost records. On a run where every candidate lost records, the
  smallest number in that column belongs to the path that discarded fastest,
  and no test reads the table body.

## Run it

```sh
cargo nextest run -p ring_bench --all-features
cargo test --doc -p ring_bench --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/decisions/` | Architecture decision records |
| `examples/comparison.rs` | The comparison printed for a human: `cargo run -p ring_bench --all-features --example comparison` |
| `src/lib.rs` | `Workload`, `Candidate`, `Outcome`, `Comparison`, and one runner per candidate |
| `tests/` | The behavioural suite and the manual plan; see [tests/manual/readme.md](tests/manual/readme.md) |
