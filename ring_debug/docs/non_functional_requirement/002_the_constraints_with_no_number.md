# Non-Functional Requirement: The Constraints With No Number

### Scope

- **Purpose**: Audit what actually enforces each constraint in [`non_functional_requirement/001`](001_absent_unless_called.md), and record which of them has no number and no route to one.
- **Responsibility**: Per constraint, the guard the requirement names, the thing that executes it, and how often that happens.
- **In Scope**: C1–C5 and Q1–Q4 as enforcement claims; `tests/manual/readme.md`'s six stages; the deferred latency figure's named home.
- **Out of Scope**: The constraints themselves and why they were chosen (→ [`non_functional_requirement/001`](001_absent_unless_called.md)); the fence that hides two of the guards (→ DB46, recorded there).

### Requirement

[`non_functional_requirement/001`](001_absent_unless_called.md) states five
constraints and, for each, a measurement. **A measurement that nothing runs is a
statement about intent, not a constraint.** This instance separates the two.

### Constraints

| # | Constraint | Guard the requirement names | What executes it | How often |
|---|---|---|---|---|
| E1 | C1 — no reverse manifest edge | A grep | `tests/manual/readme.md` M1, by hand; and [`integration/002`](../integration/002_the_edges_that_were_never_drawn.md)'s `sh` recipe, by the corpus gate | Every gate run |
| E2 | C2 — no family `src/` mention | A grep | M2, by hand | On request |
| E3 | C3 — O(1), allocation-free | Inspection | Nothing | Never |
| E4 | C4 — nothing runs implicitly | Inspection, *"nothing greps for it"* | **M3, by hand** — see DB47 | On request |
| E5 | C5 — checks do not perturb | `checking_leaves_both_cursors_where_they_were` | `cargo nextest` | Every test run |

Two of the five have a guard something runs unprompted. Two more have a guard that
exists and waits for a person. One has none.

**E3 is the only genuine hole, and it is two holes.** C3 makes two claims — that
every check is O(1), and that every check allocates nothing — and neither is
falsifiable by anything in the family as it stands.

#### The complexity half

The requirement defers a latency figure explicitly: *"If that changes, the
measurement belongs in `ring_bench`, which is where the family's timing
machinery lives."* That is the right home in principle. In practice
`ring_bench` takes no dependency on this crate — so the deferral points at a
crate that cannot reach the code.

This is the same untaken edge [`integration/002`](../integration/002_the_edges_that_were_never_drawn.md)
measures at zero across all 33 manifests. The consequence here is narrower and
sharper: the missing measurement has a named owner who has not been told.

#### The allocation half

Nothing in this crate can observe an allocation. A counting or recording global
allocator is the standard way to turn "allocates nothing" into a test that fails
when it stops being true, and `ring_debug` has none. Four crates in this same
family already do — `ring_barrier`, `ring_claim`, `ring_consume` and
`ring_cursor` each carry one in `tests/allocation_test.rs` — and
the repository as a whole carries thirteen across
seven crates. The technique is not
merely available in this repository, it is already in use next door, which makes
its absence here a gap rather than a missing capability.

The claim is almost certainly true — the checks are comparisons over `u64` — and
that is exactly why it is worth flagging rather than fixing: an obviously-true
claim with no falsifier is indistinguishable, later, from an obviously-true claim
that quietly stopped being true.

### Failure

| # | Failure | How it presents |
|---|---------|-----------------|
| R1 | C3 stops holding | Nothing changes. No guard, no test, no figure — the requirement reads exactly as before |
| R2 | A reader trusts Q4's *"no mechanical guard"* | They build the guard M3 already is, or they treat C4 as unchecked when it is checked |
| R3 | The manual plan stops being run | E2 and E4 silently join E3; the Run Record is the only signal, and it is a date |

**R3 is the structural one.** Three of the five constraints are guarded by a
document a person runs, and the evidence that it was run is a date in a Run Record
— `2026-08-28`, with one prediction recorded wrong. That is a real record and a
better one than nothing, and it degrades in a way an automated gate does not: by
staying exactly as convincing while getting older.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
D=ring_debug
SELF=002_the_constraints_with_no_number
echo '-- what 001 claims about Q4 --'
command grep 'not automated\|has a guard, but not' $D/docs/non_functional_requirement/001_absent_unless_called.md | cut -c1-140 | sed 's/^/  /'
echo '-- the manual plan stages --'
command grep -E '^## M[0-9]' $D/tests/manual/readme.md | cut -c1-110 | sed 's/^/  /'
echo '-- and what M3 actually runs --'
awk '/^## M3/{ f = 1 } f && /^## M4/{ exit } f && /grep/{ print "  " $0 }' $D/tests/manual/readme.md | cut -c1-150
echo '-- the Run Record date --'
awk '/^## Run Record/{ f = 1 } f && NF { print "  " $0 }' $D/tests/manual/readme.md | head -6 | cut -c1-140
echo '-- guards for C1 and C2 elsewhere in this crate docs (excluding this file) --'
command grep -r 'grep' $D/docs --include=*.md | command grep -v "$SELF" | sed 's|^ring_debug/docs/||' | command grep 'Cargo.toml' | command grep 'ring_debug' | cut -c1-120 | sed 's/^/  C1: /'
command grep -r "\\*/src" $D/docs --include=*.md | command grep -v "$SELF" | sed 's|^ring_debug/docs/||' | command grep 'ring_debug' | cut -c1-130 | sed 's/^/  C2: /'
echo '-- the home the deferred figure was sent to --'
printf '  ring_bench manifest naming ring_debug: %s\n' "$( command grep -c 'ring_debug' ring_bench/Cargo.toml || true )"
echo '-- and what would falsify the allocation half --'
printf '  global allocators in ring_*:      %s\n' "$( command grep -rn '#\[ *global_allocator *\]' --include=*.rs ring_*/ | wc -l )"
printf '  global allocators in an unrelated crate root: %s\n' "$( command grep -rn '#\[ *global_allocator *\]' --include=*.rs . | wc -l )"
```

Live output:

```
-- what 001 claims about Q4 --
  | Q4 | The crate grows a `Drop` or a lazily-initialised global | `tests/manual/readme.md`'s M3 stage greps for it by hand — the only guard
  **Q4 has a guard, but not an automated one.** `tests/manual/readme.md`'s M3
-- the manual plan stages --
  ## M1 — Does anything in the family depend on this crate?
  ## M2 — Does any family `src/` mention this crate?
  ## M3 — Does anything here run implicitly?
  ## M4 — Can the strongest check be pointed at a live `ring_core::Ring`?
  ## M5 — Is the crate fully covered?
  ## M6 — Are the three values reachable from a `ring_spsc` ring, even though a `CursorPair` is not?
-- and what M3 actually runs --
  grep -n 'impl Drop\|\<static \|lazy_static\|OnceLock\|\<ctor\>' ring_debug/src/lib.rs
-- the Run Record date --
  ## Run Record
  | Stage | Question | 2026-08-28 |
  |---|---|---|
  | M1 | Nothing depends on this crate | ✅ one line |
  | M2 | No family `src/` mentions it | ✅ empty |
  | M3 | Nothing runs implicitly | ✅ no match |
-- guards for C1 and C2 elsewhere in this crate docs (excluding this file) --
  C1: non_functional_requirement/001_absent_unless_called.md:| C1 | No family crate depends on `ring_debug` | `command grep -r
  C1: non_functional_requirement/001_absent_unless_called.md:command grep -rln 'ring_debug' divisio
  C1: workaround/002_one_cast_between_two_newtypes_that_disagree.md:  "$( command grep -rl 'ring_debug' --include=*.rs --inclu
  C1: integration/001_reaching_the_cursors_of_a_live_ring.md:  "$( command grep -c 'features' ring_debug/Cargo.toml || tr
  C1: integration/002_the_edges_that_were_never_drawn.md:sed -n '/^\[dependencies\]/,/^\[/p' ring_debug/Cargo.toml | comm
  C1: integration/002_the_edges_that_were_never_drawn.md:printf '  %s\n' "$( command grep -rl 'ring_debug' --include=Cargo.tom
  C1: integration/002_the_edges_that_were_never_drawn.md:    "$( command grep -c 'ring_debug' $c/Cargo.toml || true )"
  C1: integration/002_the_edges_that_were_never_drawn.md:printf '  ring_testkit naming ring_debug: %s\n' "$( command grep -c '
  C1: integration/002_the_edges_that_were_never_drawn.md:printf '  ring_debug naming ring_testkit: %s\n' "$( command grep -c '
  C1: decisions/001_four_edges_not_two.md:sed -n '/^\[dependencies\]/,/^\[/p' ring_debug/Cargo.toml | command grep -E '^r
  C2: non_functional_requirement/001_absent_unless_called.md:| C2 | No family `src/` mentions `ring_debug` | `command grep -rn --include
  C2: non_functional_requirement/001_absent_unless_called.md:command grep -rn --include=*.rs 'ring_debug' division/
-- the home the deferred figure was sent to --
  ring_bench manifest naming ring_debug: 0
-- and what would falsify the allocation half --
  global allocators in ring_*:      4
  global allocators in an unrelated crate root: 13
```

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_absent_unless_called.md](001_absent_unless_called.md) | C1–C5 and Q1–Q4 — the constraints this audits |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_edges_that_were_never_drawn.md](../integration/002_the_edges_that_were_never_drawn.md) | DB31 — the untaken edge that also blocks the deferred benchmark |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_a_finished_crate_in_the_first_stage.md](../lifecycle/002_a_finished_crate_in_the_first_stage.md) | DB40 — the other hand-maintained record that degrades by staying still |

### Sources

| File | Relationship |
|------|--------------|
| [`tests/manual/readme.md`](../../tests/manual/readme.md) | M1–M6 and the Run Record |

### Tests

| Test | Relationship |
|------|--------------|
| `checking_leaves_both_cursors_where_they_were` | E5 — the one guard that is a test rather than a recipe, and the only one `cargo nextest` runs |

### DB47 — the one gap the requirement flags as unguarded is guarded, by a stage in the same crate

Q4 reads: *"The crate grows a `Drop` or a lazily-initialised global — C4 fails by
inspection; **nothing greps for it**."* A paragraph then defends the absence:
*"Q4 has no mechanical guard, and is recorded as such rather than left to look
covered. A gate that detected implicit invocation would need to reason about
initialisation order across the family, which is more machinery than the risk
warrants."*

`tests/manual/readme.md` stage M3 is titled *"Does anything here run implicitly?"*
and runs

```
grep -n 'impl Drop\|\<static \|lazy_static\|OnceLock\|\<ctor\>' ring_debug/src/lib.rs
```

which is precisely the grep Q4 says does not exist. The defence is also sound about
a *different* guard — a cross-family initialisation-order gate would indeed be
disproportionate — and the guard that exists is the cheap single-file one, which
nobody argued against.

Recorded as a misleading doc because the error is in the direction that costs
something. **A stated gap invites work; this one invites building a guard the crate
already has**, and the same paragraph tells a reader that C4 is checked by nothing,
when it is checked by a listed, dated stage of the crate's own test plan.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -F '| Q4 |' ring_debug/docs/non_functional_requirement/001_absent_unless_called.md
```

Live output:

```
| Q4 | The crate grows a `Drop` or a lazily-initialised global | `tests/manual/readme.md`'s M3 stage greps for it by hand — the only guard here that is not automated |
```

**Disposition:** applied — `non_functional_requirement/001`'s Q4 row and the
paragraph beneath it no longer state that nothing greps for an implicit-invocation
regression; both now name `tests/manual/readme.md`'s M3 stage as the guard that
already runs this exact grep by hand, while keeping the (still valid) point that
an automated cross-family gate would be disproportionate. Now prints:
`the only guard here that is not automated`

### DB48 — the latency claim with no number was deferred to a crate that cannot reach it

C3 asserts two properties and measures neither. Each was deferred, and both
deferrals land somewhere that has no route to the code.

- **Latency.** *"The measurement belongs in `ring_bench`."* `ring_bench`'s
  manifest names this crate zero times — the benchmark home has no dependency
  edge back to it at all.
- **Allocation.** Nothing in `ring_debug` can observe an allocation, though four
  crates in this family already can — `ring_barrier`, `ring_claim`,
  `ring_consume` and `ring_cursor` each carry a counting global allocator in
  `tests/allocation_test.rs`. The technique is one this family already uses and
  this crate has not.

Both claims are very likely true, and neither can be shown false by any artefact
that exists. **A constraint that cannot fail is not being enforced; it is being
believed**, which is the same relationship this crate exists to replace for cursor
invariants — [`pitfall/001`](../pitfall/001_saturating_arithmetic_reports_health.md)
records the family arithmetic reporting health it cannot actually verify, and C3 is
that shape one level up, in the document that sets the crate's own budget.

The deferral is not wrong as a decision. A latency figure for two atomic loads
really would measure the harness, as the requirement says. The finding is that
naming a home is not the same as filing the work there, and it never was.
