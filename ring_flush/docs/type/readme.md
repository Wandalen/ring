# Type Doc Definition

### Scope

- **Purpose**: Define the two types this crate contributes to the family's public vocabulary.
- **Responsibility**: Give each type's definition, its validation rules, and its exported status.
- **In Scope**: `FlushPolicy` and its variants; the outcome a drive call returns.
- **Out of Scope**: `ring_types`' discriminants, which this crate does not own.

### Overview Table

| ID | Name | Purpose | domain | ddd | Status |
|----|------|---------|--------|-----|--------|
| 001 | [Flush Policy](001_flush_policy.md) | The three-variant decision, exported and therefore expensive to change | ring | value object | 🔄 |
| 002 | [Flush Outcome](002_flush_outcome.md) | What a drive call reports, and why "nothing happened" must be distinguishable from "nothing was there" | ring | value object | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_flush/docs/type
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FL[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FL45 | the trait table | n/a — observation | Six types carry the same five-derive line and the table reads it as five rulings about this one; the only departure derives the trait the table calls indefensible |
| FL46 | the serde row | n/a — inconsistency | The trait table marks `serde` open and points at a decisions record whose matching section is titled "one question deliberately not recorded here" |
| FL47 | M5 | n/a — drift | M5 recommends deriving the log entry from the outcome as future work and `record` already does it, in the words the source's own comment uses |
| FL48 | M1 | **misleading doc** | M1's enforcement column reads "Construction" and `count` comes from `try_push_batch`, which returns zero when the first push is refused — the path the source comment names |
