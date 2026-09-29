# Integration Doc Definition

### Scope

- **Purpose**: Record which crates this one depends on, which of those edges were declared and which had to be added, and the two Contract names that turned out not to compose with each other.
- **Responsibility**: State each edge, its reason, and the evidence that it is reached rather than merely declared.
- **In Scope**: The nine manifest edges; the reached closure; the `ring_flush`/`ring_factory` gap; the `Display` impl this crate's first build forced into `ring_flush`.
- **Out of Scope**: The export Contract's own membership, which is decided at the family level rather than by this crate; `ring_handle`'s omission of `try_clone`, which is [`pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md)'s subject.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Declared Edges and the Three That Were Missing](001_declared_edges_and_the_three_that_were_missing.md) | Six edges assigned, three added to write signatures, and two Contract names that do not compose | 🔄 |
| 002 | [The Only Consumer of Two Contract Names](002_the_only_consumer_of_two_contract_names.md) | Consumer counts per exported name, and the Contract crate this one uses hardest and never declares | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench/docs/integration
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### BN[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| BN[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BN17 | `integration/001`'s `### Tests` table | n/a — unenforced | The only cross-crate test citation in this crate has to be written in the two-column File/Relationship schema, which is the one schema `citations.py` cannot read, so it is checked by nothing |
| BN18 | `impl core::error::Error for RunError` | n/a — doc gap | The Compatibility Requirements table makes `Display` and `Error` the precondition for wrapping a dependency's error; both impls landed, and neither implements `source`, so nothing chains |
| BN19 | the five exported names' consumer counts | n/a — observation | Two of the five names on the export Contract have exactly one consumer and it is this crate, while `ring_types` on the same Contract is taken by thirty |
| BN20 | `ring_handle` | n/a — doc gap | `ring_handle` supplies the type every Contract candidate is built through, and appears nowhere in this crate's manifest or code |
