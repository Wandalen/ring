# Type Doc Definition

### Scope

- **Purpose**: Define the two exported types that carry rules — the one with an invariant, and the one with nine variants and a size ceiling.
- **Responsibility**: Give each type's definition and its validation rules.
- **In Scope**: `Capacity` and `RingError`.
- **Out of Scope**: `Seq` and `SlotIndex`, which have no validation rules and are documented as a layout instead (→ [`../data_structure/001`](../data_structure/001_two_position_types_and_the_fold_between_them.md)); `WaitKind` and `OverflowPolicy`, whose interesting content is their handlers' (→ [`../pattern/001`](../pattern/001_discriminants_here_handlers_elsewhere.md)).

### Overview Table

| ID | Name | Purpose | domain | ddd | Status |
|----|------|---------|--------|-----|--------|
| 001 | [Capacity](001_capacity.md) | Five rules over a private `usize`, whose failure mode is not a panic but a silently wrong memory offset | ring | value object | 🔄 |
| 002 | [RingError](002_ring_error.md) | Seven rules over nine variants, three of which are broken and unenforced | ring | value object | 🔄 |

**Four of the six exports are absent from this definition, deliberately.** A
`type/` instance is for a type with rules to state; `Seq`, `SlotIndex`,
`WaitKind` and `OverflowPolicy` have none — every `u64` is a valid position,
every `usize` a valid index, and both enums are closed sets whose members are
all equally legal. Documenting them here would produce four instances whose
Validation section reads "none", which is why they are documented under
`data_structure/` and `pattern/` instead, where they have something to say.

**The two that remain are the two with something a caller can get wrong.**
`Capacity`'s rules are all held (V1–V5); `RingError`'s are not — E5, E6 and E7
are broken with no mechanism watching any of them.


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types/docs/type
printf 'instances:               '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TY[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TY[0-9]+ ' readme.md
# instances:               2
# finding headings inside:  2
# rows in the table below:  2
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TY51 | `ring_types` | n/a — observation | `Seq`, `SlotIndex`, `WaitKind` and `OverflowPolicy` all derive or implement `Default`; `Capacity` and `RingError` do not, and only one of the two absences is deliberate |
| TY52 | ring family | n/a — coverage | `CapacityNotPowerOfTwo( usize )` and `BatchTooLarge { requested, capacity }` carry data that only this crate's own tests ever destructure |
