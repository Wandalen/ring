# Lifecycle: A Finished Crate in the Unverified Stage

### Scope

**Purpose:** Record that this crate's task file places the work at the earliest
lifecycle stage with an unwritten scope, against a crate readme and a family
index that both call it finished, and establish that the same contradiction holds
for all 33 crates rather than this one.

**Responsibility:** Task 125's state marker and its two placeholder sections, the
crate readme's completion claim, the family index row, and the stage census
across every `ring_*` task tree.

**In Scope:** `ring_trace/task/unverified/125_implement_ring_trace.md:3`,
`:23`, `:29`; `ring_trace/readme.md:16-18`; the `ring_trace` row of
`docs/crate/readme.md` — cited by row rather than by line, because that file is
shared and its line numbers drift under unrelated edits; every
`ring_*/task/` stage directory.

**Out of Scope:** The runtime lifecycle of the type is
[`lifecycle/001`](001_three_states_and_the_two_ways_out_of_one.md). The
dependency the same two files get wrong is
[`workaround/002`](../workaround/002_a_third_dependency_two_documents_still_name.md).

---

## Three Documents, Two Answers

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the task file says the state of the work is --'
command grep 'State:\|Not yet worked out\|Not yet defined' ring_trace/task/unverified/125_implement_ring_trace.md | sed 's/^/    /'
echo '  -- what the crate readme says --'
command grep 'Implemented\|every line is covered' ring_trace/readme.md | sed 's/^/    /'
echo '  -- what the family index says --'
# no -n here: the index is a shared file and its line numbers drift under
# unrelated edits, while the row itself is what the finding rests on
command grep '| `ring_trace` |' docs/crate/readme.md | sed 's/^/    /'
echo '  -- and how many of the 33 are in the same position --'
u=0
for c in ring_*/task/; do u=$(( u + $( ls "$c"unverified 2>/dev/null | wc -l ) )); done
printf '    task files in unverified/: %s\n' "$u"
for st in verified executed accepting; do
  n=0
  for c in ring_*/task/; do n=$(( n + $( ls "$c$st" 2>/dev/null | wc -l ) )); done
  printf '    task files in %s/: %s\n' "$st" "$n"
done
# the index encodes Build State as a glyph, not the word; the legend is echoed
# so a further schema migration shows up here as a changed line rather than a 0
command grep -m1 'Build State (glyph 1)' docs/crate/readme.md | sed 's/^/    /'
printf '    ring crate rows in the family index:  %s\n' \
  "$( command grep -c '| `ring_[a-z_]*` |' docs/crate/readme.md || true )"
printf '    of those, Build State ● implemented: %s\n' \
  "$( command grep -c '| `ring_[a-z_]*` |.*| ●' docs/crate/readme.md || true )"
```

Live output:

```
  -- what the task file says the state of the work is --
    - **State:** ❓ Unverified
    Not yet worked out. This task is `❓ Unverified` — it must pass the readiness
    Not yet defined — see Scope.
  -- what the crate readme says --
    Implemented. Behaviour is asserted by
    [`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.
  -- what the family index says --
    | — | `ring_trace` | Optional sequence-operation trace log | 1 | 328 | 3/3 | 19 | trace_test.rs | ●∅🟩 —🟢 |
  -- and how many of the 33 are in the same position --
    task files in unverified/: 33
    task files in verified/: 0
    task files in executed/: 0
    task files in accepting/: 0
    | Flags — Build State (glyph 1) | `●` `◐` `○` | implemented · skeleton · unfiled |
    ring crate rows in the family index:  33
    of those, Build State ● implemented: 33
```

---

### TR31 — The Task File Describes Work That Has Not Been Scoped, for a Crate That Is Done

Task 125 carries `**State:** ❓ Unverified`, the first stage in the lifecycle —
the one that means the work has not yet passed the readiness gate that would let
anyone claim it. Its Scope section reads "Not yet worked out. This task is
`❓ Unverified` — it must pass the readiness verification gate before it can be
claimed, and that gate requires this Scope section to name concrete deliverables
rather than restate the goal." Its Verification section reads "Not yet defined —
see Scope."

The crate is 328 lines with nineteen tests, a 115-line manual test plan, and a
readme that says "Implemented. Behaviour is asserted by `tests/trace_test.rs`
and read by hand against `tests/manual/readme.md`; every line is covered." The
family index agrees, with the numbers: Build State implemented, 328 lines, 3 of
3 items documented, 19 tests, Verdict green, proven by `trace_test.rs`.

**Correction (2026-09-28):** the readme quote above used to read "Implemented,
delivering this crate's trace half. Behaviour is asserted by..."; the crate
readme has since dropped that extra clause, and
now opens "Implemented. Behaviour is asserted by..." with no other change. The
finding is unaffected — the readme still claims Implemented and fully tested
while task 125 still opens `❓ Unverified`.

That row has since been migrated to a packed-glyph schema — `●∅🟩 —🟢` in place
of the words — which is why the recipe now reads the glyph and echoes the legend
that decodes it. The migration changed how the index says "implemented" and not
whether it says it: all 33 ring rows still carry `●`.

So the document whose job is to say what should be built says nothing has been
decided, and the two documents whose job is to describe what exists both say it
is finished and tested. The task file is not merely stale in its state marker —
its two substantive sections were never filled in, so it never described this
crate at any point, including before the work happened.

**Finding.** Recorded as the crate's own documentary lifecycle running backwards
from its code's. Two repairs, and the order matters: advance the state marker and
move the file out of `unverified/`, and fill Scope and Verification with what the
crate actually delivers — five operation kinds, a gated log, the poisoning
consolidation the plan records — so the file describes the work as done rather
than as unplanned. Advancing the marker without filling the sections would move a
placeholder into a stage that means "ready to claim", which is worse than leaving
it where it is.

**Disposition:** declined — the finding's own required order (fill Scope and
Verification, then advance State out of `unverified/` and through the
Readiness Verification Gate) is a real task-lifecycle closure, the same
governed action this repository tracks as its own dedicated "Close task NNN"
work item elsewhere, not a docs-corpus text edit. TR32 shows the same gap
holds for all 33 `ring_*` crates from one never-advanced task tree; closing
`ring_trace`'s task 125 alone here would fix one row of a 33-row systemic
pattern while prejudging how the other 32 get handled, and the finding itself
warns that a partial fix (marker moved, sections still empty) is worse than
the status quo. Left to a dedicated task-closure action, scoped like the
`ring_trace` (125) entry the family's task-closure sweep already tracks.

---

### TR32 — It Is Not This Crate; It Is All Thirty-Three

The stage census is unanimous in both directions. All 33 `ring_*` task files sit
in `unverified/`, with `verified/`, `executed/` and `accepting/` empty across the
entire family. And all 33 crate rows in `docs/crate/readme.md` are marked
implemented.

That symmetry rules out the explanation a single stale file would invite. This is
not a task somebody forgot to advance; it is a task tree that was generated once,
before the work, and never touched again while 33 crates were written, tested and
indexed. The lifecycle machinery exists — four stage directories per crate, a
readiness gate defined in the governing rulebook, a state marker in every file —
and nothing has ever moved through it.

**Finding.** The finding is about the instrument rather than the crate. A
lifecycle with four stages and 100% of its population in the first one is not
tracking anything, and every consumer that reads it — a person picking up work, a
tool counting what is left — gets the answer "none of the 33 crates has started"
from a workspace where all 33 are green. Fixing 33 files is a batch job; the more
useful outcome is deciding whether the task tree is meant to be maintained at all,
because a stage system nobody advances is more misleading than no stage system,
and the family index is already carrying the state honestly.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/001`](001_three_states_and_the_two_ways_out_of_one.md) | The type's own states |
| [`workaround/002`](../workaround/002_a_third_dependency_two_documents_still_name.md) | The other claim these two files share and get wrong |
| [`integration/002`](../integration/002_the_criterion_lives_one_document_away.md) | The third documentary mis-statement about this crate |
| [`integration/001`](../integration/001_a_vocabulary_for_crates_that_never_call_it.md) | Why nothing downstream would notice either way |

### Sources

| Fact | Where |
|------|-------|
| The state marker | `ring_trace/task/unverified/125_implement_ring_trace.md:3` |
| The unwritten Scope and Verification | Same file, `:23`, `:29` |
| The readme's completion claim | `ring_trace/readme.md:16-18` |
| The family index row | `docs/crate/readme.md:676` |
| 33 in `unverified/`, none anywhere else | Census above |
| 33 rows marked implemented | Census above |

### Tests

| Test | Covers |
|------|--------|
| `an_enabled_trace_records_exactly_one_entry_per_operation` | Behaviour the task file never scoped |
| `concurrent_recorders_lose_no_entry` | The concurrency work the plan records and the task does not |
| `clearing_empties_the_log_without_switching_it_off` | The deliverable no Verification section names |
