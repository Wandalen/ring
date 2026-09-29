# Integration Doc Definition

### Scope

- **Purpose**: Document the two boundaries this crate sits between — one declared dependency concealing three backends with different contracts, and an export boundary this crate sits *on* rather than behind.
- **Responsibility**: Name each seam, what crosses it, the assumption it carries, and the failure class it admits.
- **In Scope**: The `ring_core` seam and the four crates above; the five-crate Contract; gates G5 and `g3_features.sh`.
- **Out of Scope**: Each backend's own mechanism; the other four exported crates' surfaces.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [One Dependency and the Backends Beneath It](001_one_dependency_and_the_backends_beneath.md) | What one dependency conceals: three backends whose `free_capacity`, drain order and RMW contracts differ under one surface | 🔄 |
| 002 | [On the Export Surface](002_on_the_export_surface.md) | The exact inverse of `ring_spsc`'s position, and the type-leakage gap gate G5 does not cover | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle/docs/integration
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### HD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| HD[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| HD21 | the backend parity claim | **misleading doc** | "The identical test suite runs against every backend" is two tests, and the crossbeam one is behind a feature this crate's own source never uses, so the default build proves parity for one backend |
| HD22 | seam A4 | n/a — observation | A4 is the seam table's only entry marked bidirectional and it has no edge in either direction: `ring_poll` names no type of this crate and this crate's manifest names no `ring_poll` |
| HD23 | the confinement argument | n/a — drift | The instance's argument that gate G5's confinement half is vacuous is correct, and both pieces of evidence it cites have since moved — the single `export_surface.txt` became one file per family, and the gate now resolves the list per family |
| HD24 | the `ring_types` row | n/a — coverage | B4 justifies `ring_types`' place on the Contract by the error type a consumer must name, and this crate exercises none of it: `ring_types` is a dev-dependency here and no public signature of this crate mentions it |
