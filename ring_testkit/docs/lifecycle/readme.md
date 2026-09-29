# Lifecycle Doc Definition

### Scope

- **Purpose**: Trace both machines the crate runs — the shutdown state a script moves through, which ends somewhere definite, and the leaked ring, which does not end at all.
- **Responsibility**: States, transitions, terminal states and their absence, and the instance counts each machine leaves behind.
- **In Scope**: `Shutdown`'s open/closed flag as a script drives it; the allocations `leak` and `leak_ends` hand to `'static`.
- **Out of Scope**: The ring's own fullness, which is not a state of either machine — a full ring is a condition, not a mode; why the leak is necessary (→ [`workaround/`](../workaround/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The States A Script Moves Through](001_the_states_a_script_moves_through.md) | Two states, seven transitions, and the one that passes through a state it did not intend to | 🔄 |
| 002 | [The Object Whose Lifecycle Has No End](002_the_object_whose_lifecycle_has_no_end.md) | Three states, no terminal one, and a count that scales with loom's execution count | 🔄 |

**The split is by whether the machine closes.** `001`'s subject is a boolean a
script drives and reads back at teardown: two states, seven transitions, and an
`Outcome` field reporting which one it ended in. `002`'s subject enters three
states and leaves the last of them never, because leaving would need an owner
and the absence of an owner is the entire point of the function that creates it.

They are apart because a reader arrives at them from opposite directions. The
first is reached by asking *what does my script do*; the second by asking *what
did that test just leave behind*. Neither question's answer is a step in the
other's machine.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit/docs/lifecycle
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### TK[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| TK[0-9]+ ' readme.md )"
printf 'states enumerated:        %s\n' "$( command grep -hcE '^\| (\*\*(Open|Closed)\*\*|L[0-9]) \|' [0-9][0-9][0-9]_*.md | paste -sd+ | bc )"
printf 'what leaves state L3:     %s\n' "$( awk -F'|' '/^\| L3 \|/{ gsub( /^ +| +$/, "", $5 ); print $5 }' 002_*.md )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
states enumerated:        5
what leaves state L3:     nothing
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TK29 | the state table's two write columns | **misleading doc** | The States table answers *"A push is"* once per state while the crate has three write paths, and `Step::Stage`'s arm holds zero references to `shutdown` or `guard` — so a `Stage` on a Closed ring mints, stages and is counted in `staged_at_end` where the row a reader consults says it is refused. |
| TK30 | the `DrainAll` termination argument | n/a — observation | Behavioral invariant 2 credits termination to consumption being permitted in both states, but `Script::run` spawns zero threads, so nothing can be added because there is no other thread — the argument is `ring_shutdown`'s, holds identically on an open ring here, and is unfalsifiable in the only setting the scripted suite can construct. |
| TK31 | where the leak's cost is stated | **misleading doc** | `leak`'s doc sizes the workaround at *"one ring per model execution"* and no loom test calls `leak`; all three models reach `leak_ends`, whose own doc corrects the count to two while explaining a borrow error, and a `Ring` owning a `Storage< T >` makes the real figure three heap blocks per execution. |
| TK32 | the bridge's use outside its cfg | n/a — observation | `tests/testkit_test.rs` opens `#![ cfg( not( loom ) ) ]` and calls the bridge at three sites, so every default `cargo test` strands four allocations under a justification — that loom has no scoped threads — which is compiled out of that very run, and nothing in the scripted suite records it. |
