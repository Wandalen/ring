# Type Doc Definition

### Scope

- **Purpose**: Document the two values this crate's correctness arguments are written in — the single-writer position, and the derived free-space quantity whose contract is stronger here than anywhere else in the family.
- **Responsibility**: Define each value, the roles it plays simultaneously, and the validation rules governing each role.
- **In Scope**: The producer cursor and its consumer mirror; the derived free capacity.
- **Out of Scope**: The padded representation, which is `ring_cursor`'s and `ring_align`'s; the shared `Seq` newtype, which is `ring_types`'.

### Overview Table

| ID | Name | Purpose | domain | ddd | Status |
|----|------|---------|--------|-----|--------|
| 001 | [Producer Cursor](001_producer_cursor.md) | One `u64` serving as publication count, slot address, lap counter, and — uniquely to SPSC — publication boundary | ring | value object | 🔄 |
| 002 | [Free Capacity](002_free_capacity.md) | The derived quantity whose one-sided staleness turns a hint into a guarantee | ring | value object | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc/docs/type
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### SP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| SP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  2
# rows in the table below:  2
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SP48 | `SeqCell` | n/a — observation | The cursor type is imported from `ring_cursor` and its atomicity is that crate's guarantee, swapped under `--cfg loom`. |
| SP49 | `free_capacity` | **misleading doc** | One producer means the returned number cannot go stale, but `ring_core` shares one signature across both backends and its callers take the weaker contract. |
