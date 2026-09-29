# Invariant: Never Reads Past What Was Published

### Scope

**Purpose:** State the property the crate exists to hold, identify the code that
holds it, and establish what verifies it.

**Responsibility:** The invariant that no sequence is ever reported available
before its producer published it — its statement, its mechanism, and its
verification.

**In Scope:** `Consumer::available`'s dependence on `Barrier::frontier`; the
threaded test asserting it; the ordering pair that makes the read valid.

**Out of Scope:** The consumer's own cursor discipline — that is
[`002`](002_the_cursor_only_moves_forward.md). The empty-barrier default, which
is [`pitfall/002`](../pitfall/002_the_empty_barrier_is_the_case_nobody_measured.md).

---

## The Property

> For every `Seq` that `Consumer::available()` includes in its run, the producer
> has already published that sequence — that is, the producer's cursor has
> advanced past it — and the publication happens-before this consumer observes
> it.

Everything else in the crate is either machinery for this or convenience over
it.

## The Mechanism

```rust
let readable = self
  .barrier
  .frontier()
  .map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );
```

The run's length is bounded by the frontier, and the frontier is the *slowest*
of the barrier's dependency cursors. So the run never extends past the least
advanced producer, which is exactly the guarantee: a sequence is only available
once every dependency has passed it.

### CN10 — The Invariant Rests Entirely on One `map_or`, and on `slowest` Being a Minimum

Two facts hold the property, and both are one crate away:

| Fact | Where | If it were wrong |
|------|-------|------------------|
| `frontier()` is the minimum of the dependency cursors | `ring_cursor::slowest` → `ring_seqno::slowest` → `.min()` | the run would extend past a lagging producer; unpublished slots read |
| the frontier read is `Acquire` | `ring_cursor::GATING` | the run bound would be visible without the data it bounds |

`ring_consume` contributes the subtraction and the zero default. It does not
contribute the minimum, the atomic ordering, or the wraparound-safe distance —
those are `ring_seqno`, `ring_cursor`, and `ring_seqno` again.

That is the correct decomposition and it is worth stating because it changes
where a reader should look when the invariant is in question. A bug in this
crate's 479 lines could produce a wrong *length*; it could not produce a run
that includes an unpublished sequence, because this crate never chooses the
upper bound. The upper bound is chosen by a `.min()` in `ring_seqno` and read
through an `Acquire` load in `ring_cursor`.

Conversely: a change to `ring_seqno::slowest` — say, to return the *fastest*
cursor for some batching optimisation — would break this invariant in a crate
whose tests do not import `ring_seqno`, whose documentation does not mention it
except at one fully-qualified call site
([`integration/001`](../integration/001_four_edges_in_and_none_out.md) CN1), and
whose own 21 tests would still all pass if `ring_seqno` were stubbed to return the
same values for the two-cursor cases they use.

**Cost:** none today. Recorded because the invariant's true location is not
where a reader of this crate would look for it.

---

### CN11 — The Threaded Test Asserts It, and Is the Only Thing That Does

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -B1 -A37 -F '  assert_eq!( consumer.barrier().len(), 2 );' ring_consume/tests/consume_test.rs
grep -c 'thread::scope' ring_consume/tests/consume_test.rs
```

Live output:

```

  assert_eq!( consumer.barrier().len(), 2 );
  assert_eq!( consumer.barrier().frontier(), Some( Seq::ZERO ) );
}

// ── under a live producer ──────────────────────────────────────────────────

#[ test ]
fn a_consumer_never_reads_past_what_was_published()
{
  // The safety property, with a real writer moving the frontier underneath.
  // `available` may under-report — a publication that landed after the read is
  // simply not yet seen — but it must never over-report, because the caller
  // reads slots on the answer.
  const TOTAL : u64 = 4_000;
  let published = published_at( 0 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );

  std::thread::scope( | scope |
  {
    scope.spawn( ||
    {
      for frontier in 1..=TOTAL
      {
        published[ 0 ].store( Seq( frontier ), Ordering::Release );
      }
    } );

    let mut drained = 0u64;
    while drained < TOTAL
    {
      let run = consumer.available();
      assert!( run.end().0 <= TOTAL, "available reached {:?}, past everything ever published", run.end() );

      if !run.is_empty()
      {
        consumer.commit( run.end() ).expect( "committing exactly what was offered" );
        drained = run.end().0;
2
```

`a_consumer_never_reads_past_what_was_published` at `consume_test.rs:334` is the
test named for the invariant, and it is one of the crate's two threaded tests.
It runs a real producer against a real consumer and asserts the property under
actual concurrency rather than by construction.

That is the right test, and the crate's manual plan covers the structural half
the test cannot reach. What each instrument does and does not do:

| Verification | Present | Covers |
|--------------|:-------:|--------|
| a named threaded test | ✔ — `consume_test.rs:334` | the property, under real concurrency, once |
| `tests/manual/readme.md § N4` | ✔ | that the bound comes from `ring_seqno` and no arithmetic is hand-rolled here |
| `tests/manual/readme.md § N1` | ✔ | that the cursor is borrowed — the wiring mistake no test can catch |
| a `loom` model exploring interleavings | ✘ | — |
| a mutation run recording the test's sensitivity | ✘ | — |

The manual plan is substantial — six checks, run 2026-08-28, 6/6 as expected —
and it is aimed precisely where automation cannot reach. Its opening states the
reason directly: *this crate's central risk is a wiring mistake, not a logic
one*, and a `Consumer` whose cursor is private "compiles, runs, passes all 21
tests, and gates nothing."

What is absent is the last row, and `ring_claim` has it:
`tests/manual/readme.md § C1` deliberately breaks the compare-exchange into a
load-then-store and records that the suite detected it 4 times in 5, then 8 in 8
after the test was strengthened. That check is the only evidence anywhere in the
family that a concurrency test actually has the sensitivity its name implies.

The gap matters here because this invariant fails *quietly*: a consumer reading
one sequence past the frontier reads a slot the producer is mid-write on, which
on most runs produces plausible-looking data rather than a crash. A single-run
threaded test is the instrument least likely to catch it, and `§ N3`'s reasoning
— that dropping either half of the guard "leaves a version that passes every
test which only commits what it was offered" — is exactly the kind of argument
that a mutation run would turn from an assertion into a measurement.

The cheapest strengthening is not a `loom` model — the interesting interleaving
is inside `ring_cursor`, which already has the `ring_atomic` seam and where
`ring_publish`'s handshake test already uses it. It is a seventh manual check in
the shape of `ring_claim § C1`: change `map_or( 0, … )` to
`map_or( 0, | f | pending( f, position ) + 1 )`, run the suite `n` times, and
record how many runs notice.

**Cost:** reachable, and narrower than it first appears. The invariant is
correct, tested under concurrency, and supported by two manual checks aimed at
the structural risk. What is missing is any measurement of whether the threaded
test would notice if the invariant stopped holding — the one thing `ring_claim`
established for its own equivalent and this crate has not.

---

## What Holds It Up

| Layer | Contribution | Enforced by |
|-------|--------------|-------------|
| `ring_seqno::slowest` | the minimum | its own tests |
| `ring_cursor::GATING` | the `Acquire` read | one shared constant, six importers |
| `ring_barrier::frontier` | `None` for an empty set | `ring_barrier`'s suite and module doc |
| `ring_consume::available` | the subtraction and the zero default | `consume_test.rs:327` |
| the producer | publishing before advancing | `ring_publish`, and the handshake test |

Five layers, four crates. This crate is the fourth.

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| invariant | [002](002_the_cursor_only_moves_forward.md) | the consumer's own half of the discipline |
| algorithm | [001](../algorithm/001_position_frontier_pending.md) | the three steps that compute the bound |
| non_functional_requirement | [002](../non_functional_requirement/002_eleven_constants_and_the_one_that_is_shared.md) | the `Acquire`/`Release` pair the invariant needs |
| pitfall | [002](../pitfall/002_the_empty_barrier_is_the_case_nobody_measured.md) | why zero is the safe default |
| integration | [001](../integration/001_four_edges_in_and_none_out.md) | the `ring_seqno` edge the invariant actually rests on |

### Sources

| What | Where |
|------|-------|
| The bound | `ring_consume/src/lib.rs:339-342` |
| The minimum | `ring_seqno/src/lib.rs`, `slowest` |
| The ordering | `ring_cursor/src/lib.rs:89` |
| The threaded test | `ring_consume/tests/consume_test.rs:334` |
| The manual plan | `ring_consume/tests/manual/readme.md § N1`, `§ N4` |
| `ring_claim`'s mutation check | `ring_claim/tests/manual/readme.md § C1` |

### Tests

| Claim | Verified by |
|-------|-------------|
| No sequence past the frontier is available | `consume_test.rs:334`, under `thread::scope` |
| The frontier is a minimum | `ring_seqno::slowest` is `.min()`; `ring_seqno`'s own suite |
| Two threaded tests exist | `grep -c 'thread::scope'` → 2 |
| Six manual checks exist, run 6/6 | `ring_consume/tests/manual/readme.md`, Run Record 2026-08-28 |
| None of them is a mutation run | reading N1–N6; all six are structural greps with expected shapes |
