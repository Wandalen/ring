# Invariant Doc Definition

### Scope

- **Purpose**: State the two properties that must hold for a composition layer to be worth having — that it costs nothing on the path it composes, and that swapping the thing underneath it changes nothing a caller can observe.
- **Responsibility**: Each invariant's statement, the behaviours deliberately excluded from it, what would break it, and how it is checked — mechanically where a behavioural test cannot reach.
- **In Scope**: Constraints `ring_core` itself must satisfy.
- **Out of Scope**: Each backend's own invariants (→ [`ring_spsc`](../../../ring_spsc/docs/invariant/readme.md), [`ring_mpsc`](../../../ring_mpsc/docs/invariant/readme.md)); `free_capacity`'s split contract, which is a caller-facing trap rather than a crate invariant (→ [`pitfall/`](../pitfall/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [This Crate Adds No Atomic of Its Own](001_no_atomic_of_its_own.md) | Keeps `ring_spsc`'s zero-read-modify-write assertion true through composition — a failure that would surface two crates away from its cause | 🔄 |
| 002 | [Uniform Delivery Across Every Backend](002_uniform_delivery_across_backends.md) | Exactly-once delivery on every backend, with three behaviours explicitly excluded and each exclusion argued rather than discovered in a test | 🔄 |



### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core/docs/invariant
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
| CO23 | `src/lib.rs` | n/a — observation | Zero atomics in code, one in prose, zero `unsafe`, and no `ring_stats` dependency — the invariant holds by four independent measurements. |
| CO24 | the no-atomic rule | n/a — unenforced | Nothing in the build or the test suite fails if an atomic is added to this crate. |
| CO25 | `the_same_program_behaves_identically_on_every_backend` | n/a — coverage | Uniform delivery is asserted by a single test that a helper re-runs for each compiled backend. |
| CO26 | the uniformity claim | **misleading doc** | Records delivered are identical across backends; the occupancy readings observably are not, and the invariant's title does not distinguish them. |
