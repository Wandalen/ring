# Integration Doc Definition

### Scope

- **Purpose**: Edges into and out of this crate, each justified by a named use site, the absences that are deliberate, and the difference between the edges that are declared and the ones that exist.
- **Responsibility**: One runtime dependency, two dev-dependencies, four pointed absences, zero consumers, and the transitive closure that the family's own guard cannot see.
- **In Scope**: `ring_core`, `ring_config`, `ring_types`; the absence of `ring_wait` at any depth; the two crates that reach `ring_wait` without naming it.
- **Out of Scope**: What to do about the reach-versus-declaration mismatch (→ [`../decisions/002`](../decisions/002_reach_or_declaration.md)); the roster as public surface (→ [`../api/002`](../api/002_the_roster_as_a_public_constant.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Family Dependency Seam](001_family_dependency_seam.md) | One runtime edge, the measured closure, the absence that is the feature, and the consumer that is not there | 🔄 |
| 002 | [What Actually Reaches `ring_wait`](002_what_actually_reaches_ring_wait.md) | The two crates one hop past the roster, the dev-only third, and why a manifest scan is blind to all of them | 🔄 |

**Outward and inward.** `001` is this crate's own seam — what `ring_poll` depends
on, what it declines, and who depends on it. `002` is the family's graph around a
single name, and `ring_poll` appears in it only as the crate that publishes the
roster. Splitting them keeps a per-crate question from being answered with a
family-wide measurement and back: `001`'s numbers all come from one manifest and
one closure, `002`'s from thirty-three.

The two findings each file carries follow the same divide. `001`'s are about
this crate's own edges going stale — a consumer that never arrived, a
justification that expired. `002`'s are about the guard: what it cannot see, and
what half of it cannot fail on.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll/docs/integration
printf 'instances:                  %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:    %s\n' "$( command grep -hoE '^### PL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:    %s\n' "$( command grep -coE '^\| PL[0-9]+ ' readme.md )"
printf 'each instance has a recipe: %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'edges this crate declares:  %s\n' "$( command grep -cE '^ring_[a-z_]+ = ' ../../Cargo.toml || true )"
printf 'edges it declines, named:   %s\n' "$( awk '/^#### Pointed absences/{f=1} f && /^#### Consumers/{exit} f' 001_family_dependency_seam.md | command grep -oE '^\| .ring_[a-z_]+.' | command grep -oE 'ring_[a-z_]+' | wc -l )"
printf 'crates in the closure:      %s\n' "$( cd ../../.. && cargo tree -p ring_poll -e normal 2>/dev/null | command grep -oE 'ring_[a-z_]+ v' | sort -u | wc -l )"
printf 'consumers of this crate:    %s\n' "$( cd ../../.. && command grep -l 'ring_poll' ring_*/Cargo.toml 2>/dev/null | command grep -cv '^ring_poll/' || true )"
printf 'crates reaching ring_wait:  %s\n' "$( cd ../../.. && for c in ring_*/; do n=$( basename "$c" ); cargo tree -e normal -p "$n" 2>/dev/null | command grep -q ring_wait && echo x; done | wc -l )"
printf 'roster entries, declared:   %s\n' "$( command grep 'pub const PARKING_CRATES' ../../src/lib.rs | command grep -o '"ring_[a-z_]*"' | wc -l )"
```

Live output:

```
instances:                  2
finding headings inside:    4
rows in the table below:    4
each instance has a recipe: 2
edges this crate declares:  3
edges it declines, named:   4
crates in the closure:      17
consumers of this crate:    0
crates reaching ring_wait:  5
roster entries, declared:   3
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PL17 | a cross-crate constraint with no cross-crate edge | n/a — unenforced | `ring_poll` carries the tick-path assertion on `ring_handle`'s behalf and the two crates have no dependency edge in either direction — exactly one manifest in the family names `ring_poll` and it is `ring_poll`'s own — so `ring_handle` names it four times in prose and implements a second, independent guard over its own source text because it cannot import the first, leaving two guards with one shared claim, no shared definition of the set they defend, and a comment as the only link. |
| PL18 | a declined edge resting on a premise that expired | n/a — drift | The Pointed Absences table declines `ring_stats` because *"nothing consumes the numbers yet"*, and `ring_bench`, `ring_factory` and `ring_overflow` now take it as a runtime dependency — the conclusion may still be right, but it can no longer be reached by the reason on record, and a stale justification differs from a stale dependency in that nothing breaks, so the document keeps reading like a decision while getting less accurate. |
| PL19 | two crates reach a parking operation and the guard reads clean | **misleading doc** | `ring_consume` reaches `ring_wait` through `ring_barrier` and `ring_testkit` through `ring_shutdown`, both on normal edges present in a release build, and neither is in `PARKING_CRATES` because the guard reads each `Cargo.toml` for the substring `ring_wait` and neither manifest contains it — so a scheduler author consulting a `pub` constant documented as *"the family crates from which a parking operation is reachable"* is told `ring_consume` is safe to call from a tick, and it can sleep. |
| PL20 | half the guard cannot fail while the other half passes | n/a — coverage | After `assert_eq!( measured, PARKING_CRATES )` the test loops over three hard-coded names asserting none is in `measured`, which by then is provably equal to a compile-time array of three literals containing none of them, so no state of any manifest in the family can fire it — it guards exactly one input, an edit that adds a tick-path crate to the roster to silence the assertion above, while reading as a check on the family and naming 3 crates out of 33. |
