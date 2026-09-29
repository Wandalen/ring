# Non-Functional Requirement Doc Definition

### Scope

- **Purpose**: State the quality requirements a benchmark can pass or fail, so adoption of this crate is a measured event rather than an argued one.
- **Responsibility**: Document `ring_mpsc`'s quality thresholds and the procedure that measures each.
- **In Scope**: The evidence required before any prospective consumer takes a manifest edge on this crate.
- **Out of Scope**: The benchmark harness's own design (→ [`bench_harness`](../../../bench_harness/readme.md)); the functional contract being held regardless of speed (→ [`invariant/`](../invariant/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Measured Before Adopted](001_measured_before_adopted.md) | No consumer migrates onto this crate until its mechanism wins the family's own benchmark | 🔄 |
| 002 | [Bounded Capacity and Backpressure Policy](002_bounded_capacity_backpressure.md) | Footprint bounded by capacity rather than arrival rate, and a full-ring policy declared per priority class because the three are not interchangeable | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc/docs/non_functional_requirement
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### MP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| MP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  4
# rows in the table below:  4
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| MP37 | `measurement` | n/a — observation | `ring_bench` imports `Ring` from this crate but reaches the publish path through `ring_core`, so what is measured is the composition. |
| MP38 | `measurement` | n/a — coverage | The observation surface is neither called nor measured, so its cost is unknown as well as unused. |
| MP39 | `backpressure` | n/a — observation | A full ring returns `RingError::Full`; it never blocks, never grows, and never drops. |
| MP40 | `Capacity` | n/a — observation | The two arrays are sized once, so a burst is absorbed by refusal rather than by growth — which is what makes the bound meaningful. |
