# Type Doc Definition

### Scope

- **Purpose**: Define the small value types the ring's correctness arguments are written in — sequence, slot index, lap, capacity — so that "the sequence" means one thing across five other definitions rather than being re-derived in each.
- **Responsibility**: Give each value type a definition, its validation rules, and the arithmetic relating it to its siblings; record which representation choices are still open.
- **In Scope**: Sequence number, slot index, lap, and capacity as domain types, plus the decomposition that ties them together.
- **Out of Scope**: The fields that store them (→ [`data_structure/`](../data_structure/readme.md)); the states computed from them (→ [`lifecycle/`](../lifecycle/readme.md)); the payload type `T`, which this crate is deliberately agnostic about.

### Overview Table

| ID | Name | Purpose | domain | ddd | Status |
|----|------|---------|--------|-----|--------|
| 001 | [Sequence Number](001_sequence_number.md) | The monotonic claim ordinal that is simultaneously the slot address, the lap counter, and the publication token | ring | value object | 🔄 |
| 002 | [Capacity](002_capacity.md) | The fixed slot count, and the power-of-two question that decides whether addressing is a mask or a division | ring | value object | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc/docs/type
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### MP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| MP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  2
# rows in the table below:  2
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| MP48 | `Seq` | n/a — observation | This crate stamps, compares and folds sequences without owning the type or any of its guarantees. |
| MP49 | `Capacity` | n/a — observation | Slot lookup is `( seq.0 as usize ) & capacity.mask()` with no bounds check, which is sound only because `Capacity::new` rejected zero and non-powers-of-two. |
