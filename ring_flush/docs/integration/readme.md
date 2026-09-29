# Integration Doc Definition

### Scope

- **Purpose**: Account for what this crate composes from, what it cannot reach, and what sitting on the export Contract costs a crate that is a decision rather than a mechanism.
- **Responsibility**: Name each seam, its direction, what crosses it, and the assumption it rests on.
- **In Scope**: `ring_tls` and `ring_core`; the unreachable barrier; the export boundary.
- **Out of Scope**: The consumer's own scheduling, which no crate here owns.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Two Dependencies and the Barrier It Cannot See](001_two_dependencies_and_the_barrier_it_cannot_see.md) | The dependency list, and the one named trigger no declared dependency can observe | 🔄 |
| 002 | [A Decision on the Export Surface](002_a_decision_on_the_export_surface.md) | What it means for a *policy* rather than a *thing* to be one of the five nameable crates | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_flush/docs/integration
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FL[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FL17 | the dependency block | **misleading doc** | The block under "confirm rather than take it on trust" is a typed manifest excerpt showing two dependencies, not a capture, and the manifest declares three |
| FL18 | the orphan finding | n/a — observation | Two documents recorded the same orphaned-crate finding independently, and only the one with routing authority could act; neither cites the other |
| FL19 | the quoted gate mechanism | n/a — drift | The G5 mechanism quoted here was replaced by the exact-match roster the script now uses, which is the thing its own header argues a path test gets wrong |
| FL20 | the export surface | n/a — inconsistency | `Flusher::new` takes a `ring_core::Producer`, so the family's own benchmark cannot be built without naming a crate the declared five-crate surface excludes |
