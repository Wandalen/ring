# Pattern Doc Definition

### Scope

- **Purpose**: Record the two ordering rules this crate's correctness rests on — a guard that must precede a subtraction, and writes that must follow every check.
- **Responsibility**: State each pattern, what selects it, what breaks if it is reordered, and what would catch the reordering.
- **In Scope**: `check_seqs`'s two blocks; `Watch::new` and `Watch::observe`'s deferred writes.
- **Out of Scope**: The invariants being checked (→ [`invariant/`](../invariant/readme.md)); the fields being written (→ [`data_structure/`](../data_structure/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Guard That Makes the Next Line Legal](001_the_guard_that_makes_the_next_line_legal.md) | An unsigned subtraction kept sound by the block above it, and nothing else | 🔄 |
| 002 | [Commit Nothing Until Every Check Has Passed](002_commit_nothing_until_every_check_has_passed.md) | Writes deferred past every fallible step, and the fault-laundering it prevents | 🔄 |

**Both patterns are about statement order, and both are unenforced.** Neither is
expressible as a type, neither has a comment at the site, and between them one
test out of twenty-two would notice if either were reversed. They are recorded
here because a rule that lives only in the sequence of two statements is the kind
that survives review and dies in a refactor.

The two also fail differently, which is why they are separate instances. Reversing
`001` produces a *wrong answer stated confidently*; reversing `002` produces a
*right answer stated once and then withdrawn*. A diagnostic crate can survive
neither, and no single mechanism guards both.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug/docs/pattern
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### DB[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| DB[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| DB9 | `check_seqs` | **latent hazard** | `checked_sub` makes the D1 case the subtraction's own `None` branch, so the ordering that used to keep it sound is no longer expressible; eight unguarded subtractions remain elsewhere in the family. |
| DB10 | workspace profiles | n/a — unenforced | The family leaves `overflow-checks` unset in every profile, so whether a wrapped subtraction panics or lies silently is decided by build profile rather than by decision. |
| DB11 | `Watch::observe` | n/a — coverage | One test of twenty-two separates a correct watch from one that reports a permanent fault once and then calls it normal, because the property is about the receiver rather than the return value. |
| DB12 | `Watch::new` | **latent hazard** | The construction-time guarantee `new` provides is voided by observing a different pair, so the crate's strongest guarantee is undone by a call that type-checks. |
