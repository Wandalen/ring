# Invariant: Publication Ordering — Claim Acquire, Publish Release

### Scope

- **Purpose**: Fix the memory-ordering contract that makes a published slot's payload actually visible to the consumer, since it is the one property of this crate that compiles, tests green on x86, and fails on ARM when it is wrong — measured here at 14 failures in 60 runs on this workspace's own aarch64 host.
- **Responsibility**: State both handoff pairs — publication and reclamation — the mechanism that enforces each, and what a torn or lap-confused read costs downstream.
- **In Scope**: Which `Ordering` each access to the cursors and the per-slot stamp carries, and why; the provenance of that contract, which is only partly inherited.
- **Out of Scope**: Which steps those accesses sit inside (→ [Claim-Then-Publish Slot Acquisition](../algorithm/001_claim_then_publish.md), [Batch Drain by Single Cursor Swap](../algorithm/002_batch_drain_by_cursor_swap.md)); the fields being ordered (→ [Sequence-Stamped Ring](../data_structure/001_sequence_stamped_ring.md)); the exactly-once and reproducibility clauses this invariant serves but does not restate (→ [Single-Consumer Total Order](001_single_consumer_total_order.md)); what a producer does when the slot it reclaims is still occupied (→ [Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md)).

### Invariant Statement

A slot's payload writes **happen-before** the consumer's reads of that same
slot, and the consumer's reads **happen-before** any producer's reuse of it.
Two pairs, in opposite directions, both required:

**Pair 1 — publication (producer → consumer).** The producer's payload
writes are not reordered after the stamp store, and the consumer's payload
reads are not reordered before its load of that stamp:

- Producer: payload writes, then `stamps[ i ].store( sequence, Release )`.
- Consumer: `stamps[ i ].load( Acquire )`, then payload reads.

**Pair 2 — reclamation (consumer → producer).** The consumer's payload reads
are not reordered after it releases the slot, and the producer's overwrite is
not reordered before it observes that release:

- Consumer: payload reads, then `consumer_cursor.store( w, Release )`.
- Producer: `consumer_cursor.load( Acquire )` — the free-capacity check —
  then the payload overwrite.

**The claim itself would need neither, if it were only a ticket.**
A bare `producer_cursor.fetch_add( 1, Relaxed )` is sufficient for a ticket:
uniqueness and monotonicity are properties of the read-modify-write's
*atomicity*, not of any ordering it establishes, and the ticket value is never
used to reach data. Annotating such a claim `AcqRel` out of caution would buy
nothing and cost a barrier on the single hottest instruction in the crate.

**But the claim is not only a ticket, so this paragraph's own escape clause is
what actually applies.** It already said: "where the free-capacity check and
the claim are fused into one `fetch_update`-style operation, that fused
operation is `AcqRel`, because it is then doing Pair 2's acquire as well."
That is the implemented case, and it is not one of two possibilities —
`ring_claim`'s `Claimer::claim` is a `compare_exchange` loop with the gating
headroom re-read on every attempt, because a bounded ring's headroom check
*cannot* be fused into a `fetch_add`, which an earlier ruling already
established. So the fused `AcqRel` reading is the only one, and the `Relaxed`
ticket above describes a design the crate does not have.

Worth keeping rather than deleting: the `Relaxed`-ticket analysis is still the
correct reason the claim's ordering is *cheap* — the `AcqRel` is paid for
Pair 2's acquire, not for the ticket, so it is not a barrier this invariant
adds on top of an already-necessary one. The conditional was written before
the claim existed, and the branch that fired is the more expensive of the two.

`SeqCst` is explicitly rejected everywhere. It adds a global total order
across *all* atomics in the process, a property nothing here needs, and pays
for it by forcing every core to drain its store buffer — on a path executed
once per element by every producer.

### Enforcement Mechanism

**This instance closes an item its sibling left open, deliberately.**
[Single-Consumer Total Order](001_single_consumer_total_order.md) lists "the
exact atomic memory-ordering annotations proving publication atomicity" among
the things the family's winning pattern would supply. That placement was
wrong, and this doc is the correction rather than a second opinion: an
ordering discipline is a **correctness requirement every candidate must
satisfy**, not a performance variant a candidate gets to choose. A candidate
that publishes without a `Release` is not a faster mechanism to be weighed
against a slower one — it is a mechanism that does not hold the contract, and
there is nothing left to measure. The sibling's Open list has been amended to
say so.

**Provenance — partly inherited, partly originated here.** The distinction
matters because a reader who assumes this was settled elsewhere will not look
for what it did not settle.

The design material this crate's mechanism grew out of specifies **no memory
ordering at all** for the ticket-and-cost-split shape it describes, and one
candidate alternative considered alongside it is a one-sentence name-drop
that does not even commit to a primitive.

Earlier, less-curated design discussion does state a discipline, however: a
complete worked ring pairing `head.store( …, Release )` on the writer with
`head.load( …, Acquire )` on the reader, naming the resulting guarantee a
"Happens-Before relationship"; the same discussion elsewhere concludes
"Acquire/Release is the gold standard for rings" and states both failure
directions — `Relaxed` yields "bugs that show up once in a billion
transactions", `SeqCst` "will kill your speed". The deleted predecessor
crate had already transcribed the same pairing in its own "On orderings"
section. None of this is a decision this crate inherited automatically — it
is background the crate's own contract had to independently justify, not
received settled fact.

**What is therefore inherited: Pair 1, for the single-producer case.** Every
worked example above has exactly one writer. What is **originated here**:
that the pairing survives the generalization to many producers at all, that
the publication token must be the per-slot stamp rather than the shared
producer cursor (a single writer can publish by advancing the cursor; many
writers cannot, because the cursor advances at claim time and publication
completes out of order —
→ [Batch Drain by Single Cursor Swap](../algorithm/002_batch_drain_by_cursor_swap.md)'s
published-watermark problem), and Pair 2 in the form stated above.

**Pair 2 is where the inherited material is actually wrong for this crate.**
The earlier discussion's consumer advances its read cursor with
`self.tail.store( current_tail + 1, Ordering::Relaxed )`, justified as
"Relaxed is fine here, since the Mutator is the only one who changes tail".
Sole-writership is a real property and it answers a real question — nobody
else races that store — but it is not the question reclamation ordering
asks. The question is whether the consumer's *payload read*, which precedes
that store in program order, is guaranteed to complete before a producer
observing the advanced cursor overwrites the slot. A `Relaxed` store carries
no such guarantee; the compiler or the hardware may sink it above the read.
This crate's contract therefore requires `Release` there, and the
divergence is recorded rather than silently corrected because a reader
comparing this doc against the source would otherwise read it as a
transcription error.

**Enforcement is not the compiler.** Rust type-checks that an `Ordering`
argument was passed; nothing checks that it was the right one. `Relaxed` where
`Release` belongs compiles cleanly.

**The reason a green suite is weak evidence is not the one first written here,
and the correction matters.** This section originally argued that the bug is
*unobservable* on x86-64's TSO model, so a green suite on "x86-64 developer
hardware" proves nothing. That premise is false for this workspace: the host is
`aarch64-unknown-linux-gnu` (ARM Neoverse-N1), which is weakly ordered — check
with `rustc -vV | grep host`. The argument was explaining away a gap by
appealing to hardware the project does not run on.

Measured on the actual host, one ordering at a time mutated to `Relaxed`:

| Constant | Hardware parity test | `loom` model |
| -------- | -------------------- | ------------ |
| `PUBLISH` | **8 failures in 20 runs** | fails deterministically |
| `OBSERVE` | passes | fails |
| `COMMIT`  | passes — 120/120 | passes |

So the true position is stronger in one place and weaker in two. A green suite
is weak evidence for `PUBLISH` because the check is *probabilistic* (40%), not
because it is impossible. For `OBSERVE`, `loom` carries it alone. For `COMMIT`,
**nothing behavioural checks it at all** — see `tests/manual/readme.md` M4,
where this is recorded as the crate's one ordering held by argument rather than
by evidence.

Enforcement rests on three things, **all three of which now exist**:

1. **This document.** The orderings are stated here once, as contract, and
   the two algorithm instances deliberately omit them at each step rather
   than restating them where a reader could take them for incidental detail.
   `the_orderings_are_the_ones_the_publication_invariant_names` asserts that
   the crate's four public ordering constants are the ones stated here, so a
   silent weakening fails a test rather than only contradicting prose.
2. **Model checking under the test surface — `tests/mpsc_test.rs`'s
   `exhaustive` module.** Two `loom` models, run under
   `RUSTFLAGS="--cfg loom"`: one asserts a published record is never observed
   before the write that preceded it (Pair 1), the other that the consumer
   never drains past what producers published. `ring_atomic` swaps in an
   instrumented cell under the same cfg, so what loom explores is this
   crate's own protocol rather than a re-implementation of it.
3. **A weakly-ordered target — and this item's premise was wrong.** It
   required "at least one weakly-ordered target (AArch64)" and called itself
   "checkable today only as an absence." **The development host is
   `aarch64-unknown-linux-gnu`** (ARM Neoverse-N1; `rustc -vV | grep host`).
   The weakly-ordered target was never absent — it was the machine the
   document was being written on. The correct reading of this item is
   therefore inverted: the ordinary suite is *already* running where this
   failure class is observable, and the thing that needed adding was not
   hardware but a mutation to prove the hardware was being used.

**So the mutation was run, and the result is the reason item 2 is not
optional.** Weakening `PUBLISH` from `Release` to `Relaxed`, then running the
parity test alone sixty times each way:

| `PUBLISH` | 60 runs | Failure |
|-----------|---------|---------|
| `Ordering::Release` | 60 pass, 0 fail | — |
| `Ordering::Relaxed` | 46 pass, **14 fail** | `sequence Seq(2234) was drained as published but its slot was empty` |

The loom model fails **deterministically**, in 0.010s, under the same mutation.

**A 23% detection rate is a real check and not a reliable one.** Three runs in
four still report green against a genuinely broken publish, so a single green
run of the ordinary suite is not evidence either — a weaker version of the
same caveat this section opened with, not an escape from it. What closes the
gap is that loom *enumerates* interleavings rather than sampling them, so its
verdict does not depend on which one the scheduler happened to pick. The two
are complementary: real hardware at scale, and every interleaving at small
scale. Neither alone would be enough.

### Violation Consequences

**Pair 1 omitted — the torn read.** The consumer loads a stamp that says
published and reads payload bytes the producer has not made visible. In Rust
terms this is a data race and therefore undefined behavior *before* any
symptom exists to observe, which is the part that makes it dangerous rather
than merely wrong. Where a symptom does surface, it surfaces in whatever
consumed the drained element — a decoder, a matching step, a state
mutation — at some later point in the pipeline, never at the producer that
omitted the barrier. Nothing in the failure names its cause. And because the
window is a store-buffer's worth of time, the failure rate is the "once in a
billion transactions" heisenbug 660 describes: reproducible only under
sustained load, on the right hardware, and never under a debugger.

**Pair 1 omitted — the specifically worse variant.** The bytes a torn read
returns are not usually garbage. A ring reuses slots, so the most likely
contents of a not-yet-visible slot are the *previous lap's* fully-formed
element: correctly typed, structurally valid, and one capacity old. It
passes every plausibility check a consumer might apply. A crash would be a
better outcome than this, because a crash localizes.

**Pair 2 omitted — the overwrite race.** A producer sees the consumer cursor
advanced, concludes the slot is free, and overwrites it while the consumer's
read of that same slot is still in flight. The consumer's element is then
half its own and half the next lap's — the same torn read, arriving from the
opposite direction, and invisible to any test that only checks the
publication side.

**What this costs the crate's own contract.** This invariant is not an
addition to
[Single-Consumer Total Order](001_single_consumer_total_order.md); it is that
invariant's mechanism. A torn read violates its **publication atomicity**
clause directly — a partially-written element was drained. A lap-confused
read violates its **exactly-once** clause — lap *k−1*'s element is delivered
a second time, in lap *k*'s position, while lap *k*'s is lost. Neither
violation is detectable by counting elements, which is what makes an ordering
bug distinct from a logic bug: the totals reconcile.

**And downstream, per prospective consumer.** The prospective consumer's use
is intent submission where the log is the source of truth, so a lap-confused
element is a *replayed intent presented as a new one* — divergence with
nothing erroring. The deleted predecessor crate's prospective use was actor
mail, where the same element would have been a duplicated structural
command — the double-apply class its own pitfall instance named one layer
up, before it was deleted with no successor.

### Algorithms

| File | Relationship |
|------|--------------|
| ~~the deleted predecessor crate's push/drain mechanism~~ | Already transcribes Pair 1 for the CAS-stack shape — the `Release` on the winning compare-exchange paired with the `Acquire` on the drain swap. **Deleted 2026-08-26**, and it has no successor. |
| [../algorithm/001_claim_then_publish.md](../algorithm/001_claim_then_publish.md) | The producer steps this invariant annotates: `Relaxed` claim, unordered payload write, `Release` stamp |
| [../algorithm/002_batch_drain_by_cursor_swap.md](../algorithm/002_batch_drain_by_cursor_swap.md) | The consumer steps: `Acquire` watermark read, unordered payload walk, `Release` cursor release |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | The stamp Pair 1 orders against and the two cursors Pair 2 runs between; explains why the stamp rather than the cursor is the publication token |

### Invariants

| File | Relationship |
|------|--------------|
| [001_single_consumer_total_order.md](001_single_consumer_total_order.md) | The contract this invariant is the mechanism for — its publication-atomicity and exactly-once clauses are what an ordering violation actually breaks |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | Carried enforcement item 3 as a requirement on a future harness — discharged instead by the host already being aarch64, which that instance records |

### Pitfalls

| File | Relationship |
|------|--------------|
| ~~the deleted predecessor crate's own pitfall~~ | The double-apply class a lap-confused duplicate reproduces at intake grain. **Deleted 2026-08-26**, and it has no successor. |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Declares `PUBLISH`, `OBSERVE`, `COMMIT` and `OWN` as public constants — one named site per ordering in this contract, so a weakening is a one-line diff rather than a change buried in a call |
| `tests/manual/readme.md` | The dated run record for the mutation experiment, including the killed-run incident that produced the `PATIENCE` deadlines |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::the_orderings_are_the_ones_the_publication_invariant_names` | The four constants asserted against this document, so weakening one fails a test rather than only contradicting prose |
| `tests/mpsc_test.rs::exhaustive::a_published_record_is_never_observed_before_the_write_that_preceded_it` | Pair 1 under `loom` — exhaustive over interleavings, and the model that fails deterministically under the `Relaxed` mutation |
| `tests/mpsc_test.rs::exhaustive::the_consumer_never_drains_further_than_the_producers_published` | Pair 2's direction: no drain past the published frontier under any interleaving |
| `tests/mpsc_test.rs::four_producers_exchange_one_hundred_thousand_items_with_byte_parity` | The hardware half — the test whose empty-slot `expect` is what fires 14 times in 60 under the mutation |
| `tests/mpsc_test.rs::a_stale_stamp_from_the_previous_lap_does_not_read_as_published` | The lap-confusion failure mode named under Violation Consequences, asserted directly rather than left as a described risk |

### MP24 — Two of the Three Checks on This Invariant Are Outside the Default Test Path

Three independent things check the publication ordering, and they are not
equally reachable:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
printf 'ordinary body gate:  '; grep -m1 -n 'cfg( not( loom ) )' tests/mpsc_test.rs
printf 'exhaustive gate:     '; grep -m1 -n 'cfg( loom )' tests/mpsc_test.rs
printf 'loom dep gate:       '; grep -n "cfg(loom)" Cargo.toml
printf 'verb/test sets it:   '
grep -c 'cfg loom' /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/verb/test 2>/dev/null || echo 0
```

Live output:

```
ordinary body gate:  38://! The whole ordinary body is `#[ cfg( not( loom ) ) ]` because a `Ring`'s
exhaustive gate:     1261:#[ cfg( loom ) ]
loom dep gate:       26:[target.'cfg(loom)'.dev-dependencies]
verb/test sets it:   0
0
```

The value assertion — `PUBLISH == Release`, `OBSERVE == Acquire`,
`COMMIT == Release`, `OWN == Relaxed` — runs on every ordinary invocation and
catches a wrong constant, not a right constant used in the wrong place. The
`exhaustive` loom model catches the second, and compiles only under
`RUSTFLAGS="--cfg loom"`. The 60-run mutation this instance records above is the
strongest evidence of the three and is a manual experiment: nothing re-runs it.

**So the invariant that this crate's own scope line calls its one property that
"compiles, tests green on x86, and fails on ARM when it is wrong" is guarded, by
default, only at the level of the constants' declared values.** That is not an
argument for deleting either of the other two — it is the reason both are
written down here rather than left as a green suite's implicit claim.

### MP25 — The Divergence on `COMMIT` Is Deliberate and Documented

The rustdoc on `COMMIT` names the divergence and its source directly. Recorded
here because a divergence from the design this crate grew out of is the kind
of thing that gets silently reverted by a later reader "fixing" it back to
the source.

The argument is contained: the consumer's payload reads precede the store in
program order and must not sink below it, or a producer observing the advance
overwrites a slot still being read. That is a claim about this crate's drain,
which the external design does not have.
