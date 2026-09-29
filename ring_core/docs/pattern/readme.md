# Pattern Doc Definition

### Scope

- **Purpose**: Name the reusable composition shape this crate is an instance of, stated generally enough that the next crate applying it inherits the rules and the known failure mode rather than rediscovering both.
- **Responsibility**: The pattern's structure, its four applicability conditions, its two rules, and the characteristic bug it invites — with this crate's own occurrence of that bug as the evidence.
- **In Scope**: The enum-over-implementations composition shape.
- **Out of Scope**: This crate's specific code (→ [`algorithm/`](../algorithm/readme.md)); patterns internal to a backend (→ its crate).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [A Uniform Surface Over Unequal Backends](001_uniform_surface_over_unequal_backends.md) | Uniform state beside the enum, never inside a variant; the contract is the intersection and must say so; and the seam is always at the arm whose native signature cannot express it | 🔄 |



### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core/docs/pattern
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
| CO43 | the uniform surface | n/a — duplication | Uniformity is achieved separately for construction, splitting, pushing and draining, and only the push case has a named pattern. |
| CO44 | the uniform surface | n/a — observation | Every method exists on every backend; the guarantees behind two of them differ, and the pattern has no way to express that. |
| CO45 | `claim-before-move` | **measured cost** | The MPSC arm performs a reservation and a write where a direct push would do one call, so uniformity costs one extra step on the success path. |
| CO46 | `claim-before-move` | n/a — observation | Two of the three backends need nothing from this pattern, which is what makes its applicability condition worth stating explicitly. |
