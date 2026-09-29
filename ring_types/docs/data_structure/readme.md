# Data Structure Doc Definition

### Scope

- **Purpose**: Give the layouts behind the crate's six exports — two position newtypes and the fold between them, and the error enum whose three constraints decide its size.
- **Responsibility**: State each structure's abstract, its layout, and the operations over it.
- **In Scope**: `Seq`/`SlotIndex` and the mask fold; `RingError`'s 24 bytes and what sets them.
- **Out of Scope**: The validation that makes the fold safe (→ [`../algorithm/001`](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md)); the enums, which carry no payload worth a layout document.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Two Position Types and the Fold Between Them](001_two_position_types_and_the_fold_between_them.md) | A `u64` position and a `usize` index, both with public fields, and why one reaches 18 crates while the other reaches 3 | 🔄 |
| 002 | [The Error Enum as a Closed Copy Set](002_the_error_enum_as_a_closed_copy_set.md) | Twenty-four bytes set by a single variant, and the three constraints — `Copy`, `#[ non_exhaustive ]`, zero dependencies — that each forbid something different | 🔄 |

**Both instances are about what a layout forecloses rather than what it
enables.** 001's `u64`/`usize` split is what makes the two types
non-interchangeable, and the reason neither hides its field. 002's `Copy`
constraint is what forbids any variant from carrying a `String` — the single
decision that cost the family a shared error type
(→ [`../integration/002`](../integration/002_the_registry_that_declined_the_shared_error.md)).

**Neither structure has a `Drop`, an indirection, or an allocation.** That is
one sentence for the whole definition and it is measured rather than asserted
(→ [`../non_functional_requirement/001`](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md)).


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types/docs/data_structure
printf 'instances:               '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TY[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TY[0-9]+ ' readme.md
# instances:               2
# finding headings inside:  3
# rows in the table below:  3
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TY26 | `ring_mpsc` | n/a — duplication | `ring_index:42` and `ring_mpsc:543` both fold a `Seq` with `( seq.0 as usize ) & ... .mask()`, and `ring_index` is the crate that exists to own that fold |
| TY27 | ring family | n/a — observation | Neither call site can be written without `Seq`'s `pub` field, so the fold and the encapsulation asymmetry are the same fact seen twice |
| TY28 | `ring_types` | n/a — observation | The `Display` impl matches without a wildcard, so adding a variant is a compile error there — the one drift in this crate the compiler catches by itself |
