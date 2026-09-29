# Non-Functional Requirement Doc Definition

### Scope

- **Purpose**: Carry this crate's own binary Reached condition and the field-at-a-time assertion shape it mandates, plus the requirement the acceptance table leaves unstated.
- **Responsibility**: State the quality attribute, the measurable statement, the measurement method, and the threshold.
- **In Scope**: The five-field criterion; construction-time cost.
- **Out of Scope**: The ring's runtime cost, which the backend crates' criteria grade.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Five Fields, Asserted One at a Time](001_five_fields_asserted_one_at_a_time.md) | This crate's own Reached condition, and what "one field at a time" forecloses | 🔄 |
| 002 | [Construction Cost Is Paid Once and Never on the Path](002_construction_cost_is_paid_once.md) | The unstated companion — a factory may allocate freely and must leave nothing behind that a tick pays for | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_factory/docs/non_functional_requirement
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FC[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FC[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FC33 | `ring_factory` | n/a — unenforced | The requirement is capped at two fields by the return type, not by the tests — a built `Ring` retains storage and `overflow`, so "every field asserted one at a time" can only reach two of five |
| FC34 | `ring_factory` | n/a — coverage | Two doctests are the only tests of the documented call shape, they sit above `build` and `build_named`, and `build_crossbeam` — the method whose call shape is least guessable — has none |
| FC35 | `ring_factory` | n/a — unenforced | The requirement's own measurement appears nowhere in the suite; the single `alloc` match in the test file is a doc comment asserting that an allocation hook exists |
| FC36 | `ring_factory` | n/a — observation | Construction allocates once and the crate cannot see it — the allocation is `ring_store`'s, reached through `ring_core`, and this crate names neither |
