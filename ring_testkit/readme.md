# ring_testkit

Determinism-test fixtures driving scripted claim and drain sequences.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

`Script::run` drives a `ring_core::Ring`, stages through a `ring_tls::TlsBuffer`,
and publishes through a `ring_shutdown` guard, which is what makes `Step::Close`
mean something.

## What it does

A `Script` is a list of `Step`s. Run it against a ring and it returns an
`Outcome`: counters, lists and a flag describing what the ring did, with no
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
| `Push` / `PushMany` | Mint a record and publish it through the shutdown guard |
| `Recv` / `RecvMany` | Take from the consumer end |
| `Stage` / `StageMany` | Mint into the `TlsBuffer` instead of the ring |
| `Flush` | Move the staged records into the ring, one at a time |
| `Close` / `Reopen` | Flip the guard; a closed ring refuses publications and still permits reads |
| `DrainAll` | Close, then recover everything still held |

## The measurement it is built around

`ring_core::try_push` reports **`Ok` and destroys the record** when the ring is
full under `OverflowPolicy::DropNewest`. Two rings differing only in that policy,
each offered eight records at capacity 4, show this:

```
Fail:       accepted=4 refused_full=4 received=[0, 1, 2, 3] vanished=0
DropNewest: accepted=8 refused_full=0 received=[0, 1, 2, 3] vanished=4
```

The records are **identical**. So `received`, the reading a careful person
reaches for first, says the two rings did the same thing. The counts differ, but
they read as *one ring took twice the work*, not as *one ring destroyed half of
it*. `Outcome::vanished` is accepted less delivered less still-held. It is the
only reading that sees the destruction, which is why it is a method rather than
a sentence in a doc.

## What it does not establish

Ten equal single-threaded runs prove the *fixture* adds no variability: no
iteration order, no address, no hash seed. They prove nothing about concurrency,
because a script with one producer and one consumer has no interleaving to get
wrong.

That is the whole reason for `tests/exhaustive_test.rs`, which explores every
interleaving of a two-slot ring under `loom`. It passes fast enough to be
suspicious. So [`tests/manual/readme.md`](tests/manual/readme.md) M2
deliberately inverts an assertion and confirms loom finds the interleaving that
violates it. Without that negative control the fast pass would be evidence of
nothing.

## Decisions

- [`Script::run` stays on `u32` records until a consumer needs to drive its own record type](docs/decisions/001_scripts_stay_on_u32_records.md)

## Run it

```sh
cargo nextest run -p ring_testkit --all-features
cargo test --doc -p ring_testkit --all-features
RUSTFLAGS="--cfg loom" cargo test -p ring_testkit --test exhaustive_test
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | `Script`, `Step`, `Outcome`, `Anomaly`, `audit_received`, `audit_received_unordered`, `leak`, `leak_ends` |
| `tests/testkit_test.rs` | The scripted half |
| `tests/exhaustive_test.rs` | The loom models forming the exhaustive half, behind `--cfg loom` |
| `tests/manual/readme.md` | Manual plan and dated run record |
