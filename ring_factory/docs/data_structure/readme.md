# Data Structure Doc Definition

### Scope

- **Purpose**: Describe the two structures this crate handles — the record it consumes and the pair it produces — neither of which it owns.
- **Responsibility**: State each structure's shape, its operations, and where its definition actually lives.
- **In Scope**: `RingConfig` as an input; the handle pair as an output.
- **Out of Scope**: The ring itself, which is `ring_core`'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Configuration Record as Input](001_the_configuration_record_as_input.md) | Five fields, already validated and already clamped before this crate sees them | 🔄 |
| 002 | [The Handle Pair as Output](002_the_handle_pair_as_output.md) | What `build` returns, from a crate that does not declare the crate defining it | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_factory/docs/data_structure
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FC[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FC[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FC9 | `ring_factory` | n/a — observation | Three of `RingConfig`'s fields carry their constraint in the type and two carry it in a setter body, so the record's legality is half checkable by a reader and half only by running the setter |
| FC10 | `ring_factory` | **latent hazard** | Config equality compares five fields and the ring it builds depends on two, so two configs that are unequal can produce indistinguishable rings — the invariant holds in the direction stated and not in its converse |
| FC11 | `ring_factory` | **measured cost** | The owner is exactly the ring and the error rides in a niche: `Ring<u32>`, `Split<u32>` and `Result<Split<u32>, BuildError>` all measure 320 bytes, so the handle pair and its error channel are free |
| FC12 | `ring_factory` | **measured cost** | A name collision constructs and drops 320 bytes to return 24 — `build_named` builds the ring before consulting the registry, and the refusal path pays the full construction |
