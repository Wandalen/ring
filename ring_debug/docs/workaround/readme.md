# workaround

External constraints `ring_debug` absorbs on behalf of its consumers, each with the
cost it imposes and the condition under which it can be deleted.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Door Ring Core Does Not Open](001_the_door_ring_core_does_not_open.md) | `check_ends` exists because the Contract's own ring end exposes no cursor | 🔄 |
| 002 | [One Cast Between Two Newtypes That Disagree](002_one_cast_between_two_newtypes_that_disagree.md) | One comparison crossing four disagreements between `Seq` and `Capacity` | 🔄 |

**Both entries are constraints imposed by other crates in this family**, not by
anything outside it — which is the shape to expect from a diagnostic crate. It
observes types it does not own, through doors it did not design, and every gap
between what it needs and what it is offered has to be absorbed somewhere.

Both also have a named deletion condition that belongs to someone else:
`ring_core` forwarding `position()`, and `ring_types` offering a `u64` accessor on
`Capacity`. Neither is this crate's to make, and recording them here is what makes
them askable.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug/docs/workaround
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### DB[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| DB[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| DB17 | `check_ends_cannot_see_the_corruption_check_can` | n/a — observation | The crate's central limitation is pinned by a passing test rather than a prose caveat, so it fails at the moment the limitation is fixed. |
| DB18 | `check_ends` | n/a — observation | The door built for a Contract-following consumer has no consumer, so the cost it was written to avoid is a prediction rather than a measurement. |
| DB19 | `Seq` against `Capacity` | n/a — inconsistency | Two newtypes from one crate disagree about width, field visibility and validation, and one comparison in a third crate crosses all of it. |
| DB20 | `check_seqs` | n/a — observation | The crate that checks the family's arithmetic rested on an unguarded subtraction, an unchecked cast and a discarded validation in a single expression; DB9's `checked_sub` closed the first, and two remain. |
