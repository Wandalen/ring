# Pattern Doc Definition

### Scope

- **Purpose**: Name the two practices this crate is built from — configuration as data, and one construction path — and state what each costs.
- **Responsibility**: For each, give the problem, the solution, its applicability, and its consequences.
- **In Scope**: The record-not-arguments practice; the single-entry-point practice.
- **Out of Scope**: The builder pattern that produces the record, which is `ring_config`'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Configuration as Data](001_configuration_as_data.md) | Why the shape of a ring is a value rather than a choice of constructor, and what that forbids | 🔄 |
| 002 | [One Way In](002_one_way_in.md) | A single construction path is the only thing that makes the set of legal rings enumerable | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_factory/docs/pattern
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FC[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FC[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FC37 | `ring_factory` | **latent hazard** | The pattern makes an illegal configuration representable and one actually is: `OverflowPolicy::DropOldest` passes every `ring_config` validation and is refused by `ring_core`, which is why `build` returns a `Result` at all |
| FC38 | `ring_factory` | n/a — observation | A built ring keeps one of the record's five fields as a stored value, so nothing downstream can report what a ring was built from |
| FC39 | `ring_factory` | n/a — drift | The count was five and is now nine — nine public constructors across `ring_spsc`, `ring_mpsc`, `ring_core` and this crate, against the five the pattern's status still records |
| FC40 | `ring_factory` | n/a — observation | Seven crates import `ring_core` directly and one imports this crate, so "one way in" is a promise to consumers outside the family rather than a practice inside it |
