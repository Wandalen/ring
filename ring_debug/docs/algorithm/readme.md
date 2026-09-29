# Algorithm Doc Definition

### Scope

- **Purpose**: Specify how a cursor pair is evaluated against a ring that may be running — the comparison itself, and the three separate ordering decisions it rests on.
- **Responsibility**: State each step, what it costs, what breaks if it is reordered, and what would notice.
- **In Scope**: `check`, `check_seqs`, the `Acquire` choice, the D1-before-D2 order, the producer-before-consumer load order.
- **Out of Scope**: What the comparisons mean (→ [`invariant/`](../invariant/readme.md)); the stateful sequence they run inside (→ [`lifecycle/`](../lifecycle/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Checking a Pair Without Touching It](001_checking_a_pair_without_touching_it.md) | Four reads, two comparisons, one subtraction that must not saturate | 🔄 |
| 002 | [The Third Ordering](002_the_third_ordering.md) | Which cursor is read first, and which false positive that selects | 🔄 |

**The two split along what is written down.** `001` documents the memory ordering
and the check order, both of which are argued at length in the source and in
[`api/001`](../api/001_the_check_surface.md)'s guarantee table. `002` documents the
third ordering, which is made identically at all three read sites and stated
nowhere.

They are the same eight lines of code read for two different decisions. A reader
who wants to know *what the check concludes* needs `001`; a reader who wants to
know *what the check could conclude wrongly* needs `002`.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug/docs/algorithm
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### DB[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| DB[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| DB21 | failure mode N2 | **misleading doc** | The failure table quantifies a regression as costing exactly one test when four independent tests across three entry points would fail, understating the suite's coverage in the one place a reader would rely on the figure. |
| DB22 | the `Acquire` cost row | **misleading doc** | The crate's only claim about machine behaviour prices an acquire load as free on x86, which is not the architecture this workspace builds on and not a target the manifest configures. |
| DB23 | the load order | **latent hazard** | Reading the producer before the consumer selects D1 — the violation with no corroborating symptom — as the false positive read skew can manufacture; the decision was spelled at all three read sites and recorded at none, and is now made once in `observe_pair` and stated in its rustdoc. |
| DB24 | the test suite | n/a — coverage | Every design decision in the crate is a concurrency decision and the suite contains no threads, so the acquire ordering, the quiescence precondition, and the skew window are all argued rather than exercised. |
