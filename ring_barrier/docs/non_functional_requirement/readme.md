# non_functional_requirement

Two requirements, both about the wait: one prices it, one bounds it. Both are
measured rather than argued — a counting `GlobalAlloc` around
`std::alloc::System`, release build, asserted by
`ring_barrier/tests/allocation_test.rs` on every `cargo nextest` run.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Every Frontier Read Allocates](001_every_frontier_read_allocates_nothing.md) | 8 bytes per dependency per read, the budget-sized multiplier `wait_for` applies, and the fact that nothing benchmarks it |
| 002 | [A Non-Blocking Wait Must Look Exactly Once](002_a_non_blocking_wait_must_look_exactly_once.md) | `WaitKind::None`'s bounded-time contract, what enforces it, and the test whose failure mode is a hang |

### Measured, in Release

| Operation | Allocations | Bytes |
|-----------|------------:|------:|
| `Barrier::over( … )` / `len()` / `is_empty()` | 0 | 0 |
| `frontier()` — 0, 1, or 3 dependencies, or ×1000 | 0 | 0 |
| `available( … )` / `admits( … )` | 0 | 0 |
| `wait_for( …, None, 1 )`, satisfied at once | 0 | 0 |
| `wait_for( …, Spin, 10_000 )`, budget spent | 0 | 0 |

Every row was nonzero until `ring_cursor` commit `b7e075ca` removed the
`collect()` the fold went through — see
[001](001_every_frontier_read_allocates_nothing.md) for the full before/after
table and the ten assertion labels the test itself prints.

### Regenerate

The figures above are no longer typed out by hand from a scratch bin crate —
they are the live assertions in `ring_barrier/tests/allocation_test.rs`,
run by every `cargo nextest` invocation in the family
([001](001_every_frontier_read_allocates_nothing.md) quotes all ten labels and
the control arm that makes them falsifiable).

```sh
cd "$(git rev-parse --show-toplevel)"
# the fold the table above measures, and the budget wait_for runs against
grep -n "wait_until" ring_barrier/src/lib.rs
grep -n "DEFAULT_SPINS *:" ring_wait/src/lib.rs
# any bench target anywhere in the family
ls -d ring_*/benches 2>/dev/null || echo '(no crate in the family has a benches directory)'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BR13 | `ring_barrier` | **measured cost** | Measured: a stalled `wait_for( …, Spin, 10_000 )` performs 10,000 reads and **0 allocations**; a satisfied `wait_for( …, None, 1 )` performs 2 reads and **0 allocations** — the read counts still match `n` and `n + 1`, the allocations they used to cost are gone since `ring_cursor` commit `b7e075ca` |
| BR42 | `ring_barrier` | n/a — observation | A stalled `wait_for` costs `spins × len()` atomic loads and the two factors arrive from different call sites, often different crates — 8,192 at the default budget with eight dependencies, and neither signature suggests they multiply |
| BR43 | family | n/a — duplication | The one-look guarantee is asserted here and implemented in `ring_wait::pause`'s `None` arm two crates away, with neither test referencing the other; a `wait_for` that stopped forwarding `kind` would fail only this one, for a reason its name does not describe |
