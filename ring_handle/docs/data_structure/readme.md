# Data Structure Doc Definition

### Scope

- **Purpose**: Document what a handle actually holds — one field — and the three candidate shapes for it, since that choice determines `Send`, the split's cost, and whether the family keeps its no-RMW promise.
- **Responsibility**: Name the fields, the sharing shape, the operations, and the state deliberately not carried.
- **In Scope**: Both handles' representation; the shared backend reference.
- **Out of Scope**: The ring's own fields, which are `ring_core`'s and its backends'.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Two Handles Over One Backend](001_two_handles_over_one_backend.md) | One field, three candidate shapes, and five kinds of state refused with the reason for each | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle/docs/data_structure
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### HD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| HD[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| HD9 | `Drain` | **latent hazard** | `Drain`'s bound is captured when `drain()` is called and never re-read, so a producer publishing during the drain is not seen — nothing in the type's name or signature says the bound is a snapshot |
| HD10 | `Drain` | n/a — observation | `Drain` is the only structure in the crate whose correctness argument is this crate's own; every other type's behaviour is `ring_core`'s, forwarded |
| HD11 | the D2 shape | **misleading doc** | "No per-handle state of any kind" is true of this crate's two structs and false of what they wrap: `ring_core::Producer` carries an `OverflowPolicy` field, which is why the two handles measure different widths |
| HD12 | the D2 tradeoff | n/a — observation | The ergonomic tax D2 was priced against — callers threading a lifetime through their own types — is paid by nobody: no crate in the family stores a handle in a struct of its own |
