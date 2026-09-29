# Invariant Doc Definition

### Scope

- **Purpose**: State the three properties the rest of the family is entitled to assume without checking — one enforced by the type system, one by a coverage gate, one by nothing at all.
- **Responsibility**: For each, state the invariant, its enforcement mechanism, and the consequences of violation.
- **In Scope**: Every `Capacity` yielding a valid mask; tier 0 declaring no dependencies; every `RingError` rendering to a distinct, payload-bearing message.
- **Out of Scope**: The validation procedure itself (→ [`../algorithm/001`](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md)); the enum-closure requirement, which is graded rather than absolute (→ [`../non_functional_requirement/002`](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Every Capacity Has a Valid Mask](001_every_capacity_has_a_valid_mask.md) | The property 15 crates fold on without re-testing — held by a private field, a validating constructor, and an absent `Default` | 🔄 |
| 002 | [Tier Zero Depends on Nothing](002_tier_zero_depends_on_nothing.md) | The property that makes the family's dependency forest acyclic by construction — with **no enforcement mechanism**, only a comment | 🔄 |
| 003 | [Every Error Renders Distinctly](003_every_error_renders_distinctly.md) | Nine `Display` arms, a hand-maintained roster the test crate cannot check, and the coverage gate that caught the omission that already happened | 🔄 |

**The three sit at three different points on one axis — how much of the
invariant the compiler holds — and that is the point of grouping them.**

001 cannot be violated: the mechanism is the type system, and a test that tried
would not compile. 003 is held in part — the compiler forces every variant to
*have* a `Display` arm, and then stops, leaving distinctness and payload
interpolation to a hand-written roster and gate G1's coverage threshold. 002 has
nothing but a comment, and can be violated by one line in a manifest.

**Both weak links have a named, concrete failure.** 002's is replacing the
hand-written `Display` with `error_tools`, the house convention everywhere else
in the workspace. 003's has already occurred: `PolicyUnsupported` was added, the
roster was not updated, and G1 reported `ring_types/src/error.rs 16/17` — caught
by a gate the crate does not own, one step before it would have mattered.

**002's missing gate is the crate's one concrete unfiled task.** Asserting
`ring_types`'s `[dependencies]` is empty is a single grep in a gate script
beside the twenty-two that already run, and it would fail on the day the convention
arrived rather than after the family had absorbed it.


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types/docs/invariant
printf 'instances:               '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TY[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TY[0-9]+ ' readme.md
# instances:               3
# finding headings inside:  5
# rows in the table below:  5
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TY34 | ring family | n/a — observation | `is_power_of_two` and `count_ones() == 1` have zero occurrences across all 32 other crates, which is the invariant working as an absence |
| TY35 | ring family | n/a — coverage | One `Capacity::new` call establishes it now (`ring_config/src/lib.rs:71`) — TY20 removed the other; 29 `.get()` reads and 2 `.mask()` reads consume it without re-checking |
| TY36 | `ring_types` | n/a — unenforced | The acyclicity property the family rests on is stated in prose and checked by no gate; the twenty-two scripts under `bench_harness/gate/` check the corpus, not the manifest |
| TY37 | `ring_types` | n/a — unenforced | `#![ no_std ]` is now declared (`src/lib.rs:27`), so the property holds because a `std::` path fails to compile, not because none happens to be written |
| TY38 | `ring_types` | n/a — unenforced | `every_error_displays_distinctly` checks non-emptiness, pairwise distinctness and two embedded values; permuting all nine messages leaves it green |
