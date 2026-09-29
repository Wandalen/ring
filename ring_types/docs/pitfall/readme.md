# Pitfall Doc Definition

### Scope

- **Purpose**: Record the two traps this crate currently sets — a doc comment that is wrong about a release build, and two error variants nothing can produce.
- **Responsibility**: For each, state the scope, the trap, the failure it produces, and the mitigation.
- **In Scope**: `Seq::next`'s overflow claim; `NameTaken` and `NameUnknown`.
- **Out of Scope**: Traps in what other crates do with these types; the classifier gap, which is a design consequence rather than a trap (→ [`../algorithm/002`](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [`Seq::next` Wraps Where Its Doc Says It Saturates](001_seq_next_wraps_where_its_doc_says_it_saturates.md) | A false statement about release-build behaviour, with a reproducing probe — and a narrowing that exhausts the space in 4.3 seconds | 🔄 |
| 002 | [Two Name Errors Nothing Constructs](002_two_name_errors_nothing_constructs.md) | Declared, classified, formatted, tested — and produced by no code in the workspace, because the `Copy` constraint forbids the payload they would need | 🔄 |

**Both traps are documentation-shaped rather than behaviour-shaped**, and that
is what makes them worth recording here rather than filing as bugs. Nothing
currently misbehaves: no reachable workload advances a `Seq` far enough to wrap,
and no caller receives a `NameTaken` it cannot handle, because none is ever
produced. What both instances record is a claim a future reader would be
entitled to rely on and should not.

**001's mitigation is a one-sentence comment edit, and this crate's docs
deliberately do not make it.** The fix belongs in `src/`, which the concurrent
implementation session owns, so the instance carries the probe and the exact
wording rather than a drive-by change.

**002's mitigation is not a fix at all.** The two variants cannot carry the name
they are about without breaking the `Copy` requirement
(→ [`../non_functional_requirement/001`](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md)),
so the honest options are to delete them or to state in the type's own docs that
they exist for a consumer the family does not have. Deciding which is above this
crate (→ [`../decisions/readme.md`](../decisions/readme.md)).


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types/docs/pitfall
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
| TY49 | `ring_types` | n/a — unenforced | `seq_does_not_wrap_within_any_reachable_workload` asserts arithmetic on `u64::MAX` and would pass unchanged if `Seq::next` were deleted |
| TY50 | `ring_types` | n/a — observation | `NameTaken` and `NameUnknown` are classified, rendered and asserted by this crate suite, so coverage reports them as exercised while nothing in the workspace raises them |
