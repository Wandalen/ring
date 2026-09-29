# Invariant Doc Definition

### Scope

- **Purpose**: Properties that hold across the crate, each with the measurement that establishes it and the boundary it stops at.
- **Responsibility**: Two — what cannot be reached from the tick path, and what a budget does and does not bound — plus, for each, the gap between the property claimed and the property checked.
- **In Scope**: The parking-crate roster and what it watches; the attempts-not-time distinction and the one helper exempt from it.
- **Out of Scope**: The family's single liveness flag (→ [`ring_shutdown/docs/invariant/001`](../../../ring_shutdown/docs/invariant/001_exactly_one_liveness_flag.md)); what a caller should do about a large budget (→ [`../pitfall/002`](../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [No Parking Operation Is Reachable From the Tick Path](001_no_parking_operation_on_the_tick_path.md) | This crate's structural claim, the graph measurement that carries it, and the vocabulary that arrived anyway | 🔄 |
| 002 | [A Budget Bounds Attempts, Not Time](002_a_budget_bounds_attempts_not_time.md) | The weaker property that is actually true, kept apart from the one it is mistaken for, and the helper it does not cover | 🔄 |

**One is about the graph, one is about the loop.** `001` is a claim nothing in
this crate's code can establish — it is a property of the family's dependency
edges, asserted here because this is where the feature lives. `002` is a claim
the source establishes by construction: the loop bound is read before the loop
starts. Neither can be folded into the other, because the first fails when
another crate's manifest changes and the second fails when this crate's control
flow does.

Both findings under each file follow from the same pressure. A whole-crate
invariant is written once and read rarely, while the thing it generalises over
keeps moving — so `001`'s pair is about a set drifting away from the copies of
it, and `002`'s about a generalisation quietly acquiring an exception.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll/docs/invariant
printf 'instances:                  %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:    %s\n' "$( command grep -hoE '^### PL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:    %s\n' "$( command grep -coE '^\| PL[0-9]+ ' readme.md )"
printf 'each instance has a recipe: %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each states its invariant:  %s\n' "$( command grep -lc '^### Invariant Statement' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each names its enforcement: %s\n' "$( command grep -lc '^### Enforcement Mechanism' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'structurally enforced:      %s\n' "$( command grep -l 'structural' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'enforced by test only:      %s\n' "$( command grep -l 'roster cannot drift\|asserted against the' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'helpers the crate exposes:  %s\n' "$( command grep -cE '^pub fn [a-z_]+' ../../src/lib.rs || true )"
printf 'of those, budget-bounded:   %s\n' "$( command grep -oE '^pub fn [a-z_]+' ../../src/lib.rs | sed 's/^pub fn //' | while read -r n; do awk -v n="$n" '$0 ~ "^pub fn "n"$" || $0 ~ "^pub fn "n"<" {f=1} f{print} f&&/^\{$/{exit}' ../../src/lib.rs | command grep -q 'Budget' && echo x; done | wc -l )"
```

Live output:

```
instances:                  2
finding headings inside:    4
rows in the table below:    4
each instance has a recipe: 2
each states its invariant:  2
each names its enforcement: 2
structurally enforced:      1
enforced by test only:      1
helpers the crate exposes:  4
of those, budget-bounded:   3
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PL21 | the hazard's vocabulary is reachable; only its implementation is not | n/a — observation | The invariant is stated over crates and `ring_wait` is unreachable at any depth, but `WaitKind` — whose `Yield` and `Park` variants are the hazard — is owned by `ring_types`, which sits in this crate's normal closure, so `WaitKind::Park` is already spellable here; a future `ring_core` operation taking a `WaitKind` would put a parking *request* on the tick path with no crate gaining a `ring_wait` edge, and both roster crates are already conduits of exactly that shape, taking a caller-supplied `kind` rather than choosing one. |
| PL22 | the set that is expected to change is written out by hand | n/a — duplication | `{ ring_barrier, ring_shutdown, ring_wait }` was hand-typed in the Purpose bullet, in the Invariant Statement, and as the enforcement check's expected output — *"expect exactly three lines"*, a pass condition typed directly beneath the command meant to produce it — while `PARKING_CRATES` in source is the only copy anything asserts against, so a correct roster edit leaves the document describing the previous family and the typed "three" reads green against a set the corpus measures at five. |
| PL23 | a whole-crate invariant generalising over a function the source exempts | n/a — inconsistency | The statement claimed every helper returns within `budget.attempts()` ring operations, and `drain_up_to` takes no `Budget` at all — it is bounded by `while taken < max`, a caller-supplied `usize` with none of `Budget`'s clamping — while `Tick::drain`'s own doc comment argues the exemption correctly and at length, so the generalisation and its exception are both written down, in different files, with nothing checking them against each other. |
| PL24 | the stated margin catches the known failure and hides the unknown one | n/a — coverage | The wall-clock test bounds 20 000 attempts at 500 ms and writes the sleeping-side margin into its failure message (*"would cost about 1.0 s"*), while the spinning-side margin — single-digit milliseconds, roughly 100× under the ceiling — appears in neither the test nor its message, so a pure-spin loop degrading from 5 ms to 400 ms passes silently; this file previously claimed both margins were stated in the test. |
