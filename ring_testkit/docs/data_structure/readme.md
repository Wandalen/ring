# Data Structure Doc Definition

### Scope

- **Purpose**: Define the two shapes this crate is built around — the list a run is written as, and the block of numbers it comes back as.
- **Responsibility**: Layouts, what each shape makes free, what it forbids, and the assumptions each one carries in its widths and its arithmetic.
- **In Scope**: `Script`'s `Vec< Step >` and `stage_limit`; `Outcome`'s ten fields and the derived `vanished`.
- **Out of Scope**: The meaning of individual variants and fields (→ [`type/`](../type/readme.md)); the procedures that walk these shapes (→ [`algorithm/`](../algorithm/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Script As A Flat Step List](001_the_script_as_a_flat_step_list.md) | Why the input has no control flow, and the one field that is not a step | 🔄 |
| 002 | [Nine Counters And A Number That Is Two Things](002_nine_counters_and_a_number_that_is_two_things.md) | Why the tenth value is a method, and the two places the layout leaks an assumption | 🔄 |

**The split is input against output, and it is not symmetric.** `001`'s subject
is what the shape *forbids* — no branches, no data, no step reading another's
result — because every one of those absences is what makes two runs comparable.
`002`'s subject is what the shape *asserts*: ten fields whose consistency
nothing enforces, one of which is a `u32` among `usize`s because it is secretly
two numbers.

They are apart because a reader arrives at them for opposite reasons. Someone
writing a script needs `001` and can ignore `002` entirely; someone reading a
result needs `002` and never has to know the list is flat.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit/docs/data_structure
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### TK[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| TK[0-9]+ ' readme.md )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TK9 | the crate doc's first constant | **misleading doc** | The front-page example passes `Script::new( 4 )` two lines above `RingConfig::new( 8 )`, and the script contains no `Stage`, `StageMany` or `Flush` step — so the first number a reader of this crate sees sizes a staging buffer nothing writes to, next to the one that actually bounds the run. |
| TK10 | the `Many` step counts | **latent hazard** | `PushMany`, `RecvMany` and `StageMany` carry an unchecked `usize` and `run` applies no `min`, assertion or comparison to it, while the algorithm document's termination argument describes each step as "bounded by a constant in the step itself" — a constant the caller supplies with no ceiling. |
| TK11 | `vanished`'s saturating subtraction | **latent hazard** | The one reading documented as saying *records were destroyed* answers `0` — the healthy value — whenever delivered-plus-held exceeds accepted, which the crate reaches two established ways: a second run on one ring, and the hand-built `Outcome` three tests already construct. |
| TK12 | `minted`'s two meanings | n/a — observation | The field is documented only as a count and is also the exclusive upper bound `audit_received` tests every record value against; the two readings coincide solely because minting is consecutive from zero, which is stated on the enum rather than the field and checked nowhere. |
