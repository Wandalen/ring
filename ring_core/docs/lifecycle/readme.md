# Lifecycle Doc Definition

### Scope

- **Purpose**: Specify the two irreversible moments in a ring's life — the backend choice at construction, and the borrow chain that turns an owner into two handles — and the occupancy states a caller programs against between them.
- **Responsibility**: Constructor selection and its one rejection; the owner-to-handle borrow chain, what it forecloses, and the drop obligation at the end; the occupancy states, which transitions a single caller can cause, and what each reading bounds.
- **In Scope**: `Ring::new`, `Ring::new_crossbeam`, `Ring::ends`, `Ends::split`, `Producer::try_clone`, `Drop`; occupancy as seen through `len`, `is_empty`, `free_capacity`, `is_full`, and the push/drain outcomes.
- **Out of Scope**: The publish and drain procedures between those moments (→ [`algorithm/`](../algorithm/readme.md)); each backend's internal cursor states (→ its crate); closure, which this layer deliberately does not model (→ `ring_shutdown`).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Construction and Backend Selection](001_construction_and_backend_selection.md) | Why one backend is chosen by a config field and another by a separate constructor, and why the same overflow policy is valid at one and rejected at the other | 🔄 |
| 002 | [The Ends Split and Handle Lifetimes](002_ends_split_and_handle_lifetimes.md) | The three-step borrow chain, why the intermediate `Ends` is load-bearing rather than ceremony, and the cross-thread shape it forces on callers | 🔄 |
| 003 | [Occupancy Across Backends](003_occupancy_across_backends.md) | Three states, two transitions per caller, and the reason a sample-then-act loop is wrong while a return-value loop is right | 🔄 |



### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core/docs/lifecycle
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### CO[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| CO[0-9]+ ' readme.md
# instances:                3
# finding headings inside:  6
# rows in the table below:  6
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CO33 | `Ring::new` | n/a — observation | `DropOldest` is refused by the in-house backends and honoured by crossbeam, so the same config succeeds or fails depending on a build flag. |
| CO34 | `Ring::new` | n/a — observation | The producer-count branch is written as a two-arm `match` rather than `if`/`else`, and the source says why. |
| CO35 | `Ends` | n/a — observation | `ends()` then `split()` is two calls because `ring_mpsc` needs an intermediate; the other two backends could have split in one. |
| CO36 | `Ends` | n/a — doc gap | `split` borrows `&'a mut self`, so a caller who lets `ends` drop gets a borrow error the documentation does not anticipate. |
| CO37 | `occupancy` | n/a — observation | `free_capacity` is on the producer, `len` on the consumer, and at MPSC a caller can observe a pair that sums to neither zero nor the capacity. |
| CO38 | `len_and_is_empty_agree_at_every_point_of_a_lap` | n/a — coverage | The consumer-side agreement is asserted across a whole lap, single-threaded, which is exactly where CO37 says it holds. |
