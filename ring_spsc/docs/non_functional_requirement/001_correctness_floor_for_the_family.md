# Non-Functional Requirement: Correctness Floor for the Family

### Scope

- **Purpose**: State the quality obligation this crate carries on behalf of the other 32 — that it is the configuration everything else is validated against — and make it measurable rather than aspirational.
- **Responsibility**: The attribute, the statement, how it is measured, and the threshold that counts as met.
- **In Scope**: This crate's role as the family's reference configuration; the ordering obligation that follows.
- **Out of Scope**: The benchmark verdict itself, which is [`ring_bench`](../../../ring_bench/readme.md)'s; this crate's own byte-parity criterion (→ [Byte-Parity Over 100 000 Items](002_byte_parity_over_one_hundred_thousand.md)).

### Quality Attribute

**Verifiability** — specifically, the family's ability to attribute a defect to
a cause. A secondary attribute is **testability**: this is the one
configuration whose output order is fully determined by its input order.

### Statement

> The SPSC configuration must be correct, and demonstrated correct, **before**
> the multi-producer path is implemented — so that a defect found later can be
> attributed to producer cardinality rather than to the ring design.

Skipping this ordering has a precise cost: work would start at the
multi-producer case, where a correctness bug and a contention bug look alike,
with no simpler configuration to fall back to when the numbers or the ordering
disagree with expectations.

**That is a claim about diagnosis, not about difficulty.** MPSC is not merely
harder; it is *confounded*. A ring producing wrong output under four producers
has at least three candidate causes — the ring design, the claim protocol, and
a memory-ordering error that only manifests under contention — and no way to
separate them. Establishing the same ring correct at one producer eliminates
the first, leaving two.

**The determinism half is what makes the elimination possible.** This
configuration is the shape whose output order is fully determined by its
input order. Under one producer and one consumer there is exactly one correct
output sequence for a given input, so a test can assert equality rather than
membership. Under four producers the correct output is a *multiset* — arriving
with byte-parity as a multiset of 100 000 items — and a multiset assertion
cannot catch an ordering defect that preserves the elements.

| Configuration | Correct output is | A test can assert | Catches ordering defects |
|---------------|-------------------|-------------------|--------------------------|
| SPSC | One exact sequence | Equality | **Yes** |
| MPSC | A multiset, with per-producer order preserved | Membership + per-producer subsequence | Partially |

**So this crate is not merely the simplest configuration — it is the only one
whose test can fail on an ordering bug that loses nothing.** That is the
substance of the floor.

### Measurement Method

1. **Stage ordering.** This crate's ring API and `ring_mpsc`'s are staged
   together, so the ordering obligation is *within* that shared stage rather
   than across stages, and it is not enforced by any external stage gate — it
   must be enforced by the tests' own dependency: `ring_mpsc`'s claiming test
   is not run against an unproven ring.

2. **Exact-sequence assertion.** `ring_spsc/tests/spsc_test.rs` asserts the
   drained sequence equals the published sequence element for element, not as a
   multiset. This is the assertion the MPSC test structurally cannot make.

3. **Model checking.** `loom` under
   [`ring_testkit`](../../../ring_testkit/readme.md) explores the claim/publish/commit
   interleavings to a declared bound and reports zero violations. At one
   producer and one consumer the interleaving space is small enough to explore
   exhaustively rather than sampled — another property that does not survive
   the move to four producers.

4. **Shared-suite discipline.** This crate and `ring_mpsc` have separate
   claiming tests, not one parameterized suite. A shared suite run at
   `producers = 1` would look like this requirement being met while actually
   encoding the assumption
   [SPSC Correctness Does Not Transfer](../pitfall/001_spsc_correctness_does_not_transfer.md)
   warns against.

### Acceptance Threshold

| # | Criterion | Met when |
|---|-----------|----------|
| A1 | This crate's own reached-test passes | 100 000 items, byte-parity, **in order**, zero loss, no lock in the path |
| A2 | The assertion is exact-sequence, not multiset | The test would fail on a permutation that loses nothing |
| A3 | `loom` reports zero violations over the interleavings, exhaustively at this cardinality | The declared bound is reached rather than sampled |
| A4 | No read-modify-write occurs in either path | The counting ordering shim reports zero (→ [No Lock in the Path](../invariant/002_no_lock_in_the_path.md)) |
| A5 | `ring_mpsc`'s claiming test is not the mechanism by which this crate is first exercised | Separate test files, separate rows in the acceptance table |

**A2 is the criterion most likely to be quietly weakened.** Rewriting the
assertion as a sorted comparison or a multiset check makes the test pass under
more conditions, which reads as robustness and is actually the loss of the
only property this crate uniquely provides. If A2 is dropped, the family has no
configuration that can catch a pure ordering defect.

**A4 belongs here and not only in the invariant instance** because an RMW in
this path corrupts the *comparison*, not just this crate: the benchmark exists
to attribute cost to producer cardinality, and an SPSC path that also performs
an RMW makes the SPSC and MPSC numbers converge for a reason unrelated to the
variable under study.

**Not a threshold: a performance number.** This requirement says nothing about
throughput. Whether the ring is fast enough is `ring_bench`'s question, and
the family's own benchmark verdict settles it. This one is about whether a
number, once obtained, can be interpreted.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) | The path A4 measures |
| [../algorithm/002_single_consumer_drain.md](../algorithm/002_single_consumer_drain.md) | The path whose determinism A2 asserts |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_no_lock_in_the_path.md](../invariant/002_no_lock_in_the_path.md) | A4, and why its violation damages the family rather than this crate |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [002_byte_parity_over_one_hundred_thousand.md](002_byte_parity_over_one_hundred_thousand.md) | A1's criterion, stated in full as this crate's own reached-test |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_spsc_correctness_does_not_transfer.md](../pitfall/001_spsc_correctness_does_not_transfer.md) | The trap this role creates; measurement 4 and A5 are its structural mitigation |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_validate_simple_before_general.md](../pattern/001_validate_simple_before_general.md) | The general form of this requirement, stated as a reusable practice |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_slot_state_without_holes.md](../lifecycle/003_slot_state_without_holes.md) | Why exhaustive `loom` exploration is tractable at this cardinality |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate's own reached-test and `ring_mpsc`'s, listed as separate rows — A5 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `one_producer_and_one_consumer_exchange_one_hundred_thousand_items` — A2, the assertion the MPSC suite structurally cannot make. Each payload is derived from its own index, so position `i` holding payload `i` checks order and byte-parity in one comparison |
| `tests/spsc_test.rs` | `exhaustive::a_published_record_is_never_observed_before_the_write_that_preceded_it` and `exhaustive::the_consumer_never_sees_further_than_the_producer_published` — A3, under `RUSTFLAGS="--cfg loom"`. What loom explores exhaustively is the *cursor* protocol; the payload's visibility is asserted through a loom atomic, because the slot itself is plain memory loom does not model (→ `tests/manual/readme.md` S9) |

### SP37 — The Correctness Floor Is Measured Here and Consumed Elsewhere

The requirement's argument is that a property failing here fails everywhere, so
establishing it at cardinality one is the cheap half of the family's evidence.

**The relationship is one-directional and undeclared.** Nothing in `ring_mpsc`
references this crate's suite, and no shared test operates over both. The floor
is a claim about what the two suites jointly establish, held together by prose.

### SP38 — A Ring Smaller Than the Traffic Loses Nothing

The hundred-thousand-item parity test can pass with a ring large enough that
wrapping is rare. This one makes wrapping the common case, which is where the
reuse argument is actually exercised.

Two tests, one property, different pressure — and only the second one makes the
slot-reuse path the hot path.
