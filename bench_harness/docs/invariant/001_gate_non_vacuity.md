# Invariant: Gate Non-Vacuity

- **Status:** current
- **Tags:** layer:substrate

### Scope

- **Purpose**: Pin the property that makes the stage gates worth running at all — that each one can distinguish an implemented family from an unimplemented one, rather than reporting success because it measures nothing.
- **Responsibility**: The baseline measurement, the three ways a gate goes vacuous, G12's differently-shaped baseline, which gates were vacuous and how each was retired (G6, then G5), and the command that says when each stops being so.
- **In Scope**: The gates under `gate/` declared for the ring family and their reached/not-reached verdicts.
- **Out of Scope**: What each feature must satisfy to count as delivered (→ [`acceptance/001`](../acceptance/001_feature_reached_tests.md)).

### Invariant Statement

Every gate declared for the ring family reports NOT REACHED when that family is
unimplemented. Formally: with all 33 crates at skeleton state, `run_all.sh
--family ring` exits non-zero and reports **0 of 6** gates reached — the reading
taken 2026-08-28, when the family declared six.

G12, added 2026-08-29, satisfies the same property but cannot be measured
against skeletons at all; its own baseline is recorded under
[G12's baseline is a different shape](#g12s-baseline-is-a-different-shape-and-had-to-be-proved-separately)
below.

### Rationale

The family baseline taken 2026-08-28 against the untouched skeletons
measured this:

| Measure | Reading |
|---------|---------|
| `cargo tarpaulin` line coverage | `No coverable lines found` |
| `#![deny(missing_docs)]` | passes in all 33 crates |
| `cargo nextest run` | 0 tests across 33 binaries |

Both the coverage gate and the documentation gate were **green before a single
line of the family existed**. A gate in that state does not distinguish "not
started" from "finished", so it cannot carry a stage's reached-verdict — it is a
ratchet that only bites later, not a test of the goal.

Each gate therefore pairs its assertion with a non-vacuity check on the same
subject: coverage is paired with *coverable lines > 0*; documentation with
*every crate exports at least one public item*; feature claims with *the count
is exactly 22*; manual records with *the count is exactly 33*; the export
surface and unsafe allowlist with *a declaration exists at all*.

### Enforcement Mechanism

```bash
bash bench_harness/gate/run_all.sh --family ring
```

- **Unit:** gates reached, out of however many `declared/ring/gates.txt` names — 6 when this invariant was written, 7 when G12 landed, 14 as of 2026-09-07
- **Required reading while the family is unimplemented:** `0/N gates reached`, exit 1
- **Required reading at the Final Goal:** `6/6 gates reached`, exit 0 — the reading actually taken on 2026-08-28, before G12 existed

`--family ring` is required rather than optional. A bare `run_all.sh` grades
whichever family `declared/family.txt` names, and that is now `orbital` — an
unrelated grading target, eleven different gates, and a green board that says
nothing about this invariant's subject.

`g1` invokes `cargo tarpaulin` and is slow; launch it detached rather than in
the foreground.

### Violation Consequences

A gate that passes on skeletons is the failure this invariant exists to catch.
One was found and fixed during construction: `g2_docs.sh` counted public items
with the pattern `pub(`, which also matches `pub(crate)` and `pub(super)` —
both crate-private. A crate exporting nothing outside itself would have
satisfied the non-vacuity check. The pattern is now `pub ` (trailing
whitespace), verified by adding a `pub(crate) fn` to a skeleton crate and
confirming the gate still reported that crate as exporting no public item.

#### The opposite failure: a gate that can never pass

Non-vacuity has a mirror image, found at the S1 verdict. `g1_coverage.sh` read
the summary percentage `cargo tarpaulin` prints, which is workspace-wide:
`-p` selects which packages' *tests to run*, not which sources to *report on*.
Unrelated crates elsewhere in the workspace therefore sat in the
denominator, and the gate reported `26.83% over 723 lines` at the moment the
ring family was in fact at 194/194. No amount of work on this family could
have moved it to 100%.

A gate that cannot pass is as useless as one that cannot fail: both stop
distinguishing states of the thing under test. The gate now recomputes from
tarpaulin's own per-file `Tested/Total Lines` breakdown, restricted by name to
the crates in scope — the family's `declared/<family>/crates.txt`, or one
stage's subset under a stage run — under either crate root, and names the
specific files that are short when it fails.
Verified by re-running: `100% line coverage over 194 coverable ring-family
lines`, against the same tarpaulin output that had reported 26.83%.

Note this fix does **not** weaken the non-vacuity pairing above. The
`No coverable lines found` check still fires on skeletons, and the
`total > 0` check now reads the family's own line count rather than the
workspace's — so an all-skeleton family gives `zero coverable lines in the ring
family` rather than borrowing non-vacuity from unrelated crates that happen to
have code.

#### A third failure: a gate with no instrument pointed at the artifact

Both defects above were the gate mis-reading its instrument. The one found at
S3 is worse, because there was nothing to mis-read.

G2 asserted documentation coverage through two checks — zero `missing_docs`
diagnostics, and at least one public item per crate. `missing_docs` reads
rustdoc comments. The item count reads code. Neither reads `readme.md`. So
every one of the 14 crates completed in S1 and S2 opened with **"Skeleton — no
implementation yet"** while carrying a full test suite, a manual plan, and
100% coverage — false in the first sentence a human reads, and true nowhere any
check had ever looked.

That the crates were *documented* was measured. That their documentation was
*accurate* was not, and the distinction is not a subtlety: a gate can be
perfectly non-vacuous about the wrong subject.

G2 now fails when a crate exporting public items still describes itself as
unimplemented, in its readme, its `docs/`, or its module header. Verified by
running it before the fix — `8 implemented crate(s)` for S1, `6` for S2 — and
after, with both stages re-verdicting at 6/6.

#### G12's baseline is a different shape, and had to be proved separately

The six gates above take their non-vacuity from the skeleton baseline: run them
against an unimplemented family and they report 0/6. **G12 cannot be baselined
that way.** On skeletons there is no code to mutate, so its declared `from`
blocks would match nothing — the gate would fail, but for the wrong reason, and
a failure for the wrong reason proves nothing about the right one.

So G12's two failure modes were provoked directly, on the finished family,
before the gate was declared 2026-08-29:

| Probe | Provoked | Reading |
|---|---|---|
| **Stale declaration** | a `from` block edited to match nothing in its target | `NOT REACHED G12 — PROBE-STALE no longer matches ring_bench/src/lib.rs`, exit 1 |
| **Blind mutation** | a mutation that edits only a doc comment, which no test can observe | `NOT REACHED G12 — 1/4 reinstated defect(s) left the suite green: PROBE-BLIND`, exit 1 |

The second is the one that matters. **A mutation the suite ignores must fail the
gate, not pass it quietly** — the whole point of G12 is that a green suite under
a reinstated defect is the finding, so a probe the tests cannot see has to be
reported rather than skipped. The first guards the input: a declaration that
stops matching its target as the code moves would otherwise silently reduce the
gate's mutant set, and a gate that grades fewer defects each time the code
changes decays toward vacuity without ever printing a different number.

Both probe files were hyphen-prefixed, confirmed untracked, and deleted after
the readings were taken.

### Known Vacuity, Both Since Retired

**Both have since been retired from this list — G6 first, G5 after.**

| Gate | Reads | Status |
|------|-------|--------|
| G5 | external dependencies confined to the 5 declared crates | **Retired 2026-09-07.** `smoke_ring_write_path` names `ring_config` and `ring_core` from outside the family, so the confinement now confines something real. |
| G6 | unsafe opt-outs confined to the 3 declared crates | **Retired 2026-08-29.** `ring_spsc` and `ring_mpsc` both carry `#![ allow( unsafe_code ) ]` in `src/lib.rs`, so the confinement now confines something real. |

Both are recorded here rather than suppressed, because a gate that is vacuous
*for now* and one that is vacuous *by construction* look identical in the run
output and only this note distinguishes them — including after retirement, since
"was vacuous, now is not" is itself a reading a run cannot give you.

#### Retirement, as measured rather than as predicted

This note first said both became real at S2 and S7. S2 is complete and neither
did, so the prediction is replaced with the measurement and the command that
takes it.

```bash
# G5 becomes real when this prints anything
find . -name Cargo.toml -not -path '*/target/*' -not -path './ring_*' -not -path '*/.claude/worktrees/*' \
  | xargs grep -lE '^\s*ring_[a-z_]+ = '
# G6 becomes real when this prints a source file under ring_*/src
grep -rl "allow( *unsafe_code" ring_*/src
```

| Gate | Measured 2026-08-28 | What would actually retire it, and whether it has |
|------|---------------------|--------------------------------|
| G5 | 0 external consumers | The first crate outside `ring_*` naming a ring crate. Not S2: S2 added six *internal* crates. `ring_bench` (S8) does not count either — it is inside the family and the gate skips it by name — its own directory is on `declared/ring/crates.txt`. On the current plan that is the wider effort's own consumer, after S9. |
| G6 | 0 crates carrying the opt-out | **Retired.** The first crate that genuinely needs `unsafe` — which arrived. See the correction below. |

#### G6's prediction was wrong, and the way it was wrong is the lesson

This note previously argued that no crate would ever need `unsafe`, that G6 was
therefore vacuous by construction, and that the allowlist should shrink to
empty rather than be kept as decoration. **Two of the three allowlisted crates
now carry the opt-out**, so the prediction was wrong and the gate is real:

```bash
grep -rn "allow( *unsafe_code" ring_*/src/
# ring_mpsc/src/lib.rs:191:#![ allow( unsafe_code ) ]
# ring_spsc/src/lib.rs:169:#![ allow( unsafe_code ) ]
```

The prediction also named the wrong crates. It listed `ring_atomic`,
`ring_store`, `ring_slot` and `ring_align` as "the four allowlisted crates".
All four exist, and **none of them is on the allowlist** — which has three
entries, a different set:

```bash
grep -vE '^\s*(#|$)' bench_harness/gate/declared/ring/unsafe_allowlist.txt
# ring_spsc
# ring_mpsc
# ring_core
```

That is the more useful half of the correction. This note is the only thing
distinguishing the two kinds of vacuity above — so a note naming crates that
were never on the allowlist was not a weaker version of that mechanism, it was
the mechanism reporting on a set that did not exist. Both readings above are
commands rather than claims, for exactly that reason.

**One entry is unexercised, and it is a decision rather than an oversight.**
`ring_core` is declared but carries no `unsafe` anywhere under `src/`:

```bash
grep -rn 'unsafe' ring_core/src/   # prints nothing
```

That is the same state that earlier struck four other names off this list —
an entry for a crate carrying no `unsafe` is a permission nobody exercises and
a bound looser than the code actually is. But `ring_core` was put on
deliberately, on a different basis: it assembles a complete ring — storage
plus the cursors that bound it — and siting the opt-out with the whole
soundness invariant is the point of the list.

**So the two principles pull opposite ways on this one name**, one empirical and
one architectural, and neither was reconciled with the other when the allowlist was set.

This note previously said G6 passed either way — that the gate's own check ran
only in one direction (is every crate *carrying* the opt-out attribute within
the declared set), never the other (is every *declared* entry still carrying
one), so the tension above sat invisible to the gate regardless of which
reading was correct. That asymmetry was itself a gap in G6, independent of
which principle should win: a declared crate that stopped needing its exemption
would never be noticed either way. `g6_unsafe.sh` now checks both directions,
and as of that fix `ring_core`'s entry — declared, unexercised — is exactly
what the added direction catches: **G6 currently reports NOT REACHED for
ring**, naming `ring_core` as a stale allowlist entry. Resolving it still means
overturning or refining a current decision, which is a decision's job and not
this note's or the gate's — but it is no longer a decision the gate can leave
unmade. Recorded here so the tension is visible rather than rediscovered, and
so the current NOT REACHED reading is not mistaken for a new regression when
its cause has been on record since before the gate could see it.

**G5's prediction came true, and that gate is now real too.** The row above
named "the wider effort's own consumer, after S9" as the thing that would
retire G5. That consumer arrived: `smoke_ring_write_path` names `ring_config`
and `ring_core` from outside the family and is absent from
`declared/ring/crates.txt`, so the gate counts it rather than skipping it, and
reads NOT REACHED. It reports six hits rather than two, because four are
`.claude/worktrees/` copies of that same manifest which the gate does not
exclude — a separate defect, and not the reason the gate became real.

Until a gate is retired or made real, its REACHED reading carries no evidence,
and a stage verdict must not cite it as supporting. **That no longer applies
to either.**

### Related

- [`acceptance/001`](../acceptance/001_feature_reached_tests.md) — what each graded feature must satisfy, over the crates these gates measure
- [`gate/readme.md`](../../gate/readme.md) — the gates themselves
