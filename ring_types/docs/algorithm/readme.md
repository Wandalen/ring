# Algorithm Doc Definition

### Scope

- **Purpose**: Give the two procedures this crate owns — both of them a handful of comparisons over a `Copy` value, and both `const`.
- **Responsibility**: State each procedure's steps, its inputs, and what it deliberately does not do.
- **In Scope**: The two-test capacity validation; the classification of an error into configuration or traffic.
- **Out of Scope**: Anything that dispatches on a policy — the discriminant/handler split puts that in `ring_wait` and `ring_overflow` (→ [`../pattern/001`](../pattern/001_discriminants_here_handlers_elsewhere.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Validating a Slot Count to a Power of Two](001_validating_a_slot_count_to_a_power_of_two.md) | The crate's only fallible operation — two tests whose order decides the message, not the outcome, and four checks it deliberately does not run | 🔄 |
| 002 | [Classifying an Error Into Configuration or Traffic](002_classifying_an_error_into_configuration_or_traffic.md) | Two `matches!` over nine variants, and the three-variant gap where both answer `false` and nothing signals it | 🔄 |

**Both procedures are total and neither allocates.** 001 is total over `usize`
and returns a `Result`; 002 is total over the enum and returns a `bool`. Between
them they are every branch the crate contains apart from `Display`'s nine arms.

**The interesting content in both is what they omit.** 001 does not round up,
cap, or warn; 002 does not partition — its two sets leave `Closed`, `NameTaken`
and `NameUnknown` outside both, which is a valid answer that no test can
distinguish from an oversight.


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types/docs/algorithm
printf 'instances:               '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TY[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TY[0-9]+ ' readme.md
# instances:               2
# finding headings inside:  4
# rows in the table below:  4
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TY19 | ring family | n/a — coverage | The one production call to `Capacity::new` is `ring_config/src/lib.rs:71` — `ring_core/src/lib.rs:227` was the other until TY20's fix removed it; the other 30 crates receive an already-validated value and never re-check |
| TY20 | `ring_core` | **latent hazard** | `ring_core::Ring::capacity` no longer calls `Capacity::new` at all — the crossbeam variant now carries its already-validated `Capacity` instead of rebuilding one, so the accessor that once could abort on a rounding backend no longer re-runs the check |
| TY21 | ring family | n/a — coverage | `is_configuration` and `is_transient` are named in `ring_gating` and `ring_shutdown` respectively, and both mentions are `///` examples rather than calls |
| TY22 | `ring_types` | n/a — observation | `Closed`, `NameTaken` and `NameUnknown` are neither configuration nor transient, and no predicate names that third class |
