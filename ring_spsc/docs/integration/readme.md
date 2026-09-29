# Integration Doc Definition

### Scope

- **Purpose**: Document the two boundaries this crate sits between — six sibling crates beneath it, and an export boundary above that its capability crosses while the crate itself does not.
- **Responsibility**: Name each seam, what crosses it, the assumption it carries, and the obligations the export indirection transfers to crates that can enforce them.
- **In Scope**: The six declared dependencies and the one deliberately absent; the `ring_factory` → `ring_handle` path; the gates.
- **Out of Scope**: Each dependency's own mechanism; `ring_handle`'s own design.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Family Dependency Seam](001_family_dependency_seam.md) | Five dependencies, and the four absences that state the crate's thesis | 🔄 |
| 002 | [Reached Through the Export Surface](002_reached_through_the_export_surface.md) | Why the capability is external while the crate is not, and what that indirection buys and costs | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc/docs/integration
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### SP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| SP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  4
# rows in the table below:  4
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SP18 | `Cargo.toml` | n/a — observation | The three absent ones — `ring_atomic`, `ring_claim`, `ring_gating` — are exactly the crates single-producer makes unnecessary. |
| SP19 | the module documentation | n/a — drift | `docs/workstream/008_ring_write_path.md` does not exist; the workstream lives at `008_ring_write_path/readme.md`. |
| SP20 | `adoption` | n/a — observation | The instance argues this crate is internal; the count that supports it is five code references from `ring_core` and two from `ring_bench`. |
| SP21 | `ring_core` | n/a — observation | All five references are storage or handle variants in `ring_core`'s backend enums, never a call. |
