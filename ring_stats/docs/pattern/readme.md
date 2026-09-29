# pattern

Two shapes organise this crate, and they explain most of what the rest of the corpus
finds. The first is a mirror: one writer and one reader per counter, named after the
counter, parameterised identically where the counters come in threes. It is exact —
five writers, five counter-naming readers, the same three-arm `match` written out on
both sides — and its exactness is a real property, because no counter here can drift
into being write-only.

The second is a fold: load several counters inside an accessor and return one number.
Two methods use it, and they are the two the crate itself puts forward as the readings
worth taking. The same workspace has two other shapes for the same problem, and both
are structurally incapable of the failure this one has.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_record_and_read_pair.md) | The Record-and-Read Pair | The mirror, its exactness, and the distinction it spent to stay uniform |
| [002](002_the_derived_reading_from_separate_loads.md) | The Derived Reading From Separate Loads | The fold, against the parameterised and snapshot forms the family already has |

## A Mirror That Cost One Distinction

The pair pattern needs one writer and one reader per family of counters, so all three
`OverflowPolicy` variants go under one verb: `record_drop` writes them and `dropped`
reads them. Two of the three genuinely are drops. The third is `Fail`, where the ring
refuses the push and hands the item back, losing nothing — and `ring_types` exposes
exactly that distinction as a `const fn`, `reports_failure`, true for one variant of
three.

The struct field is named `failed`. Both of its accessors say drop. `dropped_total`
folds all three, so a ring configured `Fail` that refused a thousand pushes reports a
thousand drops. The pattern bought uniformity and paid for it in the one distinction
the domain actually draws.

## A Fold, Where the Workspace Already Has Two Better Shapes

`ring_seqno::free_slots` takes its values as parameters and performs no atomic operation
at all — the caller owns the moment, visibly, at the call site. It names its own
intermediate `in_flight`, computing four crates away the reading this crate is named
for. `ring_atomic::CountingSeq::counts` loads four counters into locals and derives
`total` from those locals, so the whole always agrees with the parts it is handed back
beside.

`ring_stats` loads inside the accessor and returns the fold alone. It is the only one
of the three shapes that can return a number no state ever held while also withholding
the components that would show it. And it is applied to exactly the two readings a
monitor would display: `in_flight`, which the module comment holds up as the leak
detector, and `dropped_total`, the single number answering whether the ring is losing
traffic.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the mirror --'
command grep -o 'pub fn record_[a-z_]*' ring_stats/src/lib.rs
command grep -o 'pub fn \(claimed\|published\|consumed\|dropped\|wait_nanos\)\b' ring_stats/src/lib.rs
echo '  -- the two readers outside it --'
command grep -n 'pub fn dropped_total\|pub fn in_flight' ring_stats/src/lib.rs
echo '  -- and the two other shapes the family has for the same problem --'
command grep -n 'pub fn free_slots' ring_seqno/src/lib.rs
command grep -n 'pub fn counts( &self ) -> OpCounts' ring_atomic/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| ST37 | `ring_stats` | n/a — observation | Five recorders and five counter-naming readers correspond exactly, including `record_drop( policy, n )` against `dropped( policy )` with the same three-arm `match` written out byte-for-byte on both sides — so no counter can be written and not read or read and not written, and the two readers that name no counter stand out against the mirror rather than needing to be discovered |
| ST38 | `ring_stats` | **misleading doc** | The mirror requires one verb per counter family, so all three `OverflowPolicy` variants are written by `record_drop` and read by `dropped` — including `Fail`, where the ring refuses the push and loses nothing, whose field is honestly named `failed` and whose accessors both say drop, which is what lets `dropped_total` report a thousand refusals as a thousand losses while `ring_types::reports_failure` sits one call away |
| ST39 | `ring_stats` | n/a — observation | The workspace solves "derive a reading from several atomics" three ways — `ring_seqno::free_slots` takes the values as parameters and performs no load, `ring_atomic::counts` derives `total` from its own four locals so the whole always matches the parts returned with it, and `ring_stats` loads inside the accessor and returns the fold alone — and only the third can return a value no state held while withholding the components that would show it; `RingStats::snapshot` now offers the second shape beside it, leaving the crate with both and the accessors unchanged |
| ST40 | `ring_stats` | n/a — doc gap | The five exact single-load readers are the ones nobody looks at, and the two derived ones are exactly what the crate puts forward — `in_flight` as the leak detector its module comment describes, `dropped_total` as the number a monitor prints — while the ordering rationale reasons about "a stats read" in the singular, which is the right unit for the five and the wrong one for the two |
