# Invariant Doc Definition

### Scope

- **Purpose**: State the properties a ring must satisfy that the family's own arithmetic assumes and never verifies, and the mechanisms this crate offers for each.
- **Responsibility**: Enumerate each invariant, what it excludes, what enforces it, what detects a breach, and what a breach costs.
- **In Scope**: D1–D3 over raw cursors; D4 over derived readings; enforcement mechanisms E1–E5; violation consequences V1–V8.
- **Out of Scope**: How the checks are evaluated (→ [`algorithm/`](../algorithm/readme.md)); why a breach is invisible to the ring itself (→ [`pitfall/`](../pitfall/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Cursor Invariants Over a Live Ring](001_cursor_invariants_over_a_live_ring.md) | D1–D3 — ordering, distance, monotonicity, over the raw cursors | 🔄 |
| 002 | [The Conservation Law and Why It Holds](002_the_conservation_law_and_why_it_holds.md) | D4 — two derived readings accounting for one ring, and why it cannot fail | 🔄 |

**The split is by what the invariant is a statement about, and it is not a
presentational choice.** D1–D3 constrain cursors and are checked by comparing them.
D4 constrains two numbers *computed from* cursors, includes a term the ring does
not own, and is checked by an expression built out of the same constant it is
compared against.

That difference is the crate's central limitation stated precisely rather than
approximately. The doors that reach the cursors can see corruption; the door the
family's Contract actually offers checks a proposition the arithmetic beneath it
cannot falsify. Both facts belong in this definition, and neither is legible while
they share a document.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug/docs/invariant
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### DB[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| DB[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| DB33 | the enforcement table | n/a — doc gap | The inventory of what enforces the cursor invariants names both entry points a Contract caller cannot reach and omits the only one they can. |
| DB34 | mechanism E1 | **misleading doc** | Monotonicity protection is credited to a type that offers none — `Seq` has no decrementing method and a public field, and this crate's own suite writes a lower one into a live cursor nine times. |
| DB35 | `check_ends` and D4 | **latent hazard** | The conservation law is an algebraic identity in the arithmetic that evaluates it, so no cursor corruption of any kind can fail it; the single corrupt input that does not simply pass underflows into a panic in a debug build and wraps back to a pass in a release one. |
| DB36 | two `free_slots` implementations | n/a — inconsistency | The family computes free capacity twice, saturating in one crate and by plain subtraction in another, and they diverge exactly on the lapped ring — while the test documenting this crate's central limitation models the function under test with the arithmetic it does not use. |
