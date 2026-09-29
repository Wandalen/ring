# Non-Functional Requirement: Byte-Parity Over 100 000 Items

### Scope

- **Purpose**: State this crate's binary Reached condition as its own acceptance criterion, and decompose it into the five independent claims it actually bundles.
- **Responsibility**: The attribute, the exact statement, the measurement, and the threshold — with each clause's failure mode named.
- **In Scope**: The reached-test's five clauses; what each catches; what none of them catches.
- **Out of Scope**: Why this crate must be proven first (→ [Correctness Floor for the Family](001_correctness_floor_for_the_family.md)); throughput, which is [`ring_bench`](../../../ring_bench/readme.md)'s.

### Quality Attribute

**Correctness**, in the specific sense of data integrity end to end: every byte
written arrives, unchanged, once, in order.

### Statement

Verbatim from
[the acceptance table](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md),
this crate's own row:

> One producer and one consumer exchange 100 000 items with byte-parity between
> what was written and what was read, in order, with zero loss and no lock in
> the path.

**Reached is binary.** The acceptance table states the rule directly: "Where a
criterion below names a number, the number is the test's assertion, not a
target to approach." 99 999 items is not 99.999% reached; it is not reached.

**Five independent claims, and each fails differently:**

| # | Clause | Fails as | Would be caught by |
|---|--------|----------|--------------------|
| C1 | 100 000 items exchanged | A hang, or a count mismatch | Any test that counts |
| C2 | Byte-parity | Torn or corrupted payloads | A payload comparison — but only if the payload is non-trivial |
| C3 | In order | A permutation that loses nothing | **Only an exact-sequence assertion.** A multiset check passes |
| C4 | Zero loss | Missing records | A count, and a per-item ledger |
| C5 | No lock in the path | Nothing observable at runtime | **Only a counting shim.** No functional test can see it |

**C3 and C5 are the two that a reasonable test can pass while violating**, and
they are the two that matter most. A ring that reorders without losing anything
satisfies C1, C2 and C4. A ring that takes a mutex satisfies C1 through C4
perfectly — it is *more* likely to be correct, and it has discarded the entire
point of the crate.

**C2 is weaker than it sounds unless the payload is chosen adversarially.** A
payload of identical bytes, or of a small integer, round-trips correctly
through several genuinely broken implementations — a slot that is never written
but happens to hold a plausible value passes. The payload must be
distinguishable per item (the item's own index is sufficient) and wider than a
word, or C2 tests less than it appears to.

### Measurement Method

1. **Two threads, one ring.** The `( Producer, Consumer )` pair moved onto two
   OS threads — not two tasks, not one thread alternating. A single-threaded
   test satisfies every clause while exercising none of the memory ordering
   (→ [Producer Cursor](../type/001_producer_cursor.md)'s V6).

2. **A per-item ledger, not a count.** Each item carries its own index in its
   payload; the consumer asserts item *n* arrives at position *n*. This
   discharges C1, C3 and C4 together, and it is what makes C3 an exact-sequence
   assertion rather than a multiset one.

3. **A payload wider than a word, with per-item distinct content**, so C2 has
   something to compare that a stale or unwritten slot cannot accidentally
   match.

4. **A counting ordering shim** wrapping the atomic operations, asserting zero
   read-modify-writes and no lock acquisition across the whole run. This is the
   only mechanism that discharges C5, and it is the same technique `ring_tls`
   uses for its own zero-atomic claim — family machinery, not a per-crate
   invention.

5. **Bounded time.** The run completes within a wall-clock bound rather than
   hanging, which catches an accidental spin loop that would otherwise satisfy
   every functional clause.

6. **Under `loom` as well as natively.** The native run at 100 000 items
   exercises throughput and the common interleavings; `loom` enumerates the
   interleavings the native run does not reach. Neither substitutes for the
   other.

   The reason is *not* that the host is strongly ordered — it is aarch64
   (Neoverse-N1), weakly ordered. It is that a native run **samples** while
   `loom` **enumerates**: measured, weakening `HANDOFF` to `Relaxed` leaves the
   100 000-item run passing while the `loom` model fails every time
   (→ `tests/manual/readme.md` S9). Scale buys throughput evidence, not
   interleaving coverage.

### Acceptance Threshold

| # | Criterion | Met when |
|---|-----------|----------|
| B1 | Exactly 100 000 items are drained | The ledger has 100 000 entries, no more, no fewer |
| B2 | Item *n* is at position *n*, for all *n* | Exact-sequence equality, not multiset membership |
| B3 | Each item's payload is byte-identical to what was written | Compared over a per-item-distinct payload wider than a word |
| B4 | Zero read-modify-write operations and zero lock acquisitions occur | The counting shim reports zero for both |
| B5 | The run completes within its wall-clock bound | No hang, no unbounded spin |
| B6 | `loom` reports zero violations over the same protocol | The declared bound reached, not sampled |

**B4 is the criterion that distinguishes this from a correctness test of any
queue.** B1, B2, B3 and B5 would be satisfied by a `Mutex<VecDeque>`. B4 is
what asserts the ring is the thing that was built.

**This threshold does not assert throughput, latency, or capacity behaviour.**
A ring meeting every criterion above may still be slower than the mutex-guarded
candidate it is benchmarked against — that outcome is permitted, and
`ring_bench`'s measured configurations exist to discover it. The family's own
benchmark verdict owns that question; this requirement establishes only that
the numbers, whatever they are, were taken from something correct.

**It also does not assert behaviour at capacity boundaries.** 100 000 items
through a ring of capacity 1 024 laps the buffer roughly 98 times, which
exercises wraparound thoroughly, but the test says nothing about the Full path
unless the consumer is deliberately stalled. That is a separate criterion
carried by [Ring Occupancy Between the Cursors](../lifecycle/004_ring_occupancy.md)'s
tests, and it is worth naming as a gap in *this* criterion rather than assuming
the item count covers it.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) | The path C5 and B4 measure |
| [../algorithm/002_single_consumer_drain.md](../algorithm/002_single_consumer_drain.md) | The path C3 and B2 assert over |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | The surface the producing thread drives |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | The surface the ledger is built from |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_no_lock_in_the_path.md](../invariant/002_no_lock_in_the_path.md) | C5 and B4, stated as an invariant rather than a criterion |
| [../invariant/001_exactly_one_producer_one_consumer.md](../invariant/001_exactly_one_producer_one_consumer.md) | The cardinality measurement 1 establishes by moving the pair to two threads |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_correctness_floor_for_the_family.md](001_correctness_floor_for_the_family.md) | Why B2's exactness is the property the family depends on |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_ring_occupancy.md](../lifecycle/004_ring_occupancy.md) | The Full-path gap this criterion does not cover |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_producer_cursor.md](../type/001_producer_cursor.md) | V6, the ordering rule measurement 1 and B6 exist to exercise |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate's row — the verbatim statement, and the binary-Reached rule |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `one_producer_and_one_consumer_exchange_one_hundred_thousand_items` — this crate's own claiming test for the criterion above |
| `tests/spsc_test.rs` | `tests/manual/readme.md` S2 — B4, the criterion a correct `Mutex<VecDeque>` would fail. Established by reading the crate's compiled surface for `fetch_`, `compare_exchange`, `Mutex` and `RwLock` and finding none, which covers every run rather than the one a shim happened to instrument |

### SP39 — Byte Parity Over One Hundred Thousand, With One Producer

`one_producer_and_one_consumer_exchange_one_hundred_thousand_items` is this
crate's end-to-end. The same element count as `ring_mpsc`'s
`four_producers_exchange_one_hundred_thousand_items_with_byte_parity`, which
makes the pair a like-for-like comparison of the two backends.

**What the two tests can detect differs.** The sibling's is what fires 14 times
in 60 under an ordering mutation; this one has one writer and one reader, so a
weakened ordering has far less interleaving to expose it.

### SP40 — A Full Ring Reports Rather Than Blocks, and Hands the Record Back

`try_push` returns `Err( record )` — the caller gets its value back and can
retry or route it elsewhere. `a_full_ring_reports_rather_than_blocks` covers the
other half: no wait, no growth.

The pair is what "bounded, lossless refusal" means concretely, and it is the
same contract as the sibling's. Where they differ is the trap
(→ [`../pitfall/002`](../pitfall/002_a_departed_counterpart_is_indistinguishable_from_a_slow_one.md)):
a caller that treats `Err` as retry-later spins forever against a dead
consumer.
