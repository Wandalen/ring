# gate

The stage gates. Each exits 0 (REACHED) or non-zero (NOT REACHED);
`run_all.sh` reports one line per gate and exits 0 only when all of them pass,
which is a plan's Final Goal reached-test.

Every gate pairs its assertion with a **non-vacuity check**, because the
workspace baseline showed why that is necessary: on the untouched skeletons,
`cargo tarpaulin` reported *No coverable lines found* and `#![deny(missing_docs)]`
passed in all 33 crates. Both gates were green before a single line of the
family existed. A gate that cannot tell "not started" from "finished" measures
nothing, so each one below also asserts that there is something to measure.

## Families

The gates grade one **family** — a declared list of crates, features, and the
subset of gates that applies to it. Which family a bare run grades is declared
in `declared/family.txt`; each family has a directory beside it.

```bash
bash bench_harness/gate/run_all.sh                  # the declared default family
bash bench_harness/gate/run_all.sh --family ring    # another family
bash bench_harness/gate/run_all.sh g2               # one gate by id
bash bench_harness/gate/run_all.sh --stage S1       # one stage's own scope
bash bench_harness/gate/run_all.sh --every          # G18 and G19 plus every family
```

Every run echoes the family and the gate list it loaded, so a gate quietly
dropped from a family shows up in that family's own output rather than only in
a diff of its declaration.

`--every` is the whole-repository reading, and the only one of these that is not
about a single family: G18 and G19 once each, then one block per directory under
`declared/`, with a status derived from how many blocks came up short rather than
from whichever family happened to run last. Those two are the preamble because
neither belongs to a family — G18 reads coverage *across* families, G19 reads a
document that spans every family this repository could ever declare — so neither appears in any
`gates.txt` and neither would otherwise run at all. It exists because that reading
used to be assembled by hand — one invocation per family, G18 standalone, then
arithmetic —
and a reading assembled by hand is a reading a family can be silently dropped
from. The family list is obtained by listing `declared/` at run time and appears
nowhere in the script; a hardcoded list would reintroduce the same omission one
directory at a time, and a family added tomorrow would go missing from the total
without anything saying so. It is slow for the same reason `g1` is — it runs
every gate of every family — so launch it detached.

Membership is an explicit list rather than a bare `<prefix>_*` glob. An
earlier design replaced the glob after measuring what it costs: a glob reports
whatever is on disk, so it cannot distinguish "the family is complete" from
"the family does not exist", and G5 and G6 — both conjunctions over a declared
set — read REACHED against a family with zero crates in it. `common.sh`'s
`assert_declared_crates_exist` now runs before every gate's own assertion, so a
declaration naming a crate that is not in the tree fails loudly. A list also
admits members that no prefix would catch: a family's own demo binary is the
crate that grades it and belongs inside it despite not sharing its prefix.

`g1` runs `cargo tarpaulin` and is slow — launch it detached rather than in the
foreground.

`g12` is the only gate that **writes to `src/`**. It reinstates a recorded
defect, runs that crate's suite, restores the file, and verifies byte-identical
restoration before moving on; a trap restores on any exit, including a
`Ctrl-C` or a killed test run. Two consequences worth knowing before running it:
it must not overlap a concurrent build of the crates it mutates, in either
direction, and if it ever reports a restoration mismatch, believe it — that
message means a file in the tree is still carrying a deliberate defect.

## What each gate proves it ran

The distinction that matters for a baseline reading is *not reached* versus
*did not run*, and only the first is a verdict. G1 states both cases
separately: "tarpaulin ran and reports no coverable lines" fires on tarpaulin's
own message and therefore proves execution, while a zero from G1's per-file
regex is only trusted after the output is confirmed to contain a coverage
report at all. G2, G3, G4, G7 and G8 print counts, so their messages carry the
same evidence. G5 and G6 are conjunctions and carry it in their non-vacuity
halves — a declared export crate that exports nothing fails G5, and a family
crate that does not inherit the workspace lint table fails G6.

## The one thing here that is not a gate

`mutant_survey.sh` produces no verdict and belongs to no family's gate list. It
exists because G12 can only replay defects somebody already found, so a crate
whose blind spot nobody has hit yet reads exactly like a crate with none. The
survey breaks a crate every way `cargo mutants` knows how and reports what the
suite failed to notice; each survivor is a candidate for recording as a G12
declaration, and deciding whether one is worth defending stays a judgement.
Run it when a crate's tests change shape, not on every gate run — its output is
a reading to act on rather than a threshold to hold.

**"When a crate's tests change shape" is now G13's job to notice rather than
yours.** That sentence was advice for as long as it was only a sentence, and it
failed the first time it was tested: the 2026-08-30 triage added eleven
assertions across eight crates, and the re-sweep confirming them ran because
somebody remembered it should. So the survey now writes a `surveyed/<crate>.surveyed`
record on a clean run — the date, plus a digest of the `src/` and `tests/` it
swept — and G13 fails when a digest stops matching the crate on disk.

The split is deliberate and is the whole reason this can be a gate at all.
Whether a survivor is a defect or a decision is a judgement, and judgements do
not belong on a board that exits 0 or 1; that half stays here, unautomated.
Whether a settled judgement is still about the current code is a digest
comparison, and there was never a good reason for a person to be the one making
it.

```bash
bash bench_harness/gate/mutant_survey.sh ring_bench        # one crate
bash bench_harness/gate/mutant_survey.sh --family ring     # the whole family
```

**The whole ring family is 39 minutes cold, 32 warm.** That figure is worth
knowing because the cost was overestimated for as long as it was unmeasured —
`--in-place` refuses `--jobs`, so the sweep is serial and no other work may touch
the tree while it runs, which reads as expensive until someone times it. 1,160
mutations across 33 crates, 29 of them surviving. Sweeping is a scheduling
decision, not a budget one.

What does not shrink is the triage. A survivor is not a defect: of those 29,
eleven were real and eighteen were mutations no test could ever kill — a
`free_capacity` that already answers zero for every reachable input, a padding
check that is true by the alignment its own crate asserts. The tool reports the
mutation and stops; every survivor costs a human read of the source, and that
half of the bill is the same whether the sweep took forty minutes or four.

Its exits are 0 (nothing new), 1 (new survivors), 2 (**the tree did not come
back** — believe it, same as G12's restoration mismatch), and 3 (nothing new,
but an acceptance in `declared/<family>/accepted/` no longer matches anything).

**Three things it got wrong first, all worth knowing before trusting a run.**

It surveys with `--all-features`, because `cargo mutants` runs the suite with
default features and mutates source text — so a mutation inside a
`#[ cfg( feature = ... ) ]` body that is off by default compiles as dead code
and passes every time, however good the tests are. Without the flag, one
`ring_bench` run reported 22 survivors of which 18 were that artifact.

It subtracts the family's accepted survivors, because some things go unnoticed
on purpose — `ring_bench` refuses to assert on a clock, so timing-only
mutations survive it permanently and correctly. A survey that re-presents
settled rulings as fresh findings on every sweep stops being read, which costs
more than it saves.

And it clears its output directory before every run, which is the one of the
three that was actually dangerous. `cargo mutants` rotates its previous results
only once it produces new ones, so a run that dies on the baseline build leaves
the **previous crate's** results in place — and the survey read them, and
reported them under the new crate's name, in zero seconds, with a confident
exit 1. A deliberately broken `ring_types` baseline reported `ring_bench`'s
three survivors while every check in the script passed: the tree hash matched,
because nothing had moved, and the mutant count was healthy, because it was
counting the previous run.

All three are the failure this file opens with — a check that cannot tell "not
started" from "finished" measures nothing — and the third is its worst form,
because a vacuous result at least looks empty. **A stale one arrives carrying
evidence.**

| File | Responsibility |
|------|-----------------|
| `common.sh` | Family resolution, declaration readers, and pass/fail reporting — sourced, not executed |
| `declared/` | Per-family declarations the gates read — see [declared/readme.md](declared/readme.md) |
| `g1_coverage.sh` | 100% line coverage over the family, and coverable lines > 0 |
| `g2_docs.sh` | Zero `missing_docs`, every crate exports a public item, none still calls itself a skeleton |
| `g3_features.sh` | Every declared feature claimed by a test that cites it |
| `g4_manual.sh` | Every crate carries a dated manual run record |
| `g5_export_surface.sh` | Only declared crates are reachable from outside the family, and they export something |
| `g6_unsafe.sh` | The workspace unsafe deny is inherited, and opt-outs are declared and justified |
| `g7_determinism.sh` | Every declared smoke binary moves when its declared input moves, and is byte-identical run to run, debug to release, and across a second instruction set |
| `g8_oracle_form.sh` | No reached-test asserts against a recorded literal rather than a derived oracle |
| `g9_lint.sh` | Clippy is clean over every crate in scope, tests included |
| `g10_pinned_math.sh` | No unpinned transcendental on the deterministic path, repo-wide — called directly or reached through a math-library helper |
| `g11_cgtools_exclusive.sh` | No third-party math dependency and no hand-rolled vector type, across every crate root this repository scans |
| `g12_mutation.sh` | Every recorded historical defect, reinstated in place, still turns its crate's suite red |
| `g13_survey_freshness.sh` | Every crate swept clean, and swept against the source and tests it carries now |
| `g14_corpus_shape.sh` | The declared doc definitions, the instance and finding floors, and the three counts agreeing |
| `g15_corpus_recipes.sh` | Every published recipe still prints what its document quotes |
| `g16_corpus_citations.sh` | Links resolve, cited tests exist, and the two findings tables agree |
| `g17_corpus_vocabulary.sh` | Tier strings from the declared set; finding ids unique and contiguous |
| `g18_family_coverage.sh` | Every crate claimed by exactly one family, or exempted with a reason — read across families, so in no family's `gates.txt` |
| `g19_measured_columns.sh` | Every family Overview Table column the Legend calls measured still equals the tree — reports disagreements, never edits the table; spans every family, so in no family's `gates.txt` |
| `g22_exemption_expiry.sh` | Every exemption still earned — fails on one whose crate has crossed the Substance Threshold; grades the shared `exempt.txt`, so in no family's `gates.txt` |
| `corpus/` | The checkers G14-G17 and G20-G21 run, and the scoreboard that renders their counts — see [corpus/readme.md](corpus/readme.md) |
| `mutant_survey.sh` | Proposes defect candidates mechanically — reports mutations the suite failed to notice |
| `run_all.sh` | Runs every declared gate, or the named subset, and aggregates the verdict |
