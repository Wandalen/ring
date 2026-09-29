# API Doc Definition

### Scope

- **Purpose**: The whole public surface, with each item's cost attributed to whoever chose it.
- **Responsibility**: One instance covering every exported item, four compatibility guarantees, and the settled-absent list.
- **In Scope**: `Budget`, `Progress`, `Tick`, the four free helpers, `PARKING_CRATES`.
- **Out of Scope**: `ring_core`'s own producer and consumer surfaces (→ [`ring_core/docs/api/001`](../../../ring_core/docs/api/001_producer_surface.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Tick-Path Surface](001_tick_path_surface.md) | Every item, who chose its cost, and which guarantee is convention rather than construction | 🔄 |
| 002 | [The Roster As A Public Constant](002_the_roster_as_a_public_constant.md) | One exported array, the claim its doc comment makes, and the test that cannot check it | 🔄 |

**The surface, then the one item on it that makes a claim about somebody else.**
`001` covers all eight exported items as a surface — what each costs and who
chose that cost. `002` takes the single item whose value is a statement about the
rest of the family rather than about this crate, because a public constant that
asserts something externally checkable is a different kind of API item from a
type or a function.

They go stale on different events. `001` on a change to a signature, a derive, or
the test count it used to hardcode; `002` on any family manifest gaining or losing
a `ring_wait` edge — an edit in another crate entirely, which is the property that
made it worth separating.

The four findings pair up. PL5 and PL6 are about `001`'s subject, the surface: one
derive list is backwards for the caller the crate is written for, and one count
typed into prose drifted the moment a test was added. PL7 and PL8 are about
`002`'s: the roster's summary sentence claims reachability while the array holds
direct declarations, and the test named as its enforcement checks the array
against the same narrow scan, so the two crates already in the gap are invisible
from inside it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll/docs/api
printf 'instances:                  %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:    %s\n' "$( command grep -hoE '^### PL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:    %s\n' "$( command grep -coE '^\| PL[0-9]+ ' readme.md )"
printf 'each instance has a recipe: %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'top-level pub items:        %s\n' "$( command grep -cE '^pub (const|enum|fn|struct) ' ../../src/lib.rs || true )"
printf 'Budget derives:             %s\n' "$( awk '/^pub struct Budget/{print prev} {prev=$0}' ../../src/lib.rs | command grep -oE '[A-Z][A-Za-z]+' | tr '\n' ' ' )"
printf 'Progress derives:           %s\n' "$( awk '/^pub enum Progress/{print prev} {prev=$0}' ../../src/lib.rs | command grep -oE '[A-Z][A-Za-z]+' | tr '\n' ' ' )"
printf 'of Ord/Hash, on Progress:   %s\n' "$( awk '/^pub enum Progress/{print prev} {prev=$0}' ../../src/lib.rs | command grep -cE 'Ord|Hash' || true )"
printf 'the roster, by name:        %s\n' "$( command grep -oE 'pub const [A-Z_]+' ../../src/lib.rs )"
printf 'crates the roster lists:    %s\n' "$( command grep '^pub const PARKING_CRATES' ../../src/lib.rs | command grep -oE '"ring_[a-z_]+"' | tr -d '"' | tr '\n' ' ' )"
printf 'manifests naming ring_wait: %s\n' "$( cd ../../.. && command grep -l 'ring_wait' ring_*/Cargo.toml 2>/dev/null | wc -l )"
printf 'depending on the listed two:%s\n' "$( cd ../../.. && for f in ring_*/Cargo.toml; do awk '/^\[dependencies\]/{g=1;next} /^\[/{g=0} g' "$f" | command grep -qE '^(ring_barrier|ring_shutdown)' && printf ' %s' "${f%/Cargo.toml}"; done )"
printf 'tests in the suite:         %s\n' "$( command grep -c '#\[ test \]' ../../tests/poll_test.rs || true )"
printf 'the guard reads manifests:  %s\n' "$( command grep -oE 'Cargo.toml' ../../tests/poll_test.rs | head -1 )"
```

Live output:

```
instances:                  2
finding headings inside:    4
rows in the table below:    4
each instance has a recipe: 2
top-level pub items:        8
Budget derives:             Debug Clone Copy PartialEq Eq PartialOrd Ord Hash 
Progress derives:           Debug Clone Copy PartialEq Eq 
of Ord/Hash, on Progress:   0
the roster, by name:        pub const PARKING_CRATES
crates the roster lists:    ring_barrier ring_shutdown ring_wait 
manifests naming ring_wait: 3
depending on the listed two: ring_consume ring_testkit
tests in the suite:         32
the guard reads manifests:  Cargo.toml
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PL5 | the input type is ordered and hashable, the output type is neither | n/a — observation | `Budget` derives `PartialOrd`, `Ord` and `Hash` and is set once per tick; `Progress` derives none of the three and is the value a scheduler handles per subsystem per frame, so it cannot be sorted, `max()`-ed, used as a map key or put in a set — and `Progress::then` exists precisely because folding results is the expected operation, leaving `count()` as the routine escape hatch to a `usize` that has all three, which is a type whose users convert out of it to do the ordinary thing. |
| PL6 | a count typed into prose drifted the moment a test was added | n/a — drift | The Tests table read *"22 tests covering the surface"* against a suite of 23, with the same figure repeated a third time inside a `sh` block that looks executable and asserts nothing — while the doc-example count written the same way at the same time stayed correct, so two hand-typed figures diverged with nothing to tell a reader which is which: the recipe checker executes and compares nothing, and the suite does not read its own documentation. |
| PL7 | the roster's summary sentence is broader than the array under it | **misleading doc** | The doc comment opens *"The family crates from which a parking operation is reachable"* — a transitive claim, and the one a reader wants — over an array holding the three crates that declare `ring_wait` in their own manifest, while five reach it: those three plus `ring_consume` through `ring_barrier` and `ring_testkit` through `ring_shutdown`, both one edge deep and both ordinary family members; the narrower sentence two paragraphs later describes the array accurately, so the two claims differ and the false one is the summary line read first. |
| PL8 | the guard credited with enforcing the roster cannot see the case that breaks it | **latent hazard** | The doc comment names `the_tick_path_cannot_reach_a_parking_operation` as what keeps the array honest, and the test does read every `ring_*/Cargo.toml` and assert the scan equals `PARKING_CRATES` — but a manifest scan finds only direct declarations, `ring_consume`'s manifest contains `ring_barrier` rather than `ring_wait`, so scan and array agree at three, the assertion passes, and the reachability property the sentence claims to protect is untested with two crates already sitting in the gap. |
