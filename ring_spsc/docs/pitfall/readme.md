# Pitfall Doc Definition

### Scope

- **Purpose**: Document the trap this crate's own role creates — that being the family's validated reference invites reading its properties as properties of the ring design.
- **Responsibility**: Name the trap, the properties that silently fail to carry, the failure each produces, and what actually mitigates it.
- **In Scope**: Reasoning and code carried from this configuration to the multi-producer one.
- **Out of Scope**: The multi-producer ring's own pitfalls (→ [`ring_mpsc`](../../../ring_mpsc/docs/pitfall/readme.md)); whether either wins the benchmark.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [SPSC Correctness Does Not Transfer to MPSC](001_spsc_correctness_does_not_transfer.md) | Seven properties established here, every one of which is false under MPSC, and why the code transfers when the reasoning does not | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc/docs/pitfall
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
| SP44 | `non-transfer` | n/a — observation | Code written against this crate cannot be pointed at `ring_mpsc` without editing, because `split` returns a pair here and `Ends` there. |
| SP45 | `non-transfer` | **latent hazard** | Moving code from `ring_mpsc` to `ring_spsc` is the dangerous direction, and nothing prevents it beyond the same signature change. |
| SP46 | `departure` | n/a — observation | A departed producer leaves its published records drainable; a departed consumer stalls the producer permanently. |
| SP47 | `RingError::Full` | **misleading doc** | `Full` is the honest error and it names a transient condition, so a caller reading only the error type will retry a permanent failure. |
