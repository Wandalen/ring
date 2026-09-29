# Non-Functional Requirement Doc Definition

### Scope

- **Purpose**: Give this crate's one qualitative promise a threshold and a measurement, so that "swappable backends" is a reading rather than a claim.
- **Responsibility**: The requirement, its numeric threshold, the method, the value measured, and the part of it still unmeasured.
- **In Scope**: The caller-facing cost of moving a program between backends.
- **Out of Scope**: Comparative throughput between backends (→ `ring_bench`); each backend's own performance requirements (→ its crate).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [A Backend Swap Is a Build Flag, Not a Rewrite](001_backend_swap_is_a_build_flag.md) | Threshold: one differing line, zero `cfg` in the caller. Measured at 1 and 0 — plus the guard test that keeps the measurement from going vacuous | 🔄 |



### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core/docs/non_functional_requirement
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
| CO39 | backend swap | **misleading doc** | Swapping backends is a build flag with no code change except for `DropOldest`, which changes `Ring::new` from `Ok` to `Err`. |
| CO40 | `src/lib.rs` | n/a — observation | Every method here is a `match` forwarding to another crate, and none is marked `#[ inline ]`. |
| CO41 | `try_recv_batch` | **measured cost** | The crossbeam arm collects into an intermediate `Vec` per drain; the two in-house arms extend the caller's buffer in place. |
| CO42 | the no-atomic threshold | n/a — unenforced | The zero-atomic requirement is asserted in a different crate's test suite and not at all in this one. |
