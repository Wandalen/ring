# Non-Functional Requirement Doc Definition

### Scope

- **Purpose**: Document the two quality obligations this crate carries — one on behalf of the whole family, one as its own binary acceptance criterion.
- **Responsibility**: State each attribute, its measurement method, and the threshold that counts as met.
- **In Scope**: The correctness-floor role and the ordering obligation it implies; this crate's own five-clause Reached condition.
- **Out of Scope**: Throughput and the benchmark verdict, which are `ring_bench`'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Correctness Floor for the Family](001_correctness_floor_for_the_family.md) | Why this configuration must be proven first, and the exact-sequence assertion no other configuration can make | 🔄 |
| 002 | [Byte-Parity Over 100 000 Items](002_byte_parity_over_one_hundred_thousand.md) | This crate's own Reached condition decomposed into five claims, two of which a reasonable test can pass while violating | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc/docs/non_functional_requirement
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
| SP37 | the floor | n/a — observation | This crate is where the family's correctness properties are cheapest to establish, and `ring_mpsc` is where they have to survive contention. |
| SP38 | the floor | n/a — coverage | `a_ring_smaller_than_the_traffic_still_loses_nothing` is the wrap-under-pressure case the parity test alone would not force. |
| SP39 | the parity test | n/a — observation | The scale matches the sibling's four-producer test, so the two are comparable — and only the sibling's can fail from an ordering defect. |
| SP40 | `backpressure` | n/a — observation | `a_failed_push_returns_the_record_rather_than_swallowing_it` is the property that makes refusal usable rather than lossy. |
