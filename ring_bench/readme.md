# ring_bench

Comparative write-path measurements: mutex, ring, and thread-local staging.

Depends on [`ring_factory`](../ring_factory/readme.md), [`ring_tls`](../ring_tls/readme.md), [`ring_flush`](../ring_flush/readme.md), [`ring_stats`](../ring_stats/readme.md), [`ring_spsc`](../ring_spsc/readme.md), [`ring_mpsc`](../ring_mpsc/readme.md), [`ring_core`](../ring_core/readme.md), [`ring_slot`](../ring_slot/readme.md), [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list, and [`../readme.md`](../readme.md) describes the family as a
whole.

Originally scoped for six dependency edges. Three more were added, each because
a signature had to name its types. This is the third time it has happened in
the family, and the first where the missing name sits on the export Contract's
own argument list.
→ [`docs/integration/001`](docs/integration/001_declared_edges_and_the_three_that_were_missing.md).

## What it does

The last crate of the family, and the only one whose output is a *number*
rather than a type. It runs one workload against every candidate write path,
and the comparison itself is the deliverable.

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
| `verb/` | Crate-scoped test/lint/build; see the workspace [verb/readme.md](../verb/readme.md) |
| `Workload` | One description of capacity, producers, records, batch, accumulator cells, and `Set`/`Delta` semantics, run identically against every candidate |
| `Candidate` | The five (six with `crossbeam`) write paths under comparison |
| `Outcome` | What one candidate did with one workload: offered, reported, received, timed, and folded into an accumulator table |
| `Comparison` | Every candidate against one workload, plus the ones the producer count excluded |

The crate measures every candidate under both accumulator semantics: `Set`
(overwrite, the harness's original axis) and `Delta` (commutative sum).

## Three findings, each established by measurement

Each of these came from running the harness and reading a number that
contradicted the expectation. None was visible from the task text that
scoped this crate, and two of the three overturn something a document
elsewhere in the family asserts.

### `Ok` is not evidence a record was kept

`OverflowPolicy::default()` is `DropNewest`, so the family's own documented
idiom, a bare `RingConfig::new( n )`, produces a ring on which `try_push`
returns `Ok` for a record it discarded. The first working version of this
harness counted those `Ok`s and reported `contract_ring` as **lossless at 256
records in a 16-slot ring**. It would also have reported it as *fast*, and
correctly, because discarding is the cheapest thing a queue can do.

So [`Outcome::received`] is drained from the ring and is the only count treated
as truth. `reported` keeps what the API claimed, and the crate publishes the gap
as `silently_discarded` instead of hiding it. **The trap corrupts a verdict, not
a record**, and the verdict is what this harness exists to produce.
→ [`docs/pitfall/003`](docs/pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md).

### The Contract door caps at one producer a structure that has no cap

`ring_factory::build` returns a `ring_handle::Split`, whose `Ends::split` yields
exactly one producer and offers no way to ask for a second.
`ring_core::Producer::try_clone` is the operation that would, and `ring_handle`
deliberately does not re-expose it. So `ContractRing` and `OffTheShelf` are both
capped at one producer, and neither cap belongs to the data structure behind it.
`ring_mpsc`'s ring and crossbeam's `ArrayQueue` are both multi-producer.

The candidates must be compared under the same producer counts, and through one
door they cannot be. `DirectMpsc` exists to make the gap measurable.
→ [`docs/pitfall/001`](docs/pitfall/001_the_door_caps_what_the_structure_does_not.md).

### Two crates on the same five-name export Contract do not compose

`ring_factory::build` hands back a `ring_handle::Split`. `ring_flush::Flusher`
needs a `ring_core::Producer`. Nothing on the Contract converts one into the
other, and `ring_flush` re-exports neither the type nor a way to build one. So
`TlsOverRing` builds its ring through `ring_core` directly, below the door the
Contract names. → [`docs/integration/001`](docs/integration/001_declared_edges_and_the_three_that_were_missing.md).

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
→ [`docs/decisions/002`](docs/decisions/002_no_test_asserts_an_ordering.md).

| File | Responsibility |
|------|-----------------|
| `docs/` | Scope, related crates, and the decisions register; see [docs/readme.md](docs/readme.md) |
| `examples/comparison.rs` | The comparison printed for a human: `cargo run -p ring_bench --all-features --example comparison` |
| `src/lib.rs` | `Workload`, `Candidate`, `Outcome`, `Comparison`, and the six runners |
| `tests/` | The behavioural suite and the manual plan; see [tests/manual/readme.md](tests/manual/readme.md) |
