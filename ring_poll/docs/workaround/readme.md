# Workaround Doc Definition

### Scope

- **Purpose**: Constraints `ring_poll` absorbs into the shape of its code, each with the cost it imposes and the condition under which it can be deleted.
- **Responsibility**: Two — a pause hint written locally rather than imported, and a loop shape chosen for a coverage tool.
- **In Scope**: The constraint, the local replacement, what it actually costs, and where the reason is recorded.
- **Out of Scope**: The `PARKING_CRATES` roster being hand-typed, which is a filed decision with a threshold rather than an absorbed constraint (→ [`../decisions/001`](../decisions/001_should_the_roster_be_generated.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Reimplementing the Pause Hint](001_reimplementing_the_pause_hint.md) | A hint emitted locally rather than imported, and the backoff policy that differs as a result | 🔄 |
| 002 | [A Counter-Bounded `while`](002_a_counter_bounded_while.md) | A loop shape chosen for `llvm-cov`, and the note that did not travel with it | 🔄 |

**This directory said "none" for as long as it existed, and the argument that
produced that answer is worth keeping because it was nearly right.** The
dependency closure is entirely in-house, the implementation is four bounded
loops over `ring_core`, and there is no third-party API being papered over. On
the ordinary reading — a shim around someone else's broken library — the crate
genuinely has none.

The definition this directory actually uses is wider than that reading, and both
instances meet it. `001` is a constraint the crate chose (refusing a dependency
to keep a dangerous call out of autocomplete range) and `002` is one imposed on
it (a coverage tool that cannot attribute a hit to a bare `loop` line). Both
change the shape of the code, both have a cost, and both have a deletion
condition — which is the whole test.

They are separate because the two halves that matter differ completely. `001`'s
replacement is *visible*: the missing dependency is in the manifest, a test
asserts the roster, and a manual probe proves the import fails to resolve.
`002`'s is *invisible*: a counter-bounded `while` is what most people would write
anyway, so nothing distinguishes the constrained shape from the natural one.

The four findings split along exactly that line. `001`'s are about a cost that
was mispriced — the readme called it three duplicated lines, when the helper one
crate over runs an escalating backoff this crate does not have and has no
compiled call site anywhere in the family (PL49) — and about the reason living
at one of three copied sites (PL50). `002`'s are about the reason not living
anywhere in `src` at all, where the crate that measured it wrote six lines at the
loop (PL51), and about a deletion condition that names no version, links no
issue, and has nothing that would ever notice it had been met (PL52).

Read together they say something about the definition rather than about either
instance: a workaround whose shape is indistinguishable from ordinary code is
the one that most needs a note at the site, and is the one most likely not to
have one.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll/docs/workaround
printf 'instances:                    %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:      %s\n' "$( command grep -hoE '^### PL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:      %s\n' "$( command grep -coE '^\| PL[0-9]+ ' readme.md )"
printf 'each instance has a recipe:   %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each states a deletion cond:  %s\n' "$( command grep -lc '^### Deletion condition' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'external crates at any depth: %s\n' "$( cd ../../.. && cargo tree -p ring_poll -e normal 2>/dev/null | command grep -cvE 'ring_|^$' || true )"
printf '001 — ring_wait as a dep:     %s\n' "$( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ../../Cargo.toml | command grep -c 'ring_wait' || true )"
printf '002 — llvm-cov named in src: %s\n' "$( command grep -c 'llvm-cov' ../../src/lib.rs || true )"
printf 'loops the second one shapes:  %s\n' "$( command grep -cE '^\s*while ' ../../src/lib.rs || true )"
printf 'hint sites the first one has: %s\n' "$( command grep -c 'core::hint::spin_loop' ../../src/lib.rs || true )"
```

Live output:

```
instances:                    2
finding headings inside:      4
rows in the table below:      4
each instance has a recipe:   2
each states a deletion cond:  2
external crates at any depth: 0
001 — ring_wait as a dep:     0
002 — llvm-cov named in src: 2
loops the second one shapes:  4
hint sites the first one has: 3
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PL49 | the workaround was priced as duplication, and it is not duplication | **misleading doc** | This readme dismissed the local pause hint as not-a-workaround because the cost was *"reimplementing a `spin_loop()` hint that already exists one crate over — three lines"*, and neither half holds: `ring_wait::pause` under `WaitKind::Spin` runs `for _ in 0..=( attempt % 8 ) { spin_loop(); }`, an escalating backoff of one to eight hints, where this crate emits exactly one per attempt always — so adopting it would change the retry policy rather than deduplicate it — and `ring_wait::pause` has zero compiled call sites in the family, its only two textual occurrences being its own doctest and this crate's manual probe P1, a file written to fail to compile; what remains is that `ring_poll` has a flat backoff nobody recorded as a choice, arrived at as a side effect of a dependency refusal argued on entirely different grounds. |
| PL50 | the reason is written at one of the three copies | n/a — duplication | Three sites emit `core::hint::spin_loop()`, and one — in `push_within` — carries the comment that wards off the edit that would break the crate's central guarantee (*"Yielding here would be the parking this crate exists to keep off the tick path"*); the copies in `push_batch_within` and `recv_within` are bare, so a maintainer profiling either sees three lines with no argument attached, and the natural reach for a hot spin is exactly the `yield_now()` the comment exists to prevent. |
| PL51 | the crate that inherited the rule did not inherit the note that protects it | **latent hazard** | `ring_shutdown` measured the `llvm-cov` region-attribution problem — 72/73 lines with `loop`, 73/73 with a `while`, identical suite — and wrote six lines at the governed loop naming the tool, the mechanism, both numbers and the probe; `ring_poll` took the shape and left the note behind, so `llvm-cov` appears zero times in this crate's `src` and only in documents written afterwards about something else, and a maintainer rewriting `while attempt < budget.attempts()` as a `loop` with an inner `break` — a change that reads as pure simplification — meets nothing in the file, with the cost surfacing as a coverage regression days later attributed to whatever else moved. |
| PL52 | the deletion condition names no version and nothing would notice it was met | n/a — unenforced | This directory's own definition requires *"the condition under which it can be deleted"*, and for the loop shape that condition — `llvm-cov` attributing hits to a bare `loop` line — pins no tool version in either crate, links no issue, and is re-checked by nothing; the measurement behind it was taken once, in another crate, against an unrecorded toolchain, and the family is no safety net because `ring_bench` and `ring_publish` still use a bare `loop`, so a family-wide check would fail today for unrelated reasons and cannot be added without first ruling on those two. |
