# ring_barrier

Consumer barrier over the minimum of dependent cursors.

Depends on [`ring_types`](../ring_types/readme.md), [`ring_cursor`](../ring_cursor/readme.md), [`ring_wait`](../ring_wait/readme.md), and on [`ring_gating`](../ring_gating/readme.md) for tests only, never from the library.

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list. [`../readme.md`](../readme.md) describes the family as a whole.

Implemented. It delivers the consumer half of the gating mechanism, which
decides how far a consumer may read, given the minimum across everything it
depends on. `ring_gating` is the producer half and reads the same set from the
other side. [`tests/barrier_test.rs`](tests/barrier_test.rs) asserts the
behaviour, which is also read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md). Every line is covered.

The difference from `ring_gating` is that capacity is not in the answer. A
producer may run one lap ahead of the slowest consumer and no further, because
capacity is what makes the next slot the one that consumer is reading. A
consumer may read up to whatever its dependencies have finished, and the number
of slots the ring has does not enter into it. The two share the cursors, not the
arithmetic. That is why `Barrier::over` takes a bare `&[ PaddedCursor ]` and
never names `ring_gating`. A barrier points at whatever cursors it depends on,
wherever they live. A gating set's are one such place, a publisher's is another.

275 lines, one type, nine methods, 57 lines of code. It has no loop, no `unsafe`,
no destructor, no named `Ordering`, no `&mut self`, and no cast between integer
widths. Every one of those is an absence the compiler cannot check, so
[`docs/`](docs/readme.md) documents the crate at full depth. What is left after
the delegation is what to ask of the cursors and what to hand back, and both are
decisions rather than code.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/` | Scope, related crates, and open trade-offs. See [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `barrier_test.rs` and the manual plan under `manual/` |

### Reading order

| Start with | For |
|------------|-----|
| [`docs/api/001`](docs/api/001_nine_methods_over_one_borrowed_slice.md) | The whole public API in one table, in three tiers |
| [`docs/algorithm/002`](docs/algorithm/002_wait_for_asks_twice.md) | Two phases, three endings, and what the second read costs |
| [`docs/lifecycle/002`](docs/lifecycle/002_a_consumer_draining_behind_a_producer.md) | The sequence this crate's behaviour is accepted against, step by step |
| [`docs/pattern/001`](docs/pattern/001_the_borrowed_view_and_the_owned_set.md) | Why the slice, and the failing test that produced the rule |
| [`docs/pattern/002`](docs/pattern/002_the_quantity_the_predicate_and_the_wait.md) | The three-rung ladder, and the four third rungs nobody calls |
| [`docs/definition/readme.md`](docs/definition/readme.md) | The full instance index and all twenty-one findings |
