# Decisions Doc Definition

### Scope

- **Purpose**: Open questions this crate cannot settle on its own evidence, and questions the implementation settled against what the pre-implementation specification assumed.
- **Responsibility**: Two — what this crate is for given `ring_core` already splits, and why `is_closed()` is absent despite two API instances specifying it.
- **In Scope**: The `ring_core` overlap; the tension between this crate's own split and `ring_poll`'s non-parking constraint.
- **Out of Scope**: Settled choices documented where they take effect (the D2 backend shape → [`data_structure/001`](../data_structure/001_two_handles_over_one_backend.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [What This Crate Is For, Given `ring_core` Already Splits](001_what_this_crate_is_for.md) | Four narrowings, and the open question of whether four is enough to justify a crate | ❓ |
| 002 | [Why `is_closed` Is Absent](002_why_is_closed_is_absent.md) | Two features in tension, resolved by measurement rather than by preference | ✅ |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle/docs/decisions
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### HD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| HD[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| HD13 | the status line | n/a — drift | This instance's status line reads open and defers further resolution; `ring_factory/docs/decisions/001` is accepted and answers it, so the question is closed in one file and open in the other |
| HD14 | narrowing N4 | **wrong doc** | N4's row says three `ring_core::Ring` methods are withheld and the count is four, and one of the four — `ends()` — is not a read but the construction route the narrowing is about |
| HD15 | the chosen option | n/a — inconsistency | The accepted option composes `is_closed` around a `Shutdown` value, and this crate neither produces nor accepts one — the composition it describes has no site in this crate's surface |
| HD16 | the `ring_poll` coupling | n/a — observation | `ring_poll` is named seven times across this crate's source, tests and manual plan and executed zero times: neither test file reads a manifest, so the coupling is entirely prose in this direction |
