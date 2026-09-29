# API Doc Definition

### Scope

- **Purpose**: Fix the operation surface this crate presents to callers who must not know which backend is underneath, and mark — per operation — which promises are uniform and which are per-backend.
- **Responsibility**: Method sets, signatures, error shape, allocation behaviour, compatibility guarantees, and the recorded divergence from `ring_handle`'s specified surface.
- **In Scope**: The producer-side publish surface and the consumer-side drain surface, as contracts a caller may rely on.
- **Out of Scope**: The procedures behind them (→ [`algorithm/`](../algorithm/readme.md)); the enum they dispatch over (→ [`data_structure/`](../data_structure/readme.md)); the reconciliation with `ring_handle` (→ [`integration/`](../integration/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Producer Surface](001_producer_surface.md) | The publish side — a refusal that never destroys the record, and one reading (`free_capacity`) whose contract is strictly stronger at SPSC than the signature admits | 🔄 |
| 002 | [Consumer Surface](002_consumer_surface.md) | The drain side — emptiness is not an error, the batch appends, and every occupancy reading is advisory at every backend | 🔄 |



### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core/docs/api
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
| CO5 | `Producer` | n/a — coverage | `try_push`, `try_push_batch`, `free_capacity` and `is_full` are called by libraries; `try_clone` is not. |
| CO6 | `Producer` | n/a — doc gap | Thread-safety is a per-backend property of the borrowed handle, and the surface documentation states it nowhere. |
| CO7 | `Consumer` | **latent hazard** | The consumer-side occupancy readings inherited the producer-side asymmetry undocumented; `len` and `is_empty` state it now and name `try_recv` as the authority. |
| CO8 | `Consumer` | n/a — observation | One consumer per ring is a property of the split, not of the type, and holds because `Ends::split` is the only constructor. |
