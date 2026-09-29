# Integration Doc Definition

### Scope

- **Purpose**: Work out a dependency list of five and the much larger closure it reaches, and a position on the export surface that this family's own Contract ruling makes load-bearing for three capabilities this crate does not implement.
- **Responsibility**: Enumerate the seams, the absences, and the failure each carries.
- **In Scope**: The five declared crates; the undeclared `ring_handle`; the unreachable `ring_wait` and `ring_flush`.
- **Out of Scope**: The mechanism each dependency implements, which is that crate's own documentation.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Declared Edges and the Reached Closure](001_declared_edges_and_the_reached_closure.md) | The seam table, the closure as a regenerable count, and the two config fields nothing on the arc reads | 🔄 |
| 002 | [The Crate the Export Surface Routes Through](002_the_crate_the_export_surface_routes_through.md) | This family's own Contract ruling makes three capabilities reachable only through `build`; what that obliges | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_factory/docs/integration
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FC[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FC[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FC17 | `ring_factory` | n/a — observation | Every declared edge is used and three quarters of the manifest is about edges that are not: 15 comment lines against 5 dependency lines |
| FC18 | `ring_factory` | n/a — observation | Nineteen crates in the reached closure, five declared, and the difference is invisible here — the manifest names direct edges and nothing in this crate states the transitive set |
| FC19 | `ring_factory` | n/a — unenforced | G5's confinement half cannot fail, and the half that can is not the one this instance is about — a second instance of the vacuous-gate class first found in G6 |
| FC20 | `ring_factory` | n/a — unadopted | Eight empty `Error` impls across the family, five wrapping variants, and zero `source()` implementations, so every error chain in the family walks to length one |
