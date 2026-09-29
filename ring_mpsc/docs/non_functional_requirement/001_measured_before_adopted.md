# Non-Functional Requirement: Measured Before Adopted

### Scope

- **Purpose**: Make adoption a checkable requirement, so no consumer migrates onto this crate on argument alone.
- **Responsibility**: State the quality attribute, the measurement procedure, and the threshold adoption requires.
- **In Scope**: The evidence required before either prospective consumer takes a `[dependencies]` edge on this crate.
- **Out of Scope**: The harness's candidate set and workload design, owned at the family level; migration mechanics after a winning verdict (future work).

### Quality Attribute

Throughput and scalability under multi-producer contention — the one
dimension that justifies replacing a working mechanism with a shared crate.

### Statement

The mechanism this crate ships must be the **measured winner** of the
family's benchmark harness over the synthetic archetype-table workload,
against the candidate set, under **both** SET and Delta accumulator
semantics — and must specifically beat the mechanism it would replace in
each prospective consumer:

- the prospective consumer's documented single-threaded intent-submission
  serialisation point (a baseline already planned to be lifted), and
- the deleted predecessor crate's hand-rolled `AtomicPtr` CAS-stack
  ~~mailbox mechanism~~ (deleted, with no successor).

**The candidate set, named.** The family benches six patterns
against each other and against the two baselines above: thread-local
buffering with single-threaded merge, MVCC double-buffering, this crate's
own Disruptor-shaped ring, a shared exclusive-access buffer, declared
read/write DAG scheduling, and atomics. Three of the six are heavier or
less-specified alternatives this NFR's "measured winner" clause is measured
against directly: MVCC copies every array wholesale on every merge, a cost
this crate's per-record append avoids by construction — and pays a second,
independent cost this crate's own unique-slot-per-claim discipline avoids
structurally rather than by arbitration: two order-blind parallel writers
under MVCC can each finish with a different idea of what one shared value
should hold, and the mechanism itself has no built-in way to say which
result is right — settling it needs bespoke, pair-by-pair arbitration code,
exactly the hand-written special-casing a shared crate is meant to replace;
this crate's own claim
(→ [Claim-Then-Publish Slot Acquisition](../algorithm/001_claim_then_publish.md))
sidesteps the hazard entirely: Step 1's unique sequence guarantees no two
producers are ever assigned the same slot to begin with, so there is never a
collision left over for a merge step to settle; a shared exclusive-access
buffer trades away this crate's own parallel-claim property entirely for
instant same-frame visibility, which nothing in this crate's own contract
requires; atomics is named as a direct-write alternative skipping the
buffer-and-merge phase altogether, with no source elaborating its
contention behavior under sustained multi-producer load.

A losing or inconclusive verdict leaves every consumer exactly as it is
today — this crate stays a skeleton, and nothing regresses.

### Measurement Method

The family's own contract: a criterion benchmark harness over synthetic
archetype tables, producing per-pattern reports plus a written verdict, with
four named failure modes reproduced as measured cases rather
than arguments. The harness's crate now exists — `ring_bench`
([`../../../ring_bench/readme.md`](../../../ring_bench/readme.md)) — but the
benchmarks themselves are not written yet,
so the concrete bench command is still outstanding rather than merely
unrecorded. Naming the crate is not the same as having the number, and this
requirement is not discharged until the number exists.

Until then the checkable artifact is the absence of any `ring_mpsc` edge in a
consumer manifest:

```sh
cd "$(git rev-parse --show-toplevel)"
# must print nothing until a winning verdict exists — checked against the
# prospective consumer's own dependency manifest, outside this repository
command grep -rn "ring_mpsc" <prospective-consumer-manifest> || true
```

Live output:

```
```

**The check lost one of its two targets and was silently broken by it.** It
also named the deleted predecessor crate's own manifest path until a later revision.
That crate was deleted,
which this instance already recorded in its Data Structures table — but the
grep command was not updated with it, so it printed a `No such file or
directory` to stderr and exit-status 2 on every run rather than the silence
the surrounding sentence promised. A verification command that errors is not
a verification command that passes, and the difference is invisible to a
reader who trusts the prose. **The generalizable form:** a deletion recorded
in a cross-reference table does not propagate itself into an executable check
in the same file; the two have to be updated by the same hand, and only the
executable one fails loudly enough to notice.

**One measurement this requirement did not ask for has already landed**, and
it belongs here because it is the crate's first real number rather than an
argument. [Publication Ordering](../invariant/002_publication_ordering.md)'s
`Release`/`Acquire` pair was mutated to `Relaxed` and the parity test run 60
times on the development host: 46 passed, **14 failed**. That is evidence
about correctness rather than throughput, so it discharges nothing here — but
it establishes that the machinery for *taking* a measurement on this crate
works, which was itself uncertain while the crate was a skeleton.

### Acceptance Threshold

Strictly better than the replaced mechanism on the measured workload. The
threshold is deliberately comparative rather than a pre-committed number —
the family's own construction is that adoption happens only on a winning verdict, so
the bar moves with the baseline being replaced, never below it. Speed never
trades against the functional contract: a faster mechanism that weakens
[Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md)
fails regardless of its numbers.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_claim_then_publish.md](../algorithm/001_claim_then_publish.md) | Step 1's unique sequence claim is the structural reason this crate sidesteps MVCC's merge-conflict hazard rather than arbitrating it |

### Data Structures

| File | Relationship |
|------|--------------|
| ~~the deleted predecessor crate's mailbox structure~~ | The baseline the winner must beat. **Deleted 2026-08-26**, and it has no successor. |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_consumer_total_order.md](../invariant/001_single_consumer_total_order.md) | The contract that holds regardless of measured speed — winning the bench never waives it |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_channel_to_ring_binding.md](../pattern/001_channel_to_ring_binding.md) | Both its rules assume a Disruptor-shaped winner; this gate is what decides whether that assumption ever holds for a `scope: thread` channel |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | The mechanism under measurement — implemented, tested, and correct, which is the precondition for benching rather than a substitute for it |
| `../../../ring_bench/readme.md` | Where the harness this requirement depends on will live; the crate exists, its benchmarks do not |

### Tests

| File | Relationship |
|------|--------------|
| `ring_bench` benchmarks (not yet written) | The criterion harness whose reports are this requirement's evidence. Named as outstanding rather than omitted, so the gap is visible in the table rather than only in the prose |
| `tests/mpsc_test.rs::four_producers_exchange_one_hundred_thousand_items_with_byte_parity` | The correctness precondition: a mechanism that loses or duplicates records under contention cannot be adopted on any throughput number |
| `tests/exhaustive.rs::a_published_record_is_never_observed_before_the_write_that_preceded_it` | The loom model that makes the above deterministic rather than probabilistic — the same precondition, checked by enumeration |

### MP37 — The Requirement Is Satisfiable Only Through `ring_core`

A benchmark that names `ring_mpsc::Ring` and then drives it through
`ring_core::Producer` is measuring both layers. Whether the composition's
dispatch `match` is visible in the result is not answerable from the number
alone.

**"Measured before adopted" therefore holds for the pair, not for this crate.**
Recorded because the requirement is stated here and the measurement is taken one
layer up.

### MP38 — No Benchmark Names the Unreached Methods

`stamps()` returns a borrowed slice of the whole array — an operation whose
cost scales with capacity, unlike every other method here. Nothing measures it,
because nothing calls it.

That is a small gap and a real one: if a prospective adopter
(→ [`../integration/002`](../integration/002_prospective_consumer_adoption.md))
polls `stamps()` per frame, the requirement's "measured before adopted" would
need a number that does not exist.
