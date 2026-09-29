# Pitfall Doc Definition

### Scope

- **Purpose**: Record the traps this crate sets that are invisible at the point where they bite — one for callers of the surface, one for anyone reading the crate's own coverage number.
- **Responsibility**: Each trap's mechanism, why it looks correct, the concrete failure it produces, and the detection that actually catches it.
- **In Scope**: Hazards specific to a uniform surface over unequal backends, and to a crate that is two programs rather than one.
- **Out of Scope**: General ring-buffer hazards, which belong to the backends; hazards resolved by choosing `crossbeam-queue` as an interim backend (→ [`workaround/`](../workaround/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [`free_capacity` Carries Two Contracts](001_free_capacity_carries_two_contracts.md) | One signature, one binding meaning at SPSC and one advisory meaning everywhere else — a reservation loop that is correct at SPSC and a spin at MPSC | 🔄 |
| 002 | [Feature-Gated Code Reads as Uncovered](002_feature_gated_code_reads_as_uncovered.md) | Tarpaulin counts `cfg`-eliminated lines as missed, so the same suite reads 78.2% and 100% — and a gate that runs one configuration leaves the other compiled by nothing | 🔄 |



### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core/docs/pitfall
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### CO[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| CO[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  3
# rows in the table below:  3
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CO47 | `free_capacity` | n/a — unadopted | The rustdoc offers two remedies — call `try_clone` to learn the reading, or treat every reading as advisory — and all four callers took the second. |
| CO48 | occupancy readings | **misleading doc** | The same two-contract asymmetry applies to `is_full`, `len` and `is_empty`, and only `free_capacity` carries the warning. |
| CO49 | feature-gated code | n/a — coverage | The default build compiles two backends and the `--all-features` build three, and no single run covers both configurations. |
