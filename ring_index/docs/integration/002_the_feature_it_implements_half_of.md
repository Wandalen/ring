# Integration: The Feature It Implements Half Of

### Scope

**Purpose:** Record that this crate's own cited feature record
was marked `present` by a block flip rather than by any check of
the code, that the "every claim" error in the
module comment was inherited from that record rather than invented here, and
that the measurement the record promises is aimed one level above where the
family's one measurement crate actually looks.

**Responsibility:** The edge between `ring_index/` and the external tracking record
it cites — this crate's own cited feature record, and the wider batch of records it belongs to.

**In Scope:** this crate's own cited feature record;
`ring_index/src/lib.rs:1-13`; `ring_bench/src/lib.rs`.

**Out of Scope:** the code-level topology is
[`integration/001`](001_two_dependents_and_a_third_that_did_it_again.md). The
measurement itself, taken with a throwaway probe, is
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_fold_costs.md).

---

## What the Feature Asks For

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^\/\/! Sequence-to-slot index mapping for power-of-two capacities\.$/p;/^\/\/! This is the half of `docs\/feature\/167_sequence_slot_index_and_power_of_two_capacity\.md`$/p' ring_index/src/lib.rs
```

Live output:

```
//! Sequence-to-slot index mapping for power-of-two capacities.
//! This is the half of `docs/feature/167_sequence_slot_index_and_power_of_two_capacity.md`
```

(The status and description text this crate's own cited feature record used to supply here is no
longer reachable: that record lives in an external repository this crate cannot
read from a standalone checkout, so only this crate's own doc-comment citation
of it, quoted above, can still be verified directly.)

This crate's own cited feature record names three things: the sequence, the derived slot index, and the
power-of-two constraint. This crate owns the derivation. `ring_types` owns the
two types and the constructor that enforces the constraint. Nothing else is
required for the record's claim to be complete.

---

### IX13 — The Cited Feature Record Flipped to `present` Without Anything Comparing It to the Code

**Finding.** When this was filed, this crate's own cited feature record read `Status: planned` — the state
of a feature nobody has started. It now reads `present`, and nothing about these
three crates was examined to make that happen: this record was one entry in a
contiguous external range of twenty-two records, all flipped together in the same batch.

The feature's three obligations are each discharged by shipped, tested code:
`Seq` and `SlotIndex` exist in `ring_types/src/id.rs` with a combined 332 lines
of test; `Capacity::new` rejects non-powers-of-two and every other capacity in
`ring_types/src/capacity.rs`; `of` performs the derivation here, under 188 lines
of test.

The citation runs one way only. The crate's own doc comment cites the record by path in its very
first doc line, so the code knows what it implements; the record has no
corresponding pointer back at any of the three crates, so there is nothing on the
tracking side recording that it has been implemented. That asymmetry is what let the status go stale
without anything catching it — there is no place the two would ever be compared.

**The flip did not repair the asymmetry; it stepped around it.** The status is
now right, and it is right for a reason unrelated to whether these three crates
discharge the record's obligations — the same batch also marked one sibling feature record
`present`, whose obligation nothing has measured
([`ring_batch` integration/002 § BA21](../../../ring_batch/docs/integration/002_the_feature_is_planned_its_problems_are_addressed.md)).
A field that lands on the right answer here and the wrong one two entries later
is not a field anyone can read. The missing back-pointer is still missing, and
it is still the only thing that would let the two be compared.

This is recorded rather than fixed: the status line lives in an external
tracking repository this crate has no access to and cannot reach from a
standalone checkout, so the status line is not this crate's to change.

**Disposition:** declined — the remedy is a status-line correction in
this crate's own cited feature record, which lives in that external tracking
repository, outside this repo's own
`src/`, `docs/`, `Cargo.toml`; there is no file in this crate that can carry
the fix.

---

### IX14 — The Family Has a Measurement Crate, and It Does Not Measure This

```sh
cd "$(git rev-parse --show-toplevel)"
echo "  ring_bench src lines:  $( wc -l < ring_bench/src/lib.rs )"
echo "  its public functions:  $( command grep -c '^\s*pub \(const \)\?fn' ring_bench/src/lib.rs )"
echo "  its mask/modulo/ring_index mentions: $( command grep -c 'mask\|modulo\|ring_index' ring_bench/src/lib.rs )"
echo "  benches/ dirs across the family:     $( ls -d ring_*/benches 2>/dev/null | wc -l )"
```

Live output:

```
  ring_bench src lines:  1450
  its public functions:  40
  its mask/modulo/ring_index mentions: 0
  benches/ dirs across the family:     0
```

(This crate's own cited feature record also names the crate meant to own this
measurement — a pointer into the same external, now-unreachable repository as
IX13's status field — but that pointer is not reproduced here since it cannot
be verified from a standalone checkout.)

**Finding.** This crate's own cited feature record promises that the masking claim
"gets measured rather than assumed," and the family does have somewhere to put
such a measurement — `ring_bench` is 1450 lines with 40 public functions and a
runnable `examples/comparison.rs`, and its own module comment says one sibling feature record
"says plainly that *the comparison is the deliverable*."

It compares whole write paths: mutex, ring, thread-local staging. At that
granularity a fold costing 0.9 cycles against a modulo costing 6.2 is invisible
— it is a rounding error inside a workload dominated by contention and cache
traffic. So the crate that exists to produce numbers produces the wrong number
for this claim, and mentions `mask`, `modulo` and `ring_index` exactly zero
times.

The gap is not "nobody built a harness." It is that the harness built is a
macro-benchmark and the promise was a micro-claim, and no one noticed the two
do not meet. The numbers in
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_fold_costs.md)
were taken with a throwaway crate outside the workspace and deleted afterward —
enough to record a finding, not enough to hold a regression.

---

### IX15 — The Module Comment's "Every Claim" Error Came From the Feature

[`integration/001`](001_two_dependents_and_a_third_that_did_it_again.md) IX11
records that the module comment claims the fold "sits on the claim path and the
read path of every single operation the family performs," and that no crate on
the claim path depends on this one. The sentence has a source:

This crate's own cited feature record read, as captured before this crate's
extraction into its own repository (it is not independently reachable from a
standalone checkout, so this is a preserved quote rather than a reproducible
recipe):

> Either the address calculation costs a modulo on every claim and every read, or the capacity is left unbounded and the ring stops being a ring — a queue that grows under load rather than applying a policy.

**Finding.** "On every claim and every read" is that record's phrasing;
"the claim path and the read path of every single operation" is the crate's. The
error was inherited, not introduced — the module comment is faithfully
restating a premise that was already wrong when it was written, before any of
these crates existed to contradict it.

That changes what the finding is worth. A crate-local slip would be a typo to
fix in one line. This is a claim that survived from that external record into
implementation prose because restating it was the correct thing to do at the
time, and nothing since has re-checked it against the manifests that now exist.
The correction belongs in both places, and only one of them is in this repo.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_index
command grep -F 'it sits on every operation that touches a slot' src/lib.rs
```

Live output:

```
//! current x86, and it sits on every operation that touches a slot. The claim
```

**Disposition:** applied — the half of this claim that lives in this repo is
fixed: the module comment no longer restates that record's inherited "every
claim and every read" error, and instead states the narrower claim the
manifests support. The other half, in
this crate's own cited feature record, is outside
this repo (see `integration/001` IX13's declined disposition, same external-repository
boundary) and is not touched here.
Now prints: `it sits on every operation that touches a slot`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/001`](001_two_dependents_and_a_third_that_did_it_again.md) | The manifest evidence that contradicts the inherited claim |
| [`non_functional_requirement/001`](../non_functional_requirement/001_what_the_fold_costs.md) | The measurement the feature asked for, taken once and not kept |
| [`decisions/001`](../decisions/001_a_power_of_two_or_nothing.md) | The constraint the feature imposes, and what it costs a caller |

### Sources

| Fact | Where |
|------|-------|
| Feature status and definition | Census above |
| The crate's citation of it | `ring_index/src/lib.rs:1,7` |
| The measurement promise | Census above |
| `ring_bench`'s size and blind spot | `ring_bench/src/lib.rs` — census above |
| Zero benchmarking infrastructure | Census above |
| The inherited "every claim" phrasing | Census above |

### Tests

| Test | Covers |
|------|--------|
| `non_power_of_two_capacity_is_rejected_upstream` | The feature's power-of-two obligation, asserted from this crate |
| `mask_equals_modulo_over_four_laps_of_every_capacity` | The feature's derivation obligation |
| *(to create)* | Nothing testable — the open findings here are a status line in another repository and a benchmark harness aimed one level too high |
