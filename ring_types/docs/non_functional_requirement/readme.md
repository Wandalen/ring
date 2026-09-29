# Non-Functional Requirement Doc Definition

### Scope

- **Purpose**: State the three measurable properties this crate must hold for the family's benchmarks to mean what they claim — and, for each, the measurement that would detect a violation.
- **Responsibility**: For each, state the quality attribute, the requirement, the measurement method, and the acceptance threshold.
- **In Scope**: Allocation freedom across all six exports; closure of the three enum variant sets under future addition; compilation without `std`.
- **Out of Scope**: Absolute invariants, which are not graded (→ [`../invariant/`](../invariant/)); latency budgets, which belong to the crates that spend them.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Errors and Positions Do Not Allocate](001_errors_and_positions_do_not_allocate.md) | Five thresholds, all met — and the one whose cost is a second error type in a family whose tier 0 claims to have one | 🔄 |
| 002 | [The Enum Sets Are Closed and Asserted](002_the_enum_sets_are_closed_and_asserted.md) | Eight thresholds across three enums, three mechanisms of three strengths, and one gap — closed in three lines | 🔄 |
| 003 | [The Crate Compiles Without std](003_the_crate_compiles_without_std.md) | Six thresholds, six met — the crate now declares `#![ no_std ]`, the one line that makes the property permanent | 🔄 |

**The three requirements are enforced by mechanisms of three different kinds**,
which is why they are separate instances rather than one. 001 is carried
entirely by a derive: `Copy` cannot own an allocation and cannot coexist with
`Drop`, so violating it fails to compile and the measurement is close to
ceremonial. 002 has no derive available — nothing in Rust says "this array lists
every variant" — so its enforcement is assembled from a wildcard-free `match`, an
array length, a per-variant `contains` loop, and a coverage gate, each catching a
different mistake. 003 has a mechanism available and now uses it — the attribute, not
restraint, is what holds the property.

**003 was the cheapest fix in the crate's documentation.** Its two
formerly-failing thresholds — the crate did not declare `#![ no_std ]`, and no
gate asserted the property — collapsed into a compile error the moment that one
line was added.

**002 recorded the crate's most concrete defect, and it is now closed.**
`OverflowPolicy::ALL` could be corrupted to `[ DropOldest, DropOldest, Fail ]` —
the `#[ default ]` policy silently dropped — and that one arm passed every test
in the workspace, including the eleven references in `ring_stats` that sweep it.
The other duplicate arm was never the dangerous one: a doctest the original
analysis had not enumerated already caught it. Both are now refused by name —
`overflow_policy_has_no_overwrite_variant` gained the per-variant `contains`
loop `WaitKind`'s test already had
(→ [`002`](002_the_enum_sets_are_closed_and_asserted.md), T6).

**The obvious corruption was never the dangerous one.** `[ Fail, Fail, Fail ]`
was always caught, by `overflow_policies_partition_by_reporting`'s variant-count
assertions. Only a duplicate that preserved both counts got through, which is
why the gap survived being looked at until it was measured rather than assumed.


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types/docs/non_functional_requirement
printf 'instances:               '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TY[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TY[0-9]+ ' readme.md
# instances:               3
# finding headings inside:  3
# rows in the table below:  3
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TY44 | ring family | **measured cost** | `Copy` on `RingError` forbids a `String` payload, which is why `ring_registry` declared `RegistryError` and why three crates now spell the same name collision |
| TY45 | `ring_types` | **latent hazard** | `overflow_policy_has_no_overwrite_variant` matched all three variants in one arm, so a fourth failed the build, but a repeated entry in `ALL` passed every test in the workspace until the `contains` loop closed it (→ `002`, T6) |
| TY46 | `ring_types` | n/a — unenforced | `#![ no_std ]` is now declared (`src/lib.rs:27`), so the crate satisfies this requirement by being unable to contain a `std::` path, not merely by containing none |
