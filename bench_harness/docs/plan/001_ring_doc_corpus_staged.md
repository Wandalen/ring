# Plan: ring doc corpus, staged validation

- **Status:** current
- **Tags:** layer:substrate, activity:process, theme:validation_instrumentation, theme:documentation_corpus, theme:scope_workstream_planning

## Definition

The execution plan for bringing all 33 `ring_*` crates to
the doc-corpus standard of thirteen doc definitions, twenty-six doc instances
and fifty-two recorded findings per crate. It is driven through
`tsk/pln_staged.rulebook.md § Machinery : Procedure - Staged Loop`.

Distinct from the family's own implementation plan, which is closed 9/9. That
plan graded code: line coverage, feature claims, exports, `unsafe`
opt-outs, mutation survival. Its one documentation gate, G2, grades rustdoc:
`missing_docs` diagnostics, public-item counts, doctests that run. It reads
exactly three files per crate by deliberate narrowing: `readme.md`,
`docs/readme.md`, `src/lib.rs`. **The `docs/` tree itself is ungated today.**
752 instances and 776 findings across the family have never been run, resolved,
or counted by anything but a person.

Three of those counts were wrong when this plan was first written, and the
machinery corrected them on its first run. The hand census had read 418
definitions, 792 instances and 778 findings by counting `definition/` as a
fourteenth definition and readmes as instances. That is the ordinary reason for
§ Machinery First and is recorded here rather than quietly overwritten.

That is the gap this plan closes, and it is a different kind of gap from the
implementation plan's. That plan's stages asked whether the code does what it claims. These ask
whether the corpus **can still be checked at all**: whether every published
recipe still reproduces, every cited test still exists, and every count still
agrees with the tree it describes.

## Scope

Committed, in order:

1. **Machinery before the corpus it grades (M0–M2).** Four gates promoted into
   `gate/`, a control corpus each must report as broken, and the standard itself
   written where the gates read it rather than restated in prose beside them.
2. **The twenty incomplete crates (S1–S6)**, grouped by *why* each is
   incomplete rather than by name: the findings layer never written over
   instances that are complete, a pass abandoned mid-crate, the vocabulary every
   other crate imports, instances short against a moving implementation, the
   family's public contract undocumented, and four skeleton corpora.

   **S1's four crates are not thin.** `ring_align` carries 4,121 lines of
   instance text against `ring_atomic`'s 4,438, and `ring_atomic` is at
   standard. This was measured after S1 began, and it corrects what this plan
   assumed. What those four lack is the findings table over the instances, not
   the instances. That makes S1 an extraction rather than an investigation, and
   it moves the difficulty. The hard part is no longer finding 52 things to say but
   declining to write the ones already said one paragraph above.
3. **The five axes a per-crate count cannot see (S7–S11).** Source corrections
   acted on, family-scope claims checked across all 33, coverage claims
   re-verified, tasks split per finding, and every prose numeral stated over a
   regenerate block's output made to come *from* that block.
4. **Final Goal (S12).** 33/33 in one run, every control arm still failing, and
   the Closing Reading.

Out of scope, and excluded deliberately rather than deferred: line and mutation
coverage of the 33 crates (the implementation plan's G1 and G12, both closed); the write path's
own correctness; and any judgement about whether a recorded finding is
*interesting*. The third exclusion is the one the plan rests on, and § What the
gate cannot decide states why.

## The asymmetry rule

A stage earns its place only if reaching it is hard and checking it is cheap. A
stage whose goal is "the directory exists and the file count is right" is
rejected. That is the count a fresh `mkdir` satisfies.

**Every stage is graded on two axes at once.** Presence (are the definitions,
instances and findings there) and **reproducibility** (does every recipe in them
still print what it says it prints). The second is what makes the goal difficult.
A finding is cheap to write and expensive to keep true, because the regenerate
block beneath it re-runs against the tree on every gate run and disagrees the
moment either side moves.

Four oracle classes cover the reproducibility axis, none requiring a recorded
golden value:

1. **Re-execution.** Every ```sh block runs; its stdout must equal the quoted
   block beneath it byte for byte.
2. **Resolution.** Every cited test name exists in the crate's own test files;
   every markdown link resolves to a file on disk.
3. **Agreement.** Each master-index row's Subject and Tier equal the owning
   definition readme's row for the same finding ID, character for character.
4. **Closed vocabulary.** Every tier string is a member of the declared set;
   every finding ID is unique and the set is contiguous from 1.

A quoted literal is admissible only where the artifact under test *is* the
literal, such as a compile probe's `error[E0599]` or a measured nanosecond
figure. Those blocks are exempt from re-execution because they carry no ```sh
block above them, the same mechanism that exempts them today.

## What the gate cannot decide

The repository's own conventions rulebook settles this, in the passage under
*Documentation : Executable Recipes* headed **"Not gated, deliberately"**.
Whether a doc states its expectation is a judgement about prose, not a property
a script can decide. That was measured directly twice: a detector tuned three
times against this corpus still misread three of six candidates.

That finding applies here with full force and is not routed around. This plan's
gates decide **structure and reproducibility**. They do not decide, and are not
claimed to decide:

| Question | Why no gate here answers it |
|---|---|
| Is this finding true? | Its regenerate block proves the *quoted output* is current, never that the prose above it reads that output correctly |
| Is this finding worth recording? | The distinction between a finding and a restatement of the paragraph above it is a reading-comprehension judgement |
| Is this tier the right tier? | The vocabulary is closed and checkable; the *choice* within it is not |
| Does this instance duplicate its sibling? | Two files can share a subject and differ entirely in what they establish |

The counterweight is not a better detector. It is that each stage's
§ Stage Table row names **how a competent attempt misses it**. That is the
specific shape of a pass that satisfies every mechanical check and leaves the corpus no
better. Those columns are the human triage the gates cannot host, written down
in advance rather than recalled at review time.

**One mechanical defence does exist, and it is worth stating rather than leaving
implicit.** S1–S6 are, on their presence axis alone, count gates, and the miss-mode
they share is a finding row that restates the paragraph above it. G15 raises the
cost of that row. Every finding sits under a definition readme whose
regenerate block must execute and match, so a finding with nothing measured behind
it has no output to quote. The restatement is still writable. It cannot bring
evidence, though, and the definition readme it lands in will have one fewer line
of regenerated output than its siblings. That is a signal a reader can see, not a
verdict a gate can take.

## Validation Machinery

Six gates, hosted in `bench_harness/gate/` alongside G1–G13 and declared
for the ring family in `declared/ring/gates.txt`. Four were promoted at M0; the
fifth and sixth, G20 and G21, were built at S7 for the reason § Machinery
First gives. The stage needed a verdict no existing gate could produce, and
then needed a second one for the obstacle the first one uncovered. Promoted from four
hyphen-prefixed scratchpad scripts that have graded thirteen crates by hand;
rewriting them here rather than reimplementing them is the Anti-Duplication
requirement, and the promotion is itself M0's deliverable.

| Gate | Asserts | Non-vacuity pairing |
|---|---|---|
| **G14** corpus shape | 13 definitions, ≥26 instances, ≥52 findings per crate, and the three counts agree between the tree, the definition readme's own regenerate block, and the Module Index | Must be proven able to fire: the control corpus holds 12 definitions and the gate must name the missing one, not merely exit non-zero |
| **G15** recipes reproduce | Every ```sh block in every instance runs and its stdout equals the quoted block beneath it | A gate that skips a block it cannot parse reports clean on the file it failed to read. The control seeds one stale quoted line, and the gate must print the diff |
| **G16** citations resolve | Every cited test name exists; every markdown link resolves; every master row's Subject and Tier equal its owner's | Two independent spellings of the same fact agreeing is the assertion; the control makes one row disagree in its Tier alone, which no link check would catch |
| **G20** disposition | Every finding on a reachable tier carries one `**Disposition:** applied — … Now prints: \`<literal>\`` or `**Disposition:** declined — <reason>` line inside its own section | The other four grade the corpus as a document and a family can pass all four with every hazard it names still present. This one asks whether anything was *done*, so its control arm is four seeded fixtures **plus a fifth that is clean**. At the moment it was written all 458 findings lacked a disposition, and a gate proven only able to fail is as untrustworthy as one proven only able to pass |
| **G21** addressing | No recipe extracts source by absolute line number. `sed -n 'N,Mp'` appears nowhere inside a ```sh fence | Its 757 live hits are its non-vacuity proof and were found rather than seeded. They are what one five-line source correction had to fight through, and 7 of the 33 crates already carry none, so the gate distinguishes rather than fails everything |
| **G17** vocabulary closed | Every tier string is in the declared set; finding IDs unique and contiguous | Its 24 live hits are its non-vacuity proof and were found rather than seeded: four hybrid `n/a — latent hazard` rows, two in a `no — ` prefix nobody declared, and eighteen using five record tiers the declaration does not carry |

**The control arm is the machinery, not a test of it.** Every one of these four
gates passes by finding nothing, and *Documentation : Executable Recipes* names
that shape precisely. An empty result means "the property holds" and "the command
is broken" equally well, and every way a recipe rots expresses itself as silence.
So `run_all.sh --control` runs each gate against
`declared/ring/corpus_control/`, a fixture carrying one seeded defect per gate,
and **a gate that reports REACHED against the control has failed** regardless of
what it reports against the family.

**Machinery parity.** M0–M2 are budgeted as their own deliverables. The six
gates plus the control corpus plus the declared standard are roughly a fifth of
this plan's stages for none of its findings, and that ratio is the point. G20
was budgeted the same way one stage later. It was written, seeded against five
fixtures and baselined before a single correction was applied.

## Stage Table

Every stage is one scoped run of
`bash bench_harness/gate/run_all.sh --family ring --stage <id> g14 g15 g16 g17`.

| # | Stage Goal | Crates | Reached when the run prints | How a competent attempt misses it |
|---|---|---|---|---|
| M0 | Machinery that can fail | n/a | All four gates run over 33 crates; **20 crates NOT REACHED**, 13 REACHED, and each gate names which crate and which count | The gates count directories, so 33 crates that already have `docs/` subdirectories all read REACHED on the first run. A gate that passes on arrival has graded nothing |
| M1 | A control corpus each gate reports as broken | n/a | `--control` reports NOT REACHED four times, each naming **its own** seeded defect | One fixture with four defects at once, so any gate failing for any reason reads as all four working. This proves something fires, never that each detector fires |
| M2 | The standard, written where the gate reads it | n/a | `declared/ring/corpus_standard.txt` supplies the 13 names, the closed tier set and the three thresholds; editing `52` to `53` flips all 13 closed crates to NOT REACHED | The standard is prose in a readme and the gate hard-codes the same numbers. That gives two sources that agree today and drift on the first edit to either |
| S1 | The findings layer for four crates that have the instances | `ring_align` `ring_cursor` `ring_gating` `ring_seqno` | 4 crates × 52 findings, 4 per definition; 26 instances each unchanged; every definition readme gains the `### Regenerate` block it lacks | Every row on a record tier. The extraction is honest work and produces no reachable finding at all, so the crate ends scannable and nothing is owed. The tier distribution, not the count, is where this stage is cheated |
| S2 | Three abandoned passes finished | `ring_publish` (46→52) `ring_wait` (24→52) `ring_barrier` (19→52) | 67 findings added; G17 clean on all three. Its 24 hits are exactly these crates, and the five undeclared record tiers are ruled on rather than absorbed | The findings left undone were left undone because they were hard; reaching 52 with six easy ones closes the count and leaves the corpus exactly as incomplete |
| S3 | The vocabulary every other crate imports | `ring_types` | +1 instance (25→26), 52 findings; every finding states what it costs the 32 dependents, not only this crate | Documented as a crate. `ring_types` has no behaviour of its own worth 52 findings. Every claim it can make is a claim about who imports it, and a corpus written without opening a dependent will fill 52 rows with derive lists |
| S4 | Four composed cores brought to standard | `ring_core` `ring_mpsc` `ring_spsc` `ring_tls` | +1 definition and +7 instances each (19→26), 52 findings each | The seven new instances are split out of the existing nineteen: 26 files, the same content, the count satisfied, nothing new documented |
| S5 | The family's public contract documented | `ring_factory` `ring_handle` `ring_flush` `ring_bench` | +4 definitions, **+22 instances** (not the +15 this row first claimed; see § Baseline Reading), 52 findings each; each crate's `api/` states what a consumer **outside** the family may rely on | Documented from the inside. The instances describe how each crate works and never state the export contract, which is the one question these four raise that the other 29 do not |
| S6 | The family's two definition vocabularies reconciled | `ring_debug` `ring_poll` `ring_testkit` `ring_shutdown`, and the `state_machine/` ruling across 12 | **+13 definitions** (not the +15 this row first claimed; measured 3/4/3/3 missing across `ring_debug`/`ring_poll`/`ring_testkit`/`ring_shutdown`), +62 instances, 52 findings each; and one ruling covering the 12 crates carrying `state_machine/`, the same 12 that carry no `item/` | `state_machine/` renamed to `lifecycle/` crate by crate to make each list match, merging what states exist with what happens in order. The split is family-wide and 12 crates deep, not a `ring_debug` quirk, so a per-crate rename is twelve unreviewed decisions wearing one name |
| S7 | The corrections acted on, not filed | all 33 | `G20 disposition (all reachable tiers): 0 problem(s)` over 33 crates. Every one of **458** findings on a reachable tier carries an applied disposition whose evidence a Live output block prints, or a decline naming something concrete. **Recursed into S7a–S7d, below** | The corrections land and the findings keep quoting the old source. The corpus goes stale in the same edit that acts on it, and every regenerate block over a corrected line now prints what its document does not say |
| S8 | Family-scope claims checked across 33 | all 33 | Every finding claiming *the only*, *all 33*, *none of*, *the family's* carries a regenerate block that counts across all 33 rather than asserting | The claims are checked by hand once and the gate greps the four phrases. A claim phrased "no other crate does this" carries none of them and passes untouched |
| S9 | Every coverage claim re-verified | all 33 | Each `n/a — coverage` finding either has its test written and the finding re-tiered, or carries a regenerate block printing **zero** for the absent test **paired with a control** over a behaviour the suite does cover, printing non-zero | The tests get written and the findings keep asserting the gap. That leaves 33 crates of documentation wrong in the direction that reads as thoroughness. And the unpaired form is worse: a must-be-empty grep whose crate was renamed reports the gap intact forever |
| S10 | Tasks split per finding | all 33 | Every reachable finding has a task; the gate cross-checks both ways, so a finding with no task and a task naming no finding each fail | One task per crate listing 52 findings as bullets: 33 tasks, every finding named, nothing independently claimable or closable |
| S11 | Prose counts derived, not asserted | all 33 | Every numeral a definition readme states *about* its own regenerate block's output is printed by that block rather than written beside it; 429 readmes, zero prose counts G15 cannot see | The numerals are deleted rather than derived. "The census above lists the implementations" is un-stale because it says nothing, and 429 files lose the one claim that made them checkable |
| S12 | Final Goal | all 33 | 33/33 REACHED in one run, `--control` still NOT REACHED four times, Closing Reading | The control arm is run before the last stage's edits rather than after, so it grades a fixture nobody perturbed |

**The arithmetic closes.** The family stands at **406 definitions, 752
instances, 776 findings**, 13 of 33 crates at standard. S1–S6 add 23
definitions, 106 instances and 951 findings, reaching **429 / 858 / 1727**
against a floor of 429 / 858 / 1716. Five already-closed crates that run past 52
carry the eleven-finding surplus.

### S7 recurses into four, by tier

S7 covers **458 findings**. One Reached Verdict spanning 458 corrections is a
**Batch Attribution**. It says the stage arrived and nothing about which of the
458 got it there, so S7 recurses. The split is by tier rather than by crate
because *what acting on a finding means* differs per tier, and a sub-stage whose
members are all the same kind of act has one miss-mode instead of four.

`disposition.py --tier` is what makes each sub-stage separately
gradable. The four scopes partition the 458 exactly, as 180 + 62 + 129 + 87,
and that is checked rather than assumed. An undeclared or record tier passed to
`--tier` exits 2 rather than reporting a clean run over an empty selection.

| # | Sub-stage | Tier | Findings | Acting on one means | How a competent attempt misses it |
|---|---|---|---|---|---|
| S7-0 | Corrections affordable at all | n/a | 757 recipes | Replace an absolute line address with a content anchor | The addresses are converted and the anchors chosen so loosely that they match in several places, so a recipe that used to print the wrong four lines now prints the right four and eleven others |
| S7a | The hazards fixed or accepted | `**latent hazard**` | 180 | Change the source, or record why the hazard is accepted | The accepts are written first because they are free, and the tier empties without a line of source changing |
| S7b | The costs recorded where they are paid | `**measured cost**` | 62 | Put the number where the caller pays it, or decline | The cost is copied into a doc comment and the recipe that measured it is not re-run, so the comment and the corpus disagree from the first edit |
| S7c | The misleading text made to mislead nobody | `**misleading doc**` | 129 | Rewrite the passage, or rule that the reading was uncharitable | Every one declined as "reads fine to me". This tier's findings are the ones that can be argued away without touching anything |
| S7d | The false statements corrected | `**wrong doc**` | 87 | Correct the statement | The sentence is deleted rather than corrected. A claim that is gone is not a claim that is right, and the finding that named it now points at nothing |

**S7-0 was not in the original recursion and was measured into it.** The split
above was written as four tier-scoped sub-stages and S7a began immediately, on
`ring_types`, whose single hazard was the smallest in the family. `Ring::capacity`
rebuilt a `Capacity` from the crossbeam queue and `expect`ed the result, putting
this crate's only fallible validation inside an infallible accessor. The fix is
five lines that carry the validated value in the variant, plus a test holding all
three backends to it.

Those five lines left **61 recipes stale across 18 crates**, and not one of the
61 disagreed with the source about anything. Every one was a line number that had
moved. G21's own baseline then measured the general case: **757 recipes address
source by absolute line number**, `sed -n 'N,Mp'`, across 26 of the 33 crates.

That is the whole of S7 in one measurement. 458 corrections against that coupling
would be a treadmill, not a stage. The corrections are cheap and the repairs
after them are not, and a corpus that charges sixty repairs for one fix is a
corpus whose defects will not get fixed. So S7-0 goes first and the four tiers
wait for it.

**The ruling this settles.** S5 ruled that *a measurement another session can
change is not evidence about this crate*. S7-0 extends it one step, to the
session's own next edit: **a citation this plan's own corrections will
invalidate is not a citation.** `awk '/pub fn capacity/,/^  }$/'` survives every
edit above it and fails loudly rather than quietly when the thing it names is
gone. `grep -n` is untouched by the rule and stays: it finds its line by content
and only the printed number moves, so it goes stale without ever becoming wrong.
`sed -n 'N,Mp'` is the one form that keeps running, keeps exiting zero, and
starts printing something else.

**Order is not arbitrary and runs most-invalidating first.** S7a changes source,
which moves what every recipe in the family prints; S7d changes prose, which
moves almost nothing. Running S7d first would correct 87 statements against a
source S7a is about to change, and the § Regression Gate would catch it as 87
newly-stale documents. That is the exact miss-mode S7's own row names. So the churn is
paid once, at the front, and each later sub-stage is written against a tree the
earlier ones have finished moving.

Those are the machine's numbers, printed by
`corpus/scoreboard.py` and asserted independently by G14. The hand census this
plan was first drafted against read 418 / 792 / 778 and was wrong three ways at
once, which is the ordinary argument for § Machinery First stated as a fact
about this plan rather than as a principle.

## Rulings

### R1. `state_machine/` folds into `lifecycle/`, family-wide

S6 reserved this question and S3 had to answer it. `shape.py` fails an
undeclared directory, and all twelve crates carrying `state_machine/` sit in
S3, S4 and S5. No stage after S2 could print REACHED until it was settled, so
the ruling moved forward. It is one ruling applied uniformly, which is what S6
asked for. What S6 forbids is twelve renames wearing one name.

**The alternative was declaring it in, and it costs more than it saves.**
`shape.py` computes `missing = want_defs - present` unconditionally, so a
fourteenth declared definition fails all twenty crates that never had one. An
"optional definition" tier with exactly one member, invented at the moment it
would avoid work, also spends the property the standard exists to enforce: that
all 33 crates carry the same thirteen, which is what makes them comparable.

**What the fold costs, stated rather than smoothed over.** The two readmes drew
a real distinction. `lifecycle/` names the crate owning each phase,
`state_machine/` names which states are reachable, and ten crates stated it
symmetrically. Reachability is not a lifecycle question. `ring_types`' capacity
machine is *about* a state nothing ever enters, and a phase narrative has no
natural place for one. The fold relocates that distinction into each merged
`lifecycle/readme.md`'s Purpose rather than preserving it as a directory
boundary. Every state table, transition and unreachability claim moved verbatim:
19 instances, 3184 lines, line-for-line identical.

**The objection was answered later, by measurement rather than by argument.**
S6 opened with a census of where state-machine content actually lives, and it
does not live at directory grain in the first place:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rl '^### State Machine' --include=*.md ring_*/docs/ \
  | sed -E 's|^ring_[a-z_]+/docs/([a-z_]+)/.*|\1|' | sort | uniq -c | sort -rn
```

Live output:

```
     25 lifecycle
     12 type
     10 api
     10 algorithm
      7 pattern
      5 integration
      5 data_structure
      4 pitfall
      4 non_functional_requirement
      4 invariant
      1 decisions
```

Eighty-seven instances across twelve crates carry a `### State Machine` section,
spread over **eleven different definitions**, and only 25 of the 87, under a
third, sit in `lifecycle/`. A `state_machine/` directory could never have held
more than that third; the other 62 sections document the state machine of a
*type*, an *API*, an *algorithm* or an *invariant*, and belong with the
thing whose states they are. So the distinction the two readmes drew was real and was
never expressed as a directory boundary. It is expressed as a section heading,
at the grain where the content is. The fold did not relocate the
distinction; it removed a directory that was holding a minority of it.

**One hole the fold did leave, and S6 closes it.** `lifecycle/` exists in 32 of
33 crates. The exception is `ring_poll`, which is also the only crate short four
definitions rather than three. It is the one place where the merge had nothing
to merge into.

## Baseline Reading

Taken 2026-08-30, before any stage's first edit, over all 33 crates. **0 of 4
gates REACHED**, which is the verdict M0 required and the reason it required it.
A gate set that read green here would have graded nothing.

```
  crate                   defs      inst    findings   state
  ------------------ --------- --------- -----------   ----------------------------
  ring_debug            9/13      8/26       0/52      +4 def, +18 inst, +52 find
  ring_testkit          9/13     10/26       0/52      +4 def, +16 inst, +52 find
  ring_poll             9/13     11/26       0/52      +4 def, +15 inst, +52 find
  ring_shutdown        10/13     13/26       0/52      +3 def, +13 inst, +52 find
  ring_core            12/13     19/26       0/52      +1 def, +7 inst, +52 find
  ring_mpsc            12/13     19/26       0/52      +1 def, +7 inst, +52 find
  ring_spsc            12/13     19/26       0/52      +1 def, +7 inst, +52 find
  ring_tls             12/13     19/26       0/52      +1 def, +7 inst, +52 find
  ring_handle          12/13     21/26       0/52      +1 def, +5 inst, +52 find
  ring_bench           12/13     22/26       0/52      +1 def, +4 inst, +52 find
  ring_flush           12/13     22/26       0/52      +1 def, +4 inst, +52 find
  ring_factory         12/13     24/26       0/52      +1 def, +2 inst, +52 find
  ring_types           13/13     25/26       0/52      +1 inst, +52 find
  ring_align           13/13     26/26       0/52      +52 find
  ring_cursor          13/13     26/26       0/52      +52 find
  ring_gating          13/13     26/26       0/52      +52 find
  ring_seqno             13/13     26/26       0/52      +52 find
  ring_barrier         13/13     26/26      19/52      +33 find
  ring_wait            13/13     26/26      24/52      +28 find
  ring_publish         13/13     26/26      46/52      +6 find
  ring_atomic          13/13     26/26      52/52      at standard
  ring_config          13/13     26/26      52/52      at standard
  ring_event           13/13     26/26      52/52      at standard
  ring_overflow        13/13     26/26      52/52      at standard
  ring_registry        13/13     26/26      52/52      at standard
  ring_slot            13/13     26/26      52/52      at standard
  ring_stats           13/13     26/26      52/52      at standard
  ring_trace           13/13     26/26      52/52      at standard
  ring_batch           13/13     26/26      53/52      at standard
  ring_store          13/13     26/26      53/52      at standard
  ring_consume         13/13     26/26      54/52      at standard
  ring_claim           13/13     26/26      55/52      at standard
  ring_index           13/13     26/26      56/52      at standard
  ------------------ --------- --------- -----------   ----------------------------
  total               406/429   752/858    776/1716    13/33 at standard
```

**The `state` column above is the scoreboard's arithmetic, and it is short.** It
subtracts a crate's *total* instance count from 26 and never applies G14's
per-definition floor of two, so a crate holding four instances in one directory
and none in another reads as needing nothing. Taking S5's own baseline surfaced
it. The four crates read `+2 +5 +4 +4` = +15 by that column and owe **+22** by
the gate, because `workaround/` is empty in all four, `decisions/` in
`ring_flush`, and `data_structure/`, `pattern/` and `pitfall/` sit at one in
`ring_handle`. The S5 row's original "+15" was copied from this column, so the
Stage Table understated its own stage until the machinery contradicted it. The
family floor still clears, since 858 becomes 865 against the same 858. But the
column should be read as a lower bound, not a shortfall.

| Gate | Problems | What they are |
|---|---|---|
| G14 shape | 104 | 20 crates under the floors; `item/` absent from 12 crates; three crates also missing `data_structure/`, `lifecycle/`, `pattern/` or `non_functional_requirement/` |
| G15 recipes | 626 | 382 UNQUOTED, 212 REGEN, 16 STALE, 16 README, broken out below |
| G16 citations | 24 | 22 cited test names no test file defines; 2 links resolving to nothing |
| G17 vocabulary | 24 | Tier strings outside the declared set |

**Where those last two land is the reading, not the counts.** G16's 24 sit in
`ring_batch` 10, `ring_index` 5, `ring_stats` 4, `ring_overflow` 2, `ring_tls` 2,
`ring_event` 1. Five of the six are crates this family calls **at standard**, and
they cite twenty-two tests that no test file defines. G17's 24 sit in exactly
three crates: `ring_publish` 10, `ring_wait` 9, `ring_barrier` 5. Those are
precisely S2's three abandoned passes, and nothing else in the family. The
vocabulary was still being settled when those three passes ran, and they stopped
before it closed. This plan predicted neither correlation. Both came from
attributing the counts rather than reporting them.

**The 24 are three defect classes, and this plan anticipated one of them.**

| Class | Rows | Strings | What it is |
|---|---|---|---|
| Hybrid: record prefix on a reachable tier | 4 | `n/a — latent hazard` | PB22, PB35, PB36, PB42. Each says "act on this" and "archived" at once. The class G17 was written for |
| A third prefix nobody declared | 2 | `no — wrong doc`, `no — stale doc` | PB40, PB43. `wrong doc` *is* reachable, so `no — wrong doc` is the same hybrid in a second spelling the first pass would not have caught |
| Vocabulary the corpus uses and the declaration does not | 18 | `n/a — performance` 5, `n/a — naming` 4, `n/a — hygiene` 3, `n/a — blunted instrument` 3, `n/a — unmeasured` 3 | Five record tiers that exist in three crates and in no declaration |

Only the first was predicted. The second is the more useful find. A hybrid
survives a census that greps for the prefix it happens to use, so `n/a —` was
never the property worth checking and membership in a closed set always was.

The third is not a defect until somebody rules on it, and **S2 owns that
ruling**: either those five tiers join `corpus_standard.txt` because the family
does distinguish a naming gap from an observation, or the 18 rows are re-tiered
into the thirteen that already exist. Declaring them is not the safe default.
A vocabulary that grows to fit whatever was written stops constraining anything,
and constraining is the one thing a closed set is for.

**Two of those four numbers were wrong when this section was first written, and
the second was a defect in the machinery rather than in the reading.** G16's 24
were recorded as tier disagreements, which are G17's; and G15's total was
recorded as 749, which is what the gate itself printed. Its four kinds sum to
626, and the 123-line surplus was the quoted/actual diff G15 attaches beneath
each stale recipe. That diff is evidence for a problem, and it was counted as
problems. `report()` now
counts only entries that do not begin with whitespace, and every gate's total
equals its own kind sum, which is a self-check the first run could not have
passed. **A miscount in that direction is the harder one to catch.** The number
is alarming rather than reassuring, and every line printed under it is real.

**G15 is where the reading changes what this plan thought it knew.** Its four
kinds are not four severities of one problem:

- **382 UNQUOTED.** A recipe with no expected output beneath it. Not rot, but a
  document that never stated what it should print. Concentrated in the crates
  whose findings were written most recently: `ring_claim` 52, `ring_wait` 46,
  `ring_seqno` 45, `ring_publish` 45, `ring_cursor` 42, `ring_gating` 37,
  `ring_barrier` 30, `ring_consume` 29, `ring_poll` 16.
- **212 REGEN.** This one is two findings wearing one label, which the
  first draft of this section got wrong. **196** are a definition readme with no
  `### Regenerate` block *at all*; only **16** are a block that ran and exited
  non-zero. Describing the first as an execution failure is the kind of wrong
  prose no gate here catches, so it is recorded rather than corrected silently.

  The 196 is the reading worth having. Of the 392 definition readmes present
  under the thirteen declared definitions, **exactly 196 carry a `### Regenerate`
  block and exactly 196 do not**, and the ones that do are not scattered. Fifteen
  crates carry one in all thirteen of their definition readmes; `ring_barrier`
  carries one; the remaining seventeen crates carry none. Those fifteen are the
  thirteen at standard plus `ring_wait` and `ring_publish`. In other words,
  **the crates that got a findings pass got a regenerate pass in the same pass.**
  The convention was never optional and never partially adopted. It arrived with
  the findings, per crate, all thirteen at once.
- **16 STALE.** Genuine rot, and the only kind that was silently wrong.
  Line-number drift (`edition = "2024"` 192→212, `unsafe-code = "deny"` 227→254,
  three `mpsc_test.rs` citations off by 14), six documents quoting
  `- **Status:** planned` where the corpus now says `present`, and three where
  `opt-level = 2` replaced the expected line.

  It was 17 on the first run. The seventeenth was a recipe whose output carried
  `Blocking waiting for file lock on package cache` from a cargo run in another
  session, and it did not reproduce. That is worth recording rather than
  quietly dropping. This gate reads a tree other sessions are writing to, so a
  STALE hit is a claim about the corpus *and* about what else was running, and
  the two are only separable by re-running.
- **16 README.** A definition directory with no readme at all: `ring_testkit`
  8, `ring_debug` 8.

**`ring_claim` and `ring_consume` were called closed and are not.** Both clear
every count, with 55 and 54 findings against a floor of 52, and carry 52 and 29
recipes with nothing quoted beneath them. *Closed* has meant findings-complete
for the whole of this family's history; it has never once meant
recipe-complete, and no reading before this one could have shown the difference.

**M2's falsifiability test, run against this same tree.** Editing
`min_findings` from 52 to 53 in `declared/ring/corpus_standard.txt` moved the
scoreboard from `13/33 at standard` to `5/33`, and the denominator from 1716 to
1749; restoring it returned both. The eight crates that flipped are the eight
sitting at exactly 52. The gates read the declaration, not their own constants.

**M1's control arm, both directions.** Each of the four gates run against its own
fixture in `declared/ring/corpus_control/` reports NOT REACHED naming its own
seeded defect: `g14_missing_definition`, `g15_stale_quoted_output`,
`g16_tier_disagreement`, `g17_hybrid_tier`. A repaired copy of the G15
fixture returns `0 problem(s)`, exit 0. The gates can both fire and go quiet.

The first control run was wrong and is recorded rather than overwritten. It
pointed every gate at all four fixtures, and G16 fired on two of them. Deleting
a definition for G14 also breaks five links, and G17's hybrid tier is also a
disagreement between two tables. That proves *something* fires, never that each
detector fires, which is M1's own stated miss-mode reproduced by its first
implementation. `corpus_docs()` now narrows each gate to its own fixture under
`GATE_CONTROL`.

**S7's baseline, taken before a single correction.** `disposition.py` over the
33 crates reports **458 problem(s)**, every one of them `DISP`. Each is a reachable
finding whose section carries no disposition line at all. Not one `FORM`, `EVID`
or `WHY`, which is the right shape for a corpus that has never been asked this
question. The failure is uniform and total, not scattered.

Two things make it a baseline rather than a number. The 458 was measured twice
by unrelated means: once by hand, greping findings-row shapes for a bolded
tier, and once by this gate, walking each Module Index fence-aware. The two
agree exactly. And the four `--tier` scopes partition it without remainder:
180 + 62 + 129 + 87 = 458.

**G20's control arm needs five fixtures where the others need one.** G14–G17 each
have a single detector and a single seeded defect. G20 has four labels, so it
carries four: `g20_missing_disposition`, `g20_unevidenced_application`,
`g20_shrugged_decline`, `g20_double_disposition`. Each reports its own label
and no other, and `g20_clean` reports `0 problem(s)` at exit 0. The fifth is
not symmetry for its own sake. At the moment the gate was written every finding
in the family lacked a disposition, so the gate had never once been observed to
pass; without a clean fixture there was no evidence it *could*, and a stage whose
verdict is unreachable by construction would have looked exactly the same from
the outside as one that was merely not yet done.

All five ignore the record tiers, and that is checked rather than assumed. Each
fixture carries an `n/a — observation` finding alongside its seeded one, and no
fixture ever reports it.

## Reached Verdicts

One row per stage, each produced by the four gates rather than by arrival at the
next stage. **Regression** is the Regression Gate reading for that round. It
covers every crate reached so far, re-run in the same run as the stage being
closed, not harvested from earlier rounds.

| Stage | Crates | Verdict | Gate reading | Regression |
|-------|--------|---------|--------------|------------|
| M0 | n/a | **REACHED** | 20 crates NOT REACHED, 13 REACHED; each gate names crate and count | n/a |
| M1 | n/a | **REACHED** | `--control` NOT REACHED four times, each naming its own seeded defect after `corpus_docs()` narrowed each gate to its own fixture | n/a |
| M2 | n/a | **REACHED** | `corpus_standard.txt` supplies 13 names, the closed tier set, three thresholds | n/a |
| S1 | `ring_align` `ring_cursor` `ring_gating` `ring_seqno` | **REACHED** | 4 crates at standard; G17 clean | 4/4 |
| S2 | `ring_publish` `ring_wait` `ring_barrier` | **REACHED** | 3 crates at standard; G17's 24 hits cleared, five undeclared record tiers ruled on | 7/7 |
| S3 | `ring_types` | **REACHED** | `13/13 defs, 30/26 inst, 52/52 findings — at standard`; all four gates 0 problems | 8/8 |
| S4 | `ring_core` `ring_mpsc` `ring_spsc` `ring_tls` | **REACHED** | 13/13 defs and 52/52 findings each; 27/28/28/28 instances against a floor of 26; all four gates 0 problems | 25/25 |
| S5 | `ring_factory` `ring_handle` `ring_flush` `ring_bench` | **REACHED** | 13/13 defs and 52/52 findings each; 28/28/28/27 instances against a floor of 26; all four gates 0 problems; per-definition floor of ≥2 checked separately and clean across all 29 | 29/29 |
| S6 | `ring_debug` `ring_poll` `ring_testkit` `ring_shutdown`, and R1 across 12 | **REACHED** | 13/13 defs each; 52/52 findings each except `ring_testkit` at 54; 26/26/27/26 instances against a floor of 26; all four gates 0 problems; `state_machine/` now 0 of 33 and `lifecycle/` 33 of 33, closing R1's one remaining hole | 33/33 |
| S7-0 | all 33 | **REACHED** | `G21 addressing: 0 problem(s)` over 33 crates, down from 757 line-addressed extractions across 26 of them; both control fixtures fire, one per spelling of the address | 33/33, with G14, G15, G16, G17 and G21 all REACHED in one run |
| S7-0b | all 33 | **REACHED** | `G21 addressing: 0 problem(s)` over 33 crates, down from 154 findings across 20 of them. That followed retracting the `grep -n` exemption on measuring 192 stale recipes family-wide, 122 of them pure line-number drift. The exemption had been narrowed to fire only when a requested line number reaches a quoted `Live output` block | 33/33, with G20 and G21 both REACHED in the same run; G15/G16/G17 open for reasons unrelated to this stage (see below) |

**S7-0's gate was wrong twice before it was right, and both times it reported
clean.** The first version knew only `sed -n 'N,Mp'`, swept the family, found
zero, and was believed. The 28 `awk 'NR >= a && NR <= b'` addresses it could not
see surfaced out of G15 a round later. Four of them were already printing
different code than the prose beside them claimed. The second version was then
blinded by its own repair. The converter emits `awk -v n1="$( … )" 'NR … 930' T`, and a
pattern anchored on the quote immediately after `awk` skips the whole call, so a
literal still sitting in it went invisible at the moment it was converted. The
fix was to stop matching and start tokenising with `shlex`, which also widened
coverage to targets like `"$G"` the regex had been dropping. **A gate that names
its rule in prose and implements one syntax for it enforces the syntax, and
nothing in a clean report distinguishes the two.**

**The stage's declared miss-mode fired, exactly as written.** The row predicted
anchors "chosen so loosely that they match in several places." The converter's
first pass did precisely that. Walking upward for the nearest non-blank line, it
anchored one `ring_bench` extraction on `}`, which appears 146 times in the file,
and a `ring_stats` one on a bare four-space `  {`. Both reproduced byte-identically
*that day*, which is what makes the failure mode worth naming. Acceptance by
output comparison cannot distinguish an anchor that is right from one that is
merely first. A uniqueness predicate on the candidate closed it, and a separate
sweep found **37 pre-existing `-m1` anchors on non-unique lines across 12
crates**. These were position addresses wearing content's syntax. Thirty-five were
re-anchored mechanically, two by hand, and one benign self-reference remains
(a document grepping its own H1, where the duplicate is the `-F` literal in the
recipe itself).

**G21 does not catch every line address, and the gap is a shape it has no
detector for.** Four `ring_core` recipes anchored *above* the block they wanted
and reached forward with a fixed `-A6`/`-A8`/`-A22`. A content anchor plus a
fixed offset spanning intervening code is still a line address, only a relative
one. The five-line source correction landed between anchor and block in all
four, sliding every window by exactly five while the recipes kept exiting zero.
They were found by diffing each recipe's output against its own quoted block and
reading the prose, not by any gate. **The rule G21 enforces is "no absolute line
address"; the rule it means is "the citation moves with the thing it names," and
the second is strictly wider than the first.**

**The two gates this stage added had never run.** `declared/ring/gates.txt`
listed `g18` and `g19`, and `run_all.sh` resolves a gate id by globbing
`${g}_*.sh`, which matched `g18_family_coverage.sh` and
`g19_measured_columns.sh`. Those are two repository-wide gates that the gate readme
explicitly states belong to no family and appear in no `gates.txt`. The
disposition and addressing checkers had no wrapper at all, so for two rounds
they were declared, invoked by hand, and absent from every family run. They are
now **G20** and **G21** with their own wrappers, their control fixtures renamed
to match, and both verified through `run_all.sh --family ring --control`: five
G20 fixtures each reporting only their own label, and two G21 fixtures, one per
address spelling.

**Adding those two wrappers broke two documents, which is the stage's own thesis
arriving on schedule.** `ring_flush`'s FL23 counts the gate directory and
narrates the number in prose; `ring_poll`'s regenerate block prints it as
context. Both went stale the moment the count moved 19 → 21. Two further blocks
broke for reasons worth separating from staleness. `ring_gating` controlled its
"no `#[ inline ]` in the family" claim against a raw count over
the whole repository. That is a workspace-wide number any concurrent session
moves, and it already disagreed with its own prose by two before this round
touched it. It is now stated as a relation, per the S5 rule the same recipe cites
two lines below for its profile check. `ring_consume` ran `cargo tree` without redirecting stderr,
so under lock contention its quoted block acquired `Blocking waiting for file
lock on package cache` lines. **A recipe whose output depends on what else is
running is unquotable, not stale.** The guard is now on both of its cargo calls.

**S3's instance count came in at 30 against a floor of 26.** The stage row asked
for +1 (25→26); five were added, because the four the row did not ask for are
what let the 52 findings be about the thirty-one dependents rather than about
this crate's derive lists, which is the miss-mode the row names. Two are ADRs promoting
questions the crate had filed as prose (`decisions/001`, `002`), two read the
40-file `item/` catalogue as a set (`item/001`, `002`), and one records a
language constraint the whole family inherits (`workaround/002`).

**The stage's own miss-mode was live and caught by machinery, not by review.**
Four claims drafted from unfiltered greps were false: `Seq( .. )` direct
construction read as 17 crates and is 7 sites in 5; `Seq( 0 )` read as 21 uses
and is 0 in production; a `#[ non_exhaustive ]` count of 2 included a
`.finish_non_exhaustive()` call; and a reach loop matching bare `new`/`get`/`next`
counted `Vec::new` and `Iterator::next` family-wide. Each was caught by running
the recipe and diffing its output against the prose, which is the discipline G15
enforces. Otherwise each would have read as a confident finding.

**S4's declared miss-mode did not occur, and the stage's real finding came from
the Regression Gate instead.** The row's fear was that the seven new instances
would be the existing nineteen redistributed, with the count satisfied and
nothing new documented. The four crates came in at 27, 28, 28 and 28 instances carrying 52
distinct findings each, against 29k–48k words of prose; splitting nineteen files
into twenty-six does not produce that. What the round did catch was in a crate
this plan had already called closed. `ring_claim`, one of the thirteen graded
by hand before the four gates existed, had 55 findings with headings and Module
Index rows and **no per-definition rows at all**, because its thirteen
definition readmes still carried the pre-standard two-column `| Finding | Where |`
schema. The three-way contract had been half-unmet since before the standard was
written, and no reading short of running the gates over it would have said so.
That is the § Ungated by decision row about the thirteen, arriving as an
observation rather than an admission.

**The tenth silent-filter trap was found the same way, in the regression round
rather than in review.** `ring_batch/docs/api/readme.md` ran
`grep -rn 'use ring_batch|claim_gated|drain_order'` with no `-E`, so the
alternation was a literal string, the scan matched nothing, and the block
reported an empty external reach for a crate `ring_tls` and `ring_cursor` both
name. It exited 1 and the gate caught it on exit status. The wrong answer was
never the thing that failed. Repairing it moved BA7's reach from zero dependents
to one, which is the finding the instance had always claimed and had, until this
round, been proving with a scan that could not have found anything.

**S5's declared miss-mode did not occur, and inverted.** The row's fear was that
the four crates would be documented from the inside, describing how each works
and never stating the export contract. What happened instead is that the
contract became the subject and did not survive contact with it. `ring_factory`
returns a `ring_handle::Split` whose `Ends::split` yields one producer with no
`try_clone`, so the two genuinely multi-producer candidates are single-producer
when reached the documented way; and `ring_flush::Flusher::new` takes a
`ring_core::Producer` that nothing on the five-crate Contract can produce, so
`ring_bench` builds its staged candidate one level *below* the door the Contract
names. Both are findings that only exist because a crate tried to consume the
contract end to end rather than describe it.

**The Regression Gate found more than the stage did, for the second round
running, and this time the cause was outside this plan entirely.** Fourteen
recipes across eleven already-closed crates went stale, none of them from an
edit to those crates. `any crate root` grew from 170 to 190 crates while S5 was being
written, by sessions with no connection to the ring family, and three distinct
mechanisms turned that growth into false documentation:

| Mechanism | Example | Fix |
|---|---|---|
| A line-number anchor into a shared, growing file | `Cargo.toml:279` became `:330`, four times over | Drop `grep -n`; quote the line's content, cite by row |
| A family-wide census catching an identically-named item in an unrelated family | `spatial_mask::LayerMask::mask()` entering `ring_index`'s `.mask()` census; `spatial_pool`'s `.fill(` entering `ring_event`'s | Scope to `ring_*/` where the claim is about the family; keep the width and state the growth where the claim is about the workspace |
| A claim of the form *nothing under X names Y*, falsified by a **document** rather than by code | `ring_bench`'s own instances citing `ring_trace` and `ring_claim` | Split the census into code-and-manifest versus documents. A citation in a corpus is not an edge in a build |

The third fix strengthens the findings it touches rather than weakening them,
because the distinction it forces into the recipe is the one the finding was
always making.

**One document could not be stabilised and had to be redesigned instead.**
`ring_config/docs/workaround/002` printed how many crates exist under `any crate root`,
so every crate any session added anywhere in the workspace made it stale. It
drifted twice inside a single gate round, 189 during the run and 190 minutes
later. Patching the number was a losing race. The recipe now prints no moving
cardinal at all. It prints the escalating count (55), its ring/non-ring split
(33/22), and whether every crate inherits the lints table (*all of them*). Every
one of those has held across three censuses of a workspace that grew by more
than half. **A
measurement another session can change is not evidence about this crate**, and a
document whose subject is a workspace-wide policy has to be written against the
invariant rather than the population.

**The machinery's own blind spot was measured, not assumed.** `recipes.py`
reports `EXIT` for an instance recipe only when the exit code is non-zero **and**
the output is empty (`recipes.py:90`); for a definition readme it checks the exit
code and never compares the output at all. The two halves are exactly
complementary. Instances are checked on what they print and not on whether they
succeeded, readmes on whether they succeeded and not on what they printed. This
is not hypothetical. Across the family, 57 recipes end a loop body with
`[ cond ] && printf …`, whose exit status is that of the last iteration's test,
so a non-zero exit is the normal case and the gate has never once reported it.
Recorded rather than repaired: rewriting 57 stable recipes would risk fresh drift
in documents that are currently correct, for no change in what the gate reports.

**S6's declared miss-mode was foreclosed before the stage began rather than
avoided during it.** The row's fear was a per-crate `state_machine/`→`lifecycle/`
rename, which would be twelve unreviewed decisions wearing one name. That could not happen,
because R1 was written as one family-wide ruling with its own census before any
crate was touched, which is what the row asked for. The stage's own contribution
was closing the hole R1 left. `lifecycle/` existed in 32 of 33 crates, the
exception being `ring_poll`, and `ring_poll` was one of S6's four. The family now
reads 0 crates carrying `state_machine/` and 33 carrying `lifecycle/`. That is the
first family-wide structural fact in this plan that is true by construction
rather than by sweep.

**The Regression Gate found more than the stage did for the third round running,
and this time it caught the stage's own blast radius.** Three crates already
closed went stale, and the first of them was stale *because of* S6. `ring_testkit`
had filed TK44, the claim that a question parked in a consumer's decision log
never reaches its owner. It proved the claim by measuring `ring_shutdown`: eleven
definitions, zero decisions naming `Stopped::reopen`, and exactly one file naming
`ring_testkit`, a test comment. Bringing `ring_shutdown` to standard moved all
three: thirteen definitions, two filed decisions, and ten files naming
`ring_testkit`, nine of them documents. The finding got *stronger*, not weaker.
`ring_shutdown` had since read `ring_testkit` closely enough to cite its
`Stopped` binding and its guard as the only ones outside itself, and still came
back with none of the question. What had been an easy story about two crates that
do not talk became a measured claim that the channel exists, carries detail in
both directions, and moves facts about code rather than open questions about
design. No reading of either crate would have surfaced that; only running the
gate over the pair after changing one of them did.

The other two were the workspace moving underneath, the same mechanism S5
recorded and the same fix:

| Crate | What moved | Ruling |
|---|---|---|
| `ring_overflow` | Crates under `any crate root` declaring a `pub const fn`, 59 → 60 | Prose stopped restating the cardinal; the census keeps it |
| `ring_stats`, `ring_batch` | `docs/feature/*.md` `Status:` tally, 367/58 → 366/59, twice, in different crates | Both recipes now print the invariant, never the population: which value dominates, how many contiguous runs, the widest |

`ring_stats` is the sharper of the two, because the volatile measurement *is* the
subject. ST31's claim is that `Status` is a marker updated in bulk across many
entries at once rather than per feature, rendered in contiguous blocks, so a
per-feature reading of it is meaningless. The recipe proving that claim went
stale twice for exactly the reason the claim gives. An unrelated bulk update
touched other entries, and nothing about `ring_stats` changed. The repair was to print run count and widest run instead of a tally.
After that the staleness stops and the claim reads the same. **A document whose
subject is a moving population has to measure the population's shape, not its
size**, which is the S5 ruling arriving a second time in a case where following
it also sharpened the finding.

**Two defects the gates cannot see were found by hand while repairing ones they
can.** `ring_overflow`'s OV41 asserted that a census "finds none of" `peek_`,
`try_`, `dry_` or `preview_`, while greping for none of them. Three are indeed
absent; `try_` occurs 72 times under `any crate root` meaning *fallible attempt*, so the
one prefix a maintainer is likeliest to reach for is the one that would say the
wrong thing, and the finding is better for saying so. Separately,
`ring_stats`'s Severity census listed ST31 under `n/a — drift` while both of its
findings rows tier it `**misleading doc**`, and closed with "21 of 52, the
highest share of any crate in this family". The count was 22, and S5 and S6
overtook the superlative by putting `ring_shutdown` and `ring_bench` at 29 of 52.
Neither is reachable by any current checker. `recipes.py` compares a
recipe to its own quoted output and never to the prose beside it, and nothing
compares two tables inside one document. Both are recorded in § Ungated by
decision as the class rather than the instances.

**S7-0's own exemption came back as S7-0b.** `grep -n` was carved out of the
addressing rule on the ground that, unlike `sed -n`/`awk`, its printed number
tracks content rather than replacing it. That reasoning is in § S7 recurses
into four, by tier, and is restated in the checker's own docstring. Remeasured on
2026-09-04 after edits to `ring_align`, `ring_config` and `ring_cursor`, the
carve-out was hiding the identical failure it was written to avoid: 192
recipes stale, 122 of them differing from their quoted output in nothing but
the number `grep -n` had printed. The exemption was retracted to fire only
when a requested line number reaches the page, meaning the recipe asks for
`-n` and the quoted `Live output` block carries a result. It leaves alone the
`grep -n` calls whose number is consumed mid-pipeline and never published.

**The reopened gate found 154 stale recipes across 20 crates, closed in ten
crate-grouped batches.** A family-wide sweep after the tenth batch confirmed
`G21 addressing: 0 problem(s)` across all 33 crates with `G20 disposition`
still at 0, but also surfaced two regressions the batches themselves had
caused. `ring_align` keeps a second, independent copy of each finding's
Subject and Tier in its `definition/readme.md` index, checked against the
owning definition by G16, and AL4's and AL34's Subject text had been reworded
to drop a line-number citation without the index being reworded to match.
Both were re-synced against the current text and reconfirmed clean by a
standalone re-run. `G21` and `G20` hold REACHED for this stage; `G15`, `G16`
and `G17` stay open in the same family sweep for reasons this stage did not
cause and does not fix: `ring_align`'s AL1 carries a tier (`**resolved**`)
outside the declared set, and `ring_bench`, `ring_trace` and `ring_types`
carry 47 STALE findings from source edits made elsewhere, after this stage's
own recipes had already been verified against the text they cite.

## Ungated by decision

| What | Why ungated | What would gate it |
|---|---|---|
| Whether a finding is true | See § What the gate cannot decide. The regenerate block proves its quoted output current, never that the prose reads it correctly | Nothing mechanical. The miss-mode column is the substitute and is deliberately not a gate |
| Whether an instance duplicates its sibling | Two files can share a subject and differ entirely in what they establish; a similarity detector at that precision would be ignored within a week | A per-definition statement of what each instance establishes that its sibling does not. Worth writing, not worth gating |
| The 13 already-closed crates' finding *quality* | They were graded by the same four checkers, by hand, one crate at a time. Re-grading them mechanically confirms structure, which is all any run confirms | Nothing. This is the honest limit of what closed means here |
| Whether a recipe's output supports the prose beside it | `recipes.py` compares a recipe to its own quoted block and never to the sentence citing it. `ring_overflow`'s OV41 claimed a census "finds none of" four prefixes while greping for none of them. Three are absent and the fourth occurs 72 times, yet it passed every gate for as long as it existed | A checker extracting numerals and quoted phrases from prose and requiring each to appear in the block above it. Buildable, and it would fire on every rhetorical numeral too, which is most of them. S11 narrows this to the one case worth gating |
| Whether two tables inside one document agree | Nothing reads a definition readme's Severity census against its own findings rows. `ring_stats` filed ST31 as `n/a — drift` in one table and `**misleading doc**` in two others, in the same file, and closed with a count one short and a superlative S5 had already overtaken | A per-file cross-table check. This is the cheapest of the three and genuinely mechanical; it is ungated because no stage has claimed it, not because it is hard |

## Final Goal

One command, one run, three assertions no earlier stage makes together:

```bash
cd "$(git rev-parse --show-toplevel)"
bash bench_harness/gate/run_all.sh --family ring g14 g15 g16 g17
bash bench_harness/gate/run_all.sh --family ring --control g14 g15 g16 g17
```

- all four gates REACHED over 33/33 crates **in that run**, not harvested across rounds
- all four gates NOT REACHED against the control corpus, each naming its own defect
- 429 definitions, 858 instances, 1727 findings, with the three counts agreeing
  between the tree, each definition readme, and each Module Index

## Cross-references

| File | Relationship |
|------|--------------|
| [`rulebook.md`](../../../../rulebook.md) | *Documentation : Executable Recipes*: the control-arm requirement and the "Not gated, deliberately" finding that bounds § What the gate cannot decide |
| [`gate/readme.md`](../../gate/readme.md) | G1–G13, the gate set this plan's G14–G17 join |
| [`acceptance/001`](../acceptance/001_feature_reached_tests.md) | The family's implementation plan's M1 deliverable, hosted here for the same reason this plan is |
