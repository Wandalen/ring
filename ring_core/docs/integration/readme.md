# Integration Doc Definition

### Scope

- **Purpose**: Account for this crate's position in the family — the edges it declares downward, and the one specification above it that nothing can hold it in agreement with.
- **Responsibility**: Every dependency and every pointed absence; the divergence from `ring_handle`'s specified surface, recorded as an obligation with a named owner rather than a note.
- **In Scope**: `Cargo.toml`'s edges, and signature-level agreement with `ring_handle`.
- **Out of Scope**: The contracts themselves (→ [`api/`](../api/readme.md)); why `crossbeam-queue` was chosen (→ [`workaround/`](../workaround/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Family Dependency Seam](001_family_dependency_seam.md) | Six in-house edges, one external edge behind four flags — one of which does less than it appears to — and four crates pointedly absent | 🔄 |
| 002 | [Divergence From `ring_handle`'s Specified Surface](002_handle_surface_divergence.md) | Every semantic row agrees; the open rows are receivers and error naming, and they reduce to one blocking crate | 🔄 |



### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core/docs/integration
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### CO[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| CO[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  4
# rows in the table below:  4
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CO19 | `Cargo.toml` | n/a — observation | `ring_mpsc` and `ring_spsc` are reached only through fully-qualified paths, so the `use` block understates the seam. |
| CO20 | `crossbeam-queue` | n/a — observation | Every in-family dependency is a path dependency; the one registry dependency is the one gated behind a feature. |
| CO21 | the eight dependents | n/a — unadopted | The import profile across the eight dependent crates is `Ring` 8, `Producer` 7, `Consumer` 5, `Ends` 2, `Backend` 0. |
| CO22 | `ring_handle` | n/a — observation | The divergence this instance documents is a narrowing — `ring_handle` withholds `try_clone` and adds no method of its own. |
