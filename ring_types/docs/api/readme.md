# API Doc Definition

### Scope

- **Purpose**: Give the surface 30 of the family's 33 crates import, split into the six names it exists for and the five predicates nothing production calls.
- **Responsibility**: State the abstract, operations, error handling, and compatibility guarantees of each surface.
- **In Scope**: Six exports, fourteen operations, three associated constants, one error channel.
- **Out of Scope**: Per-type definitions (→ [`../type/`](../type/)); what other crates do with the discriminants (→ [`../pattern/001`](../pattern/001_discriminants_here_handlers_elsewhere.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Vocabulary Surface](001_the_vocabulary_surface.md) | Six names, four private modules, one fallible call — and the guarantee whose mechanism turns out to live in `tests/` rather than `src/` | 🔄 |
| 002 | [The Five Classifier Predicates](002_the_five_classifier_predicates.md) | One production call site across all 33 crates, inside a method whose own callers are tests — measured precisely, because "unused" and "written for an absent consumer" look identical | 🔄 |

**The split is by consumer, not by type.** 001 documents what a crate imports
`ring_types` *for*; 002 documents what it could import and does not. Keeping
them apart is what made the second measurable — the predicates' call-site count
is invisible when they are listed among fourteen operations, and is the whole
finding when they are listed alone.

**Both instances end at the same unresolved question**, from opposite sides: the
export Contract names `ring_types` as one of five crates an external consumer
may import, and no such consumer exists yet. 001 records what that consumer
would be promised; 002 records what has been built for them and left idle.


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types/docs/api
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
| TY23 | ring family | n/a — observation | `RingError` is named in 18 dependent crates, `Seq` 16, `Capacity` 11, `WaitKind` 4, `OverflowPolicy` 4 and `SlotIndex` 3 — a six-fold spread across one export surface |
| TY24 | `ring_types` | n/a — observation | The crate spent both attributes on every classifier, and four of the five have no production caller anywhere in the family to benefit from either |
| TY25 | `ring_config` | n/a — coverage | `ring_config` is the single production caller of any classifier this crate exports |
