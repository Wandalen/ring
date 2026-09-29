# Pitfall Doc Definition

### Scope

- **Purpose**: Document the trap a crate made of absences invites — that every reasonable addition removes a guarantee, and the test suite goes green either way.
- **Responsibility**: Name the trap, the specific edits, what each breaks, and what actually prevents them.
- **In Scope**: Additions to this crate's public surface, and the reasoning that produces them.
- **Out of Scope**: The invariants themselves (→ [`invariant/`](../invariant/readme.md)); the general practice (→ [`pattern/`](../pattern/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [A Convenience Method Undoes the Crate](001_a_convenience_method_undoes_the_crate.md) | Eight edits with their stated reasons — one is caught by the acceptance criterion, and it is the one nobody makes | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle/docs/pitfall
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### HD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| HD[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| HD37 | the pinned `.stderr` | n/a — doc gap | The one fixture coupled to another crate's private type documents that it is coupled and not what the coupling costs — no row states what breaks, where, or who finds out first |
| HD38 | the two detectors | n/a — duplication | A positive test and a compile-fail case assert the same property, and only the compile-fail case breaks when an unrelated crate renames a private type |
| HD39 | the Caught by column | **wrong doc** | Three of the eight Caught-by cells name a mechanism in another crate and none of the three exists: `ring_poll` has no compile-fail suite, its bounded-time test never names this crate, and `ring_spsc` has no counting shim |
| HD40 | the mitigation table | n/a — drift | The one mitigation priced as future work is already built and wider than proposed — seven forbidden names, as a test rather than a CI grep — and the name-list mechanism cannot in principle catch F6, which parks without naming anything |
