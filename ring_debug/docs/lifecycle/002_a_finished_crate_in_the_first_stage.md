# Lifecycle: A Finished Crate in the First Stage

### Scope

- **Purpose**: Record the crate's own lifecycle — the stages its implementation task moves through — against the state that task is actually in.
- **Responsibility**: The task's stages, which one holds this crate, what each of the three records of the crate's state says, and why they disagree.
- **In Scope**: `task/unverified/126_implement_ring_debug.md`; the `unverified` → `verified` → executed progression; the family ledger row; the 33-crate family condition.
- **Out of Scope**: `Watch`'s runtime state machine (→ [`lifecycle/001`](001_from_one_observation_to_a_sequence.md)); what the crate does (→ [`api/001`](../api/001_the_check_surface.md)).

### States

A task lives in a subdirectory of `task/`, and the directory *is* the state. The
universal Open-state pair is `unverified` and `verified`; a task cannot be claimed
until it has passed a readiness gate that moves it to the second.

| # | State | Directory | Meaning |
|---|-------|-----------|---------|
| L0 | ❓ **Unverified** | `task/unverified/` | Filed. Not claimable — Scope has not yet named concrete deliverables |
| L1 | 🎯 **Verified** | `task/verified/` | Readiness gate passed; claimable |
| L2 | ✅ **Executed** | `task/executed/` | Work done and recorded |

**This crate's task is in L0.** Its Scope section reads *"Not yet worked out"* and
its Verification section reads *"Not yet defined — see Scope."*

### Transitions

| # | From | To | Trigger | Taken here |
|---|------|----|---------|------------|
| U1 | L0 | L1 | Readiness verification gate passes | No |
| U2 | L1 | L2 | The task is claimed and the work is done | No |
| U3 | — | — | The work is done **without** claiming the task | Yes |

U3 is not a transition in the model. It is what happened: the crate was built,
tested, documented and gated under a staged-validation run that tracks stages
rather than tasks. The task file records this itself, in a section added for the
purpose: the work was carried out under that staged-validation run rather than by
claiming this task, which remains `❓ Unverified` along with all 33 ring tasks —
recorded there so the task file does not describe the crate as unbuilt.

#### Three records, two answers

The crate's state is written down in three places.

| Record | Says | Where |
|---|---|---|
| The task's state field and directory | Not yet scoped, not claimable | `task/unverified/126_…` |
| The same file's Implementation Record | Built, 22 tests, gate 6/6, acceptance criterion satisfied | the same file |
| The family module ledger | `●` implemented · `🟩` green | [`docs/crate/readme.md`](../../../../docs/crate/readme.md) |

Two of the three agree with each other and with the directory listing. The one that
disagrees is the one the task system treats as authoritative, because in this
system the location of the file is the state — there is no field to correct
independently of moving it.

**This is a family condition, not an oversight in one crate.** All 33 `ring_*`
tasks are in `unverified/`; none has taken U1. The staged run and the task system
are two parallel records of the same work, and only one of them was being kept.

### Failure

| # | Failure | Consequence |
|---|---------|-------------|
| N5 | Reading the task state without reading the file | The crate reads as unstarted; a second worker could file duplicate work |
| N6 | Reading the Implementation Record without re-measuring | An inventory written at one moment is taken as current — see DB40 |
| N7 | Taking U1 now | The readiness gate would be asked to verify scope for work already finished, which is not what the gate is for |

**N7 is the reason this is recorded rather than fixed from inside the crate.**
Moving one task out of `unverified/` while the other 32 stay would make this crate
the exception to a family-wide condition without changing anything about the
family's actual bookkeeping. Whether the ring tasks should be gated retroactively
or closed as executed is a decision for the family as a whole, not this crate.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '-- the task file: where it lives, and what it says about itself --'
find ring_debug/task -mindepth 2 -name '*.md' | sed 's/^/  /'
command grep '^- \*\*State:\*\*' ring_debug/task/unverified/*.md | sed 's/^/  /'
awk '/^## Scope/{ f = 1; next } /^## /{ f = 0 } f && NF' ring_debug/task/unverified/*.md | head -3 | sed 's/^/  /'
echo '-- and what the same file records further down --'
awk '/^## Implementation Record/{ f = 1 } f && ( /Doc instances/ || /Gate verdict/ || /tests/ )' ring_debug/task/unverified/*.md | cut -c1-180 | sed 's/^/  /'
echo '-- that inventory, against the directory --'
printf '  definitions the record names:  %s\n' "$( awk '/^## Implementation Record/{ f = 1 } f && /Doc instances/' ring_debug/task/unverified/*.md | command grep -oE '[a-z_]+/' | sort -u | tr '\n' ' ' )"
printf '  definitions present on disk:   %s\n' "$( ls -d ring_debug/docs/*/ | xargs -n1 basename | tr '\n' ' ' )"
printf '  instances present on disk:     %s\n' "$( ls ring_debug/docs/[a-z_]*/[0-9][0-9][0-9]_*.md | wc -l )"
echo '-- the third record --'
command grep '| `ring_debug` |' docs/crate/readme.md | cut -c1-180 | sed 's/^/  /'
echo '-- and how many of the 33 ring tasks have left the first stage --'
for d in ring_*/task ; do find "$d" -mindepth 2 -name '*.md' ; done | sed 's|.*/task/||;s|/.*||' | sort | uniq -c | sed 's/^/  /'
```

Live output:

```
-- the task file: where it lives, and what it says about itself --
  ring_debug/task/unverified/126_implement_ring_debug.md
  - **State:** ❓ Unverified
  Not yet worked out. This task is `❓ Unverified` — it must pass the readiness
  verification gate before it can be claimed, and that gate requires this Scope
  section to name concrete deliverables rather than restate the goal.
-- and what the same file records further down --
  | `tests/debug_test.rs` | 22 tests + 3 doc tests, all passing |
  | Doc instances | 8, across `algorithm/ api/ integration/ invariant/ non_functional_requirement/ pitfall/ state_machine/ type/`, plus a decisions register (4 closed, 3 pending) |
  | `tests/manual/readme.md` | 6 stages, run 2026-08-28, 1 prediction wrong |
  **Gate verdict (2026-08-28):** `GATE_CRATES=ring_debug GATE_FEATURES=185
-- that inventory, against the directory --
  definitions the record names:  algorithm/ api/ integration/ invariant/ non_functional_requirement/ pitfall/ state_machine/ type/ 
  definitions present on disk:   algorithm api data_structure decisions definition integration invariant item lifecycle non_functional_requirement pattern pitfall type workaround 
  instances present on disk:     26
-- the third record --
  | — | `ring_debug` | Runtime invariant checks over a live ring | 4 | 420 | 5/5 | 22 | debug_test.rs | ●∅🟩 —🟢 |
-- and how many of the 33 ring tasks have left the first stage --
       33 unverified
```

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_from_one_observation_to_a_sequence.md](001_from_one_observation_to_a_sequence.md) | The other lifecycle in this crate — `Watch`'s, at run time |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_edges_that_were_never_drawn.md](../integration/002_the_edges_that_were_never_drawn.md) | DB31 — the other measurement of a complete argument that nobody acted on |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/readme.md](../decisions/readme.md) | Pending 1 — the open question the task file cites as its known limitation |

### Sources

| File | Relationship |
|------|--------------|
| [`task/unverified/126_implement_ring_debug.md`](../../task/unverified/126_implement_ring_debug.md) | The task, its state, and its Implementation Record |
| [`task/readme.md`](../../task/readme.md) | The local task index row, and its `last_sync` stamp |
| [`docs/crate/readme.md`](../../../../docs/crate/readme.md) | The family module ledger row |

### Tests

| Test | Relationship |
|------|--------------|
| `a_consumer_ahead_of_its_producer_is_caught` | The acceptance criterion the Implementation Record reports as satisfied |
| `the_two_ends_of_a_live_ring_agree` | Part of the 22 the record counts |

### DB39 — the task is in the state that means "not yet scoped" and contains a gate verdict for the finished work

`126_implement_ring_debug.md` lives in `task/unverified/`, carries
`- **State:** ❓ Unverified`, and its Scope section says *"Not yet worked out…
that gate requires this Scope section to name concrete deliverables rather than
restate the goal."*

Twenty lines later the same file reports a deliverable table, 22 tests plus 3 doc
tests, a gate verdict of **6/6 reached** with 100% line coverage on G1, and a named
acceptance criterion satisfied by a named test.

Both halves are accurate. The task genuinely was never scoped, because it was never
claimed — the work went through the staged run instead — and the Implementation
Record was added precisely so the file would not read as describing an unbuilt
crate. **The result is a file whose state field and whose contents are each honest
and mutually contradictory**, and the state field is the one a reader scanning
`task/readme.md`'s index sees.

Recorded as an inconsistency rather than a defect because the resolution is not
local: all 33 `ring_*` tasks are in `unverified/`, so the disagreement is between
two parallel records of the same work — the staged-validation run's ledger and the
task system — and only one of them has been kept current. Deciding which one owns
the truth is a decision for the family as a whole, not a question this crate can
answer about itself.

### DB40 — the record written to stop the file being wrong is now wrong in the other direction

The Implementation Record's own justification is *"Recorded here so the task file
does not describe the crate as unbuilt."* Its inventory line reads:

> Doc instances | 8, across `algorithm/ api/ integration/ invariant/
> non_functional_requirement/ pitfall/ state_machine/ type/`

Measured against the directory: seven of the eight it names are still there and
`state_machine/` is not — that definition was renamed to `lifecycle/`, and the
record still names the directory the crate had at the moment it was written. Seven
more directories exist that it does not name. Its instance figure is likewise the
count as of that day; the recipe above prints today's.

The mechanism is the finding rather than the numbers. **A hand-written inventory
inside a file in `unverified/` is on no regeneration path**: nothing measures it,
no gate reads it, and the file is in the one state that means nobody should be
working on it. It was correct when written and had no way to stay correct, so the
section added to keep the file from understating the crate now understates it by
seven directories and a rename.

Every other inventory in this corpus is a `### Regenerate` block whose numbers are
produced by the command printed beside them. This one is prose, and the difference
in outcome is visible in one measurement.
