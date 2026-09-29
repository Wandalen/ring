# Lifecycle: Implemented, Tested, and Still Planned

### Scope

**Purpose:** Record the crate's own lifecycle as a work item — the state its
implementation task declares against the state of the code that task describes.

**Responsibility:** The task file's declared state, its Scope section, the
dependency list it carries, and how the same reading comes out across the other 32
crates in the family.

**In Scope:** `ring_overflow/task/unverified/113_implement_ring_overflow.md`;
`ring_overflow/src/lib.rs:5`; `ring_overflow/readme.md:5`;
`ring_overflow/Cargo.toml:9-10`.

**Out of Scope:** The lifetime of the values the crate produces is
[`lifecycle/001`](001_a_resolution_computed_matched_and_discarded.md). What
`ring_stats` is depended on *for* is
[`integration/002`](../integration/002_the_stats_edge_and_the_function_nobody_imports.md).

---

## What the Task Says, and What the Crate Is

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the state of this crate implement-task, and the tests already written --'
command grep -m1 -A2 -F '# Task 113: Implement `ring_overflow`' ring_overflow/task/unverified/113_implement_ring_overflow.md | tail -n 1
command grep -c '^#\[ test \]' ring_overflow/tests/overflow_test.rs || true
echo '  -- the three prose places this crate names its dependencies --'
command grep -m1 -F '//! Depends on `ring_types` and `ring_stats`.' ring_overflow/src/lib.rs
command grep -m1 -F 'Depends on [`ring_types`](../ring_types/readme.md).' ring_overflow/readme.md
command grep -m1 -F '**Depends on:** `ring_types`' ring_overflow/task/unverified/113_implement_ring_overflow.md
echo '  -- and what the manifest declares --'
command grep -m1 -A1 -F 'ring_types = { path = "../ring_types" }' ring_overflow/Cargo.toml
echo '  -- implement-tasks still in the first Open state, of 33 ring crates --'
ls ring_*/task/unverified/*implement* 2>/dev/null | wc -l
```

Live output:

```
  -- the state of this crate implement-task, and the tests already written --
- **State:** ❓ Unverified
15
  -- the three prose places this crate names its dependencies --
//! Depends on `ring_types` and `ring_stats`.
**Depends on:** `ring_types`, `ring_stats`
  -- and what the manifest declares --
ring_types = { path = "../ring_types" }
ring_stats = { path = "../ring_stats" }
  -- implement-tasks still in the first Open state, of 33 ring crates --
33
```

---

### OV47 — The Task Sits in the State Before "Claimable" While the Work It Describes Is Finished

Task 113 declares `❓ Unverified` — the first Open state, the one a task occupies
*before* it has passed the readiness gate that would let anyone claim it. Its Scope
section reads "Not yet worked out", and explains that the gate "requires this Scope
section to name concrete deliverables rather than restate the goal". The file
carries no `## Verification Record` section.

The crate it describes is 237 lines of implemented code with 15 passing tests and,
as of this instance, a thirteen-definition documentation corpus.

**Finding.** So the work item's lifecycle and the artifact's lifecycle have come
apart completely — not by one stage, but from end to end. The task says the work
is not yet ready to be started; the work is done.

The census says this is not a local oversight: all 33 `ring_*` implement-tasks are
in `unverified/`, and all 33 crates ship a `src/lib.rs`. Zero have moved to any
later stage. A single sweep produced the tasks, the crates were then written, and
nothing walked the tasks forward.

Recording the systemic version here would be the wrong place for it — it belongs to
whatever governs the family's task set, not to this leaf. What belongs here is the
local consequence: task 113 is not a usable record of this crate's state, and the
`docs/` tree beside it is. Anyone reaching for the task to learn what
`ring_overflow` needs will be told it needs implementing.

---

### OV48 — Two of the Three Prose Dependency Lists Omit `ring_stats`, and the Correct One Is the Source File

The manifest declares two path dependencies, `ring_types` and `ring_stats`. Three
places state that dependency in prose, and they do not agree:

- `src/lib.rs:5` — "Depends on `ring_types` and `ring_stats`." Correct.
- `readme.md:5` — "Depends on `ring_types`", linked to its readme. Omits `ring_stats`.
- `task/…/113:19` — "**Depends on:** `ring_types`". Omits `ring_stats`.

**Finding.** The two that are wrong are wrong identically, and the one that is right
is the one physically closest to the code. That pattern dates the divergence rather
than just recording it: the readme and the task were written from the same
pre-`ring_stats` design, the dependency was added later, and only the module doc —
the file a developer edits while editing the code — was brought along.

The cost lands on exactly the reader the other two are for. `ring_stats` is not an
incidental edge; it is the reason the crate has two functions instead of one, since
its `fetch_add` is what forecloses `const` on `resolve`
([`workaround/001`](../workaround/001_the_recorder_forecloses_const.md) § OV37).
A reader who takes the readme's word for it sees a crate depending on one type
library and cannot account for why the mapping is written twice.

Both are one-line corrections, and the task file's is the one that also needs its
state moved, so they are naturally the same edit.

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order. The claim is about
# two dependency sentences, so the recipe reads those two sentences rather
# than counting the files containing them — see the paragraph below.
echo '  -- the readme dependency sentence --'
command grep -m1 -A1 '^Depends on' ring_overflow/readme.md
echo '  -- task 113 Depends-on line --'
command grep -m1 '^\*\*Depends on:\*\*' ring_overflow/task/unverified/113_implement_ring_overflow.md
echo '  -- ring_stats named in each of those two, and in the manifest a build enforces --'
printf '    readme dependency sentence : %s\n' "$( command grep -m1 -A1 '^Depends on' ring_overflow/readme.md | command grep -c 'ring_stats' )"
printf '    task 113 Depends-on line   : %s\n' "$( command grep -m1 '^\*\*Depends on:\*\*' ring_overflow/task/unverified/113_implement_ring_overflow.md | command grep -c 'ring_stats' )"
printf '    Cargo.toml                 : %s\n' "$( command grep -c 'ring_stats' ring_overflow/Cargo.toml )"
```

Live output:

```
  -- the readme dependency sentence --
Depends on [`ring_types`](../ring_types/readme.md) and
[`ring_stats`](../ring_stats/readme.md).
  -- task 113 Depends-on line --
**Depends on:** `ring_types`, `ring_stats`
  -- ring_stats named in each of those two, and in the manifest a build enforces --
    readme dependency sentence : 1
    task 113 Depends-on line   : 1
    Cargo.toml                 : 1
```

**The recipe had to be narrowed, and the reason is this document.** It used to
count `ring_stats` across the whole of both files and assert both counts were
one. Task 113 then gained an `## Implementation Record` — a section written to
record that the `Depends-on` line had been corrected — and its sentence naming
`ring_stats` as genuinely used pushed the whole-file count from 1 to 2. Nothing
regressed: line 19 still reads `**Depends on:** ring_types, ring_stats`. The
measurement broke because it was measuring the file when the claim was about
one line of it, so a paper trail describing the fix was indistinguishable from
a second, unwanted mention appearing. The same narrowing landed on
[`ring_event`'s twin finding](../../../ring_event/docs/workaround/002_a_third_dependency_two_documents_still_name.md)
in the same pass and for the same reason, with the polarity reversed — there
the corrected line had to stop naming a crate, here it had to start.

**Disposition:** applied — both one-line corrections this instance names are
made: `ring_overflow/readme.md`'s dependency sentence now names
`ring_stats` alongside `ring_types`, and
`ring_overflow/task/unverified/113_implement_ring_overflow.md`'s
`**Depends on:**` line does the same, against a manifest that already declared
it. The task file's own state is left untouched — moving it out of
`unverified/` is a task-lifecycle action this corpus disposition pass does not
perform. Now prints: `    task 113 Depends-on line   : 1`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/001`](../workaround/001_the_recorder_forecloses_const.md) | What the omitted dependency actually causes |
| [`integration/002`](../integration/002_the_stats_edge_and_the_function_nobody_imports.md) | The edge itself, and how little travels it |
| [`lifecycle/001`](001_a_resolution_computed_matched_and_discarded.md) | The lifecycle of the values, rather than the crate |

### Sources

| Fact | Where |
|------|-------|
| The declared task state and empty Scope | `ring_overflow/task/unverified/113_implement_ring_overflow.md:3`, `:23-25` |
| 15 tests against a 237-line crate | Census above |
| The three disagreeing dependency lists | `ring_overflow/src/lib.rs:5`, `readme.md:5`, task file `:19` |
| The two dependencies actually declared | `ring_overflow/Cargo.toml:9-10` |
| 33 of 33 implement-tasks in the first Open state | Census above |

### Tests

| Test | Covers |
|------|--------|
| `exactly_one_counter_moves_per_call` | The `ring_stats` use the readme omits |
| `resolve_agrees_with_would_resolve` | The duplication the omitted dependency explains |
| `exactly_one_counter_moves_per_call` | That the edge is load-bearing, not incidental |
