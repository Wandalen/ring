# Pattern: Validate the Simple Configuration Before the General One

### Scope

- **Purpose**: Give the practice this crate exists to embody its general form — build and prove the degenerate case first, so a later defect is attributable — and state honestly when it does not pay.
- **Responsibility**: The problem, the practice, its applicability, and the costs it carries.
- **In Scope**: The practice as applied within this family and beyond it; the conditions that make it work.
- **Out of Scope**: This crate's own specific obligation (→ [Correctness Floor for the Family](../non_functional_requirement/001_correctness_floor_for_the_family.md)); the inference hazard it creates (→ [SPSC Correctness Does Not Transfer](../pitfall/001_spsc_correctness_does_not_transfer.md)).

### Problem

A concurrent data structure's defects are **confounded**: several independent
causes produce the same symptom, and the symptom carries no information about
which one is responsible.

A multi-producer ring emitting wrong output has at least four candidate causes:

1. The ring's fundamental design is wrong.
2. The claim protocol is wrong under contention.
3. A memory-ordering annotation is too weak, and only manifests under
   contention or on a weaker-memory-model target.
4. The test itself is wrong about what correct output looks like.

Debugging proceeds by eliminating candidates, and **with nothing simpler to
fall back on, none of them can be eliminated.** Each requires the others to be
known-good to be tested in isolation, and none is.

Cause 4 deserves separate emphasis because it is the one people forget. Under
four producers the correct output is a *multiset* with per-producer subsequences
preserved — a genuinely intricate condition. A test that gets it slightly wrong
fails on correct code or passes on broken code, and there is no simpler test to
check the test against.

This crate's own rationale states the consequence of skipping the simple case
as "jump to MP too soon," and its failure mode as "a correctness bug and a
contention bug look alike."

### Solution

**Build the degenerate configuration first, prove it to a stronger standard
than the general one can be proved to, and make the general case an extension
of it.**

The practice has three parts, and the second is the one usually omitted:

1. **Identify the degenerate configuration** — the one where the hard parameter
   takes its trivial value. Here: one producer instead of N.

2. **Exploit the *stronger* assertions the degenerate case admits.** This is
   the part that makes the practice worth its cost. Simplification is not the
   point; *assertion strength* is:

   | | Degenerate (SPSC) | General (MPSC) |
   |---|---|---|
   | Correct output | One exact sequence | A multiset with per-producer order |
   | Test asserts | Equality | Membership + subsequence |
   | Catches a lossless reordering | **Yes** | No |
   | Interleaving space | Exhaustible under `loom` | Sampled |

   A degenerate case tested only with the general case's assertions has bought
   nothing but a faster test run.

3. **Keep the two provably distinct.** Separate crates, separate test files,
   separate acceptance rows — so that the general case cannot be mistaken for
   the specific one having been proven. This crate and `ring_mpsc` are separate
   rows with separate claiming tests in the acceptance table for exactly this
   reason.

**The practice's payoff is attributive, not preventative.** It does not make
the MPSC ring less likely to have a bug. It makes the bug's cause identifiable
when it appears, by having eliminated cause 1 and cause 4 in advance.

### Applicability

| Situation | Apply? |
|-----------|--------|
| A concurrent structure with a cardinality parameter (producers, consumers, threads) | **Yes** — the canonical case, and the one this crate is |
| The degenerate case admits strictly stronger assertions | **Yes** — this is the condition that makes it pay |
| The degenerate case is merely *smaller*, not *stronger* | **No.** A test that asserts the same thing over fewer items is a faster test, not a floor. The cost of a separate crate and suite buys nothing |
| The general implementation shares no code with the degenerate one | **Rarely.** If MPSC were a wholly separate algorithm, proving SPSC would eliminate no candidate cause. Here they share the buffer, the cursors, the publish ordering, and the drain — which is what makes the elimination real |
| The parameter's trivial value is not reachable in production | Yes, still — it is a test configuration, not a deployment target. But say so, or it will be maintained as if it had users |
| Schedule pressure; the general case is what ships | **This is when it is skipped and when it is most needed.** The cost of skipping is not paid at build time; it is paid during the first production incident, in debugging hours against confounded causes |

**Row three is the honest limit.** The practice is often invoked for cases where
it does not apply — "let's get the simple version working first" is good advice
generally and is *this pattern* only when the simple version supports assertions
the complex one cannot.

### Consequences

- **A separate crate, suite, and maintenance obligation.** `ring_spsc` is one
  crate among the family's 33, and it will never be the configuration a
  consumer runs in production. That is the price, and it is real.

- **The stronger assertions must actually be written.** The pattern's whole
  value sits in the degenerate case's exact-sequence test. Weakening it to a
  multiset comparison — which reads like robustness — discards the benefit
  while keeping the cost (→ [Correctness Floor for the Family](../non_functional_requirement/001_correctness_floor_for_the_family.md)'s
  A2).

- **It creates a transfer hazard.** Properties proven in the degenerate case
  are easy to carry into the general one, and here seven of them are false
  under MPSC. The pattern buys attribution and sells an invitation to
  over-generalize; that trade is why
  [SPSC Correctness Does Not Transfer](../pitfall/001_spsc_correctness_does_not_transfer.md)
  exists as a sibling instance rather than a footnote.

- **Two implementations diverge over time.** Once the general case ships and
  the degenerate one is only a test configuration, a change made for the
  general case can quietly break or bypass the degenerate one. The
  requirement that "the identical test suite" pass against both `ring_core`
  backends is the family's guard against that drift, and it needs a
  counterpart between SPSC and MPSC that the acceptance table does not
  currently name.

- **Exhaustive model checking becomes tractable exactly once.** At one producer
  and one consumer the interleaving space is small enough for `loom` to explore
  to exhaustion. That is a property of the degenerate
  configuration alone and it does not recur at any larger cardinality — so if
  it is not exploited here, it is not available anywhere.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) | The degenerate procedure whose MPSC counterpart is an extension of it |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_producer_one_consumer.md](../invariant/001_exactly_one_producer_one_consumer.md) | The cardinality assumption that defines the degenerate configuration |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_correctness_floor_for_the_family.md](../non_functional_requirement/001_correctness_floor_for_the_family.md) | This pattern applied to this crate, with measurable thresholds |
| [../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md](../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md) | The stronger assertion part 2 requires, stated as criteria |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_spsc_correctness_does_not_transfer.md](../pitfall/001_spsc_correctness_does_not_transfer.md) | The transfer hazard this pattern creates, worked out in full |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_slot_state_without_holes.md](../lifecycle/003_slot_state_without_holes.md) | The structural simplification the degenerate cardinality produces |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate and `ring_mpsc` kept as separate rows with separate claiming tests |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `one_producer_and_one_consumer_exchange_one_hundred_thousand_items` — the exact-sequence assertion part 2 requires, and the pattern's payoff: position `i` must hold payload `i`, which a multi-producer suite structurally cannot assert because no total order over its records exists |

### SP41 — The Pattern Has One Instantiation and This Crate Is It

`ring_spsc` and `ring_mpsc` are the pair. Nothing else in the thirty-three has a
reduced sibling — `ring_cursor` has no single-threaded variant, `ring_store` has
no unsynchronized one.

So the pattern is a description of one relationship rather than a generalization
tested against variation, and it is worth saying so where it is documented.
