# Lifecycle: A Finished Crate in the Unverified Stage

### Scope

**Purpose:** Read the crate's own position in the work-item lifecycle against
what it has actually shipped, and establish what the governing task file does and
does not record.

**Responsibility:** Task 115's state marker, its empty Scope and Verification
sections, the stage every ring task sits in, and the implementation, suite and
doc corpus the task describes as not yet worked out.

**In Scope:**
`ring_event/task/unverified/115_implement_ring_event.md`;
`ring_event/Cargo.toml:9-13`; `ring_event/src/lib.rs`;
`ring_event/tests/event_test.rs`.

**Out of Scope:** The dependency the task names and the manifest does not is
[`workaround/002`](../workaround/002_a_third_dependency_two_documents_still_name.md).
Who actually depends on this crate is
[`integration/001`](../integration/001_one_declarer_and_it_is_a_dev_dependency.md).

---

## The Task Against the Crate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the governing task says about this crate --'
# both ranges are anchored on their own last line rather than on a line offset
# or on end-of-file: the open-ended tail this replaces began printing an
# `## Implementation Record` the moment one was appended below the Verification
# section it was meant to stop at
awk '
  /^# Task 115: Implement `ring_event`$/ { p = 1 }
  p && /^- \*\*Unit:\*\*/ { print; p = 0; next }
  p { print; next }
  /^\*\*Depends on:\*\*/ { q = 1; print ""; }
  q { print }
  /^Not yet defined/ { exit }
' ring_event/task/unverified/115_implement_ring_event.md
echo '  -- against what the manifest declares --'
command grep 'ring_' ring_event/Cargo.toml
echo '  -- which lifecycle stage every ring task sits in --'
for f in ring_*/task/*/*.md; do echo "$f" | cut -d/ -f4; done | sort | uniq -c
echo '  -- stage directories per crate, against files in them --'
ls -d ring_event/task/*/ | wc -l
find ring_event/task -mindepth 2 -type f | wc -l
echo '  -- and what the crate has actually shipped --'
wc -l < ring_event/src/lib.rs
command grep -c '^#\[ test \]' ring_event/tests/event_test.rs || true
command grep -c '^/// ```' ring_event/src/lib.rs || true
```

Live output:

```
  -- what the governing task says about this crate --
# Task 115: Implement `ring_event`

- **State:** ❓ Unverified
- **Executor:** any
- **UnitType:** module
- **Unit:** lib/yrd_gamedev/substrate/ring/ring_event

**Depends on:** `ring_types`, `ring_slot`

## Scope

Not yet worked out. This task is `❓ Unverified` — it must pass the readiness
verification gate before it can be claimed, and that gate requires this Scope
section to name concrete deliverables rather than restate the goal.

## Verification

Not yet defined — see Scope.
  -- against what the manifest declares --
name = "ring_event"
ring_types = { path = "../ring_types" }
ring_slot = { path = "../ring_slot" }
ring_store = { path = "../ring_store" }
  -- which lifecycle stage every ring task sits in --
     33 unverified
  -- stage directories per crate, against files in them --
12
1
  -- and what the crate has actually shipped --
234
17
10
```

The last three numbers are the source file's line count, its suite's `#[ test ]`
count, and its doctest fence count — ten fences being five doctests.

## What the Suite Reports

`cargo test -p ring_event --all-features`, run detached and polled to completion:

```
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 17 tests
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 5 tests
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**That block is a quote, not a recipe, and it went stale for exactly that
reason.** It said `16` for as long as the seventeenth test had existed, because
nothing re-runs a bare fence — a recipe here would make a documentation gate
build and test the crate, which is why it is not one. What makes the staleness
recoverable rather than permanent is that the same number is *also* derivable
from the gated block above: its `#[ test ]` count is the seventeen this quote
has to agree with, and a reader comparing the two finds the disagreement
without running anything. Every number in this quote is checkable that way, and
that is the property being asked of an ungated quote — not that it cannot
drift, but that its drift is visible from something that cannot.

---

### EV31 — The Work Item Says the Scope Is Not Yet Worked Out and the Work Is Done

Task 115 carries `**State:** ❓ Unverified`, which in this lifecycle is the first
of two Open states: a task that must pass a readiness gate before anyone may
claim it. Its Scope section says "Not yet worked out" and explains that the gate
requires concrete deliverables to be named there first. Its Verification section
says "Not yet defined — see Scope."

The crate those sections describe is a 234-line library with two traits, three
free functions, five doctests and a seventeen-test suite, all passing, plus the
doc corpus this file belongs to. It is not unclaimed work awaiting a scope; it is
finished work whose scoping document was never filled in.

Two of those four numbers were wrong here until the gate above forced a reread.
The library was 204 lines and the suite had sixteen tests when this paragraph
was written, and both had moved before anyone looked again — the surface shape,
two traits and three free functions, is what stayed put. The reason the drift
survived is worth stating plainly, because it is the same reason twice: prose
and a quoted transcript are the two things in this document that nothing
recomputes, and both of them carried the old numbers while the recipe eight
lines above printed the new ones. A gate that checks recipes finds a document
that disagrees with itself perfectly readable.

This is not local. All 33 of the family's task files sit in `unverified` — the
census finds no other stage occupied anywhere — and each crate carries twelve
stage directories with exactly one file among them.

**Finding.** The lifecycle is doing no work here. A state marker that reads
`❓ Unverified` on every crate, whether the crate is a skeleton or a complete
implementation with a green suite, carries no information: it cannot distinguish
the two, and a reader who trusts it is told the opposite of the truth about this
one. Advancing the marker is not the interesting part — what is missing is that
the file has no record of what was actually built, so the only account of this
crate's deliverables is the crate itself.

**Disposition:** declined — this instance's own text names the correct venue
for a fix and it is not this corpus disposition pass: "Recording the systemic
version here would be the wrong place for it — it belongs to whatever governs
the family's task set, not to this leaf." Advancing
`ring_event/task/unverified/115_implement_ring_event.md`'s lifecycle
state, or filling its Scope section to pass the readiness gate, is a
task-lifecycle action outside a documentation-corpus disposition — the same
boundary already drawn for `ring_overflow`'s
`docs/lifecycle/002_implemented_tested_and_still_planned.md` § OV48, whose
task file state was left untouched for the identical reason.

---

### EV32 — The Verification Section Is Empty and the Verification Happened

The task file's Verification section is the artifact that would say what "done"
means for this crate. It says "Not yet defined."

Meanwhile something did verify the crate. Seventeen integration tests exercise
both slot shapes through one generic body, the refusal path, both recycling
paths, the zero-length reading and the emptiness invariant. Five doctests run
on every public item but `Peek::Out`. All twenty-two pass. Whatever standard
those were written to, it existed, and it was applied.

The gap is that nothing connects the two. The tests do not reference the task, the
task does not reference the tests, and the criteria the suite was built against —
whichever they were — are recorded in neither. The task file's own dependency
line is evidence of the disconnect in the other direction: it names `ring_types`,
`ring_slot` and `ring_seqno`, while the manifest declares `ring_types` and
`ring_slot` and no `ring_seqno` at all
([`workaround/002`](../workaround/002_a_third_dependency_two_documents_still_name.md)),
so the file describes a shape the crate does not have and has not been revisited
since it stopped being true.

**Finding.** The crate is verified and its verification is unrecorded, which is a
different problem from being unverified and looks identical from the task file.
Filling the Verification section with what the suite already covers — both shapes
through one body, the refusal, the two recycles, the emptiness agreement — costs
nothing to discover, because the tests are named after the properties they check,
and would turn twenty-two passing assertions into a stated standard the next
change can be held to.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/001`](001_four_positions_three_representations_two_observable_states.md) | The other lifecycle, one level down |
| [`workaround/002`](../workaround/002_a_third_dependency_two_documents_still_name.md) | The dependency both documents still name |
| [`integration/001`](../integration/001_one_declarer_and_it_is_a_dev_dependency.md) | What the crate's position in the graph actually is |
| [`invariant/002`](../invariant/002_a_refused_fill_changes_nothing.md) | One of the properties an empty Verification section leaves unstated |

### Sources

| Fact | Where |
|------|-------|
| `**State:** ❓ Unverified` | `ring_event/task/unverified/115_implement_ring_event.md:3` |
| "Not yet worked out" / "Not yet defined" | Same file, `## Scope` and `## Verification` |
| A third dependency the manifest does not declare | Same file, `**Depends on:**`; `ring_event/Cargo.toml:9-13` |
| All 33 ring tasks in one stage | Census above |
| Twelve stage directories holding one file | Census above |
| 234 lines, 17 tests, 5 doctests, all passing | Census and suite output above |

### Tests

| Test | Covers |
|------|--------|
| `both_shapes_land_in_storage_through_the_same_two_calls` | The property an empty Verification section would have named first |
| `peek_agrees_with_the_slots_own_emptiness` | A second, stated only in the test's own comment |
| `recycling_empties_either_shape_through_the_same_call` | A third, for the operation with no outside caller |
