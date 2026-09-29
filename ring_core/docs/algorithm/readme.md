# Algorithm Doc Definition

### Scope

- **Purpose**: Specify the two procedures this crate actually owns — dispatch to a backend on publish, and dispatch to a backend on drain — since everything else on the path belongs to a backend and is documented there.
- **Responsibility**: The per-backend step sequences, the shared overflow resolution that runs after all of them, the seam where a by-value backend signature nearly broke the uniform refusal contract, and the one place uniformity costs a copy.
- **In Scope**: `try_push`, `try_push_batch`, `try_recv`, `try_recv_batch`, and the enum dispatch beneath all four.
- **Out of Scope**: Each backend's internal publish and commit protocol (→ [`ring_spsc`](../../../ring_spsc/docs/algorithm/readme.md), [`ring_mpsc`](../../../ring_mpsc/docs/algorithm/readme.md)); `crossbeam-queue`'s internals, which are upstream.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Backend Dispatch and the Refusal Seam](001_backend_dispatch_and_the_refusal_seam.md) | Why the dispatch is an enum rather than a trait, and the shipped bug at the MPSC arm that the doc tests could not have caught | 🔄 |
| 002 | [Uniform Drain Over Three Drain Shapes](002_uniform_drain_over_three_shapes.md) | One surface over a batch-shaped backend and a pop-shaped one — including why `drain_up_to( 1 )` rather than `drain()` is load-bearing | 🔄 |



### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core/docs/algorithm
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
| CO1 | `try_push` | **latent hazard** | The one wildcard arm was on the policy enum, so a new overflow policy was the single addition that compiled silently; both remaining `Resolution` variants are named now. |
| CO2 | the three backends | n/a — observation | Each backend refuses in its own shape — the record, a `RingError`, or a `Result< (), T >` — and `try_push` normalizes all three. |
| CO3 | batch methods | **latent hazard** | The two batch methods returned a load-bearing count the compiler would not make anyone read; `#[ must_use ]` now does, and it caught twenty-four discards across five crates the moment it was added. |
| CO4 | `try_recv_batch` | **measured cost** | The in-house arms of the batch drain are a loop over the single-record path, so a batch saves a call per record and nothing else. |
