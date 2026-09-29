# Pattern Doc Definition

### Scope

- **Purpose**: Document the practice this whole crate is built from — make the disallowed operation unrepresentable rather than detectable — with an honest account of where it fails.
- **Responsibility**: Give the practice its problem, its solution, its applicability limits, and its real costs.
- **In Scope**: The practice as applied here and beyond; the conditions under which it pays.
- **Out of Scope**: This crate's two specific applications (→ [`invariant/`](../invariant/readme.md)); the criterion that keeps it enforced (→ [`non_functional_requirement/`](../non_functional_requirement/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Enforce by Withholding, Not by Checking](001_enforce_by_withholding.md) | Three parts of which the third is usually skipped, and seven applicability rows including four where the practice does not apply | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle/docs/pattern
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### HD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| HD[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| HD17 | the forward-narrow-or-add rule | n/a — doc gap | Every one of the twelve public methods obeys the three-way rule and no line of the source states it — the rule exists only in this instance, so an edit that breaks it reads as ordinary |
| HD18 | the narrow category | n/a — observation | The rule's middle category has exactly one member — `drain`, narrowed by a call-time bound — and the crate spends seven compile-fail cases enforcing the boundaries around it |
| HD19 | the three kinds | **measured cost** | Two of the rule's three kinds compile to nothing measurable — the eight bare forwards and the two rewrappings are the width of what they wrap — and the third, `Drain`, is the only one with a structure and a cost of its own |
| HD20 | the withheld-property list | n/a — drift | This instance names three axes as uncovered by the compile-fail suite and the suite covers two of them: the Clone axis has three cases and the Sync axis one, leaving only the accessor axis genuinely open |
