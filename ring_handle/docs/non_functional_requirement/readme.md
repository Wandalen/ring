# Non-Functional Requirement Doc Definition

### Scope

- **Purpose**: Document the two halves of this crate's binary Reached condition, measured separately because they are independent properties with independent failure modes.
- **Responsibility**: State each attribute, its measurement method, and the threshold that counts as met — including what the threshold does not reach.
- **In Scope**: The compile-fail criterion; the `Send`-without-shared-`&mut` criterion.
- **Out of Scope**: Throughput, which is `ring_bench`'s; `ring_poll`'s own bounded-time test.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Proven by Code That Must Not Compile](001_proven_by_code_that_must_not_compile.md) | The family's only test that gets redder as the surface grows, and the two properties it does not reach | 🔄 |
| 002 | [Send Without Sync](002_send_without_sync.md) | Three independent claims in one clause, and the one criterion that can only be checked by reading the test's own source | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle/docs/non_functional_requirement
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### HD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| HD[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| HD33 | the acceptance threshold | n/a — drift | The threshold this instance grades against names four compile-fail cases and `tests/ui_test.rs` drives seven — three cases the crate added beyond the criterion are ungraded by it |
| HD34 | measurement P7 | n/a — coverage | P7's redder-not-greener evidence comes from a manual run recorded when the suite had five cases; two of the seven that exist now have never been shown to fail for the reason they were written |
| HD35 | criterion Q3 | n/a — observation | The handle pair borrows from the `Split` and so cannot move to a `'static` thread at all; Q3 passes because the only two-thread test uses `std::thread::scope`, which the criterion does not mention |
| HD36 | measurement 5 | n/a — coverage | The crate's one two-thread test has no bounded loop, so a lost publication hangs it rather than failing it, and measurement 5 — the sanitizer run that would distinguish the two — is not run anywhere in the crate or its gates |
