# Type Doc Definition

### Scope

- **Purpose**: Describe the crate's own vocabulary — one public enum whose arity is a build property, and one concept deliberately represented by a return value rather than a type.
- **Responsibility**: Variants, derives, what each type can and cannot express, and the argument for the one that does not exist.
- **In Scope**: `Backend`; producer cardinality as reported by `try_clone`.
- **Out of Scope**: Types re-exported from the family (`RingConfig`, `OverflowPolicy`, `RingError`), which belong to their own crates.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [`Backend`](001_backend.md) | A public name for a private choice, with three variants in one build and two in the other — and one derive nothing yet uses | 🔄 |
| 002 | [Producer Cardinality as a Return Value](002_producer_cardinality.md) | Why there is no `Cardinality` enum and why `Producer` is not `Clone`: the check and the operation are the same call, so they cannot drift apart | 🔄 |



### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core/docs/type
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### CO[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| CO[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  2
# rows in the table below:  2
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CO50 | `Backend` | n/a — observation | The type carries no data, exists to be compared, and is compared only in one test file. |
| CO51 | `try_clone` | n/a — observation | `Option< Producer >` encodes "this backend permits one producer" as a runtime absence rather than as a compile-time distinction. |
