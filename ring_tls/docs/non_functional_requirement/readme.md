# Non-Functional Requirement Doc Definition

### Scope

- **Purpose**: State the quality requirements a benchmark can pass or fail, so adoption of this crate is a measured event rather than an argued one.
- **Responsibility**: Document `ring_tls`'s quality thresholds and the procedure that measures each.
- **In Scope**: The evidence required before a prospective consumer takes a manifest edge on this crate, and the payload/alignment preconditions the capabilities layered on it depend on.
- **Out of Scope**: The benchmark harness's own design; the discipline held regardless of speed (→ [`invariant/`](../invariant/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Measured Before Adopted](001_measured_before_adopted.md) | No consumer migrates onto this crate until its buffer wins the adoption benchmark | 🔄 |
| 002 | [POD, Pointer-Free, Page-Aligned Payloads](002_pod_pointer_free_payloads.md) | The precondition for both zero-copy handoff and CoW snapshotting; absent on `wasm32` | 🔄 |




### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs/non_functional_requirement
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TL[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  2
# rows in the table below:  2
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TL41 | the adoption gate | n/a — observation | `ring_bench` measures the built `TlsBuffer` against the requirement written for the byte region, and the requirement is layout-neutral enough to apply. |
| TL42 | the payload preconditions | n/a — unenforced | `TlsBuffer< T >` has no bound on `T`, so the POD, pointer-free and alignment requirements stated here are documentation only. |
