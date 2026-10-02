# ring_testkit

Determinism-test fixtures driving scripted claim and drain sequences.

Depends on [`ring_core`](../ring_core/readme.md), [`ring_tls`](../ring_tls/readme.md), [`ring_shutdown`](../ring_shutdown/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list. [`../readme.md`](../readme.md) describes the family as a whole.

The crate has three dependency edges and, unusually for this family, **all
three are used**. `ring_core` is the ring being driven, `ring_tls` is the staging
buffer, `ring_shutdown` is the guard that makes `Step::Close` mean something.
→ [`docs/integration/001`](docs/integration/001_the_three_edges_and_the_one_that_is_missing.md).

## What it does

A `Script` is a list of `Step`s. Run it against a ring and it returns an
`Outcome`: ten counters, lists and a flag describing what the ring did, with no
timing in it, so two runs of one script compare equal.

```rust
// a four-slot ring, and a script that offers it eight records
let config = RingConfig::new( 4 ).expect( "a power of two" )
  .with_overflow( OverflowPolicy::Fail );
let mut ring = Ring::new( &config ).expect( "Fail is an accepted policy" );

let outcome = Script::new( 0 )        // 0 = no staging buffer; this script never stages
  .then( Step::PushMany( 8 ) )
  .then( Step::DrainAll )
  .run( &mut ring );

assert_eq!( outcome.received, vec![ 0, 1, 2, 3 ] );
assert!( outcome.audit().is_ok() );
```

`Script::new`'s argument is the **staging** capacity, not the ring's. The caller
builds the ring however they like and passes it to `run`. That split is why
`ring_config` is a dev-dependency here rather than a real edge.

| Step | Effect |
|---|---|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `Push` / `PushMany` | Mint a record and publish it through the shutdown guard |
| `Recv` / `RecvMany` | Take from the consumer end |
| `Stage` / `StageMany` | Mint into the `TlsBuffer` instead of the ring |
| `Flush` | Move the staged records into the ring, one at a time |
| `Close` / `Reopen` | Flip the guard; a closed ring refuses publications and still permits reads |
| `DrainAll` | Close, then recover everything still held |

## The measurement it is built around

`ring_core::try_push` reports **`Ok` and destroys the record** when the ring is
full under `OverflowPolicy::DropNewest`. The prediction written into a draft of
this crate was that two rings differing only in that policy would show identical
counts and different records. Measured, it is the exact inverse:

```
Fail:       accepted=4 refused_full=4 received=[0, 1, 2, 3] vanished=0
DropNewest: accepted=8 refused_full=0 received=[0, 1, 2, 3] vanished=4
```

The records are **identical**. So `received`, the reading a careful person
reaches for first, says the two rings did the same thing. The counts differ, but
they read as *one ring took twice the work*, not as *one ring destroyed half of
it*. `Outcome::vanished` is accepted less delivered less still-held. It is the
only reading that sees the destruction, and this run is why it is a method rather
than a sentence. → [`docs/pitfall/001`](docs/pitfall/001_neither_the_count_nor_the_list_alone.md).

## What the implementation settled

| Question | Answer |
|---|---|
| Does the crate contain a model checker? | No. `loom` already is one, and four crates in the family use it. This crate contributes the **bridge**: `leak_ends`, plus `audit_received` for inside the closure |
| Where does the loom model live? | `tests/`, not `src/`. `cfg`-removed lines in `src/` count as *uncovered*, so a model there would put a permanent hole in the crate's coverage |
| Does `audit` fail on a vanished record? | No, deliberately. A destroyed record is *accounted for*; whether destruction is acceptable is the caller's policy question, and `vanished` is how they ask it |
| Is `vanished` a field? | A method. An eleventh field would be a second copy of a number three other fields already determine, free to disagree with them |
| Does `Step::Reopen` skip the close when already open? | No. An `is_closed()` guard would make the no-op cheaper and the concurrent hazard worse, because the gap between check and close is another window |

## What it does not establish

Ten equal single-threaded runs prove the *fixture* adds no variability: no
iteration order, no address, no hash seed. They prove nothing about concurrency,
because a script with one producer and one consumer has no interleaving to get
wrong.

That is the whole reason for `tests/exhaustive_test.rs`, which explores every
interleaving of a two-slot ring under `loom`. It passes in 0.02s, which is fast
enough to be suspicious. So
[`tests/manual/readme.md`](tests/manual/readme.md) M2 deliberately inverts an
assertion and confirms loom finds the interleaving that violates it. Without that
negative control the 0.02s would be evidence of nothing.
→ [`docs/non_functional_requirement/001`](docs/non_functional_requirement/001_two_runs_compare_equal.md).

## Layout

| File | Responsibility |
|------|-----------------|
| `docs/` | The measurement, the accounting law, and open trade-offs. See [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | `Script`, `Step`, `Outcome`, `Anomaly`, `audit_received`, `audit_received_unordered`, `leak`, `leak_ends` |
| `tests/testkit_test.rs` | 33 tests forming the scripted half |
| `tests/exhaustive_test.rs` | 3 loom models forming the exhaustive half, behind `--cfg loom` |
| `tests/manual/readme.md` | Manual plan and dated run record |
