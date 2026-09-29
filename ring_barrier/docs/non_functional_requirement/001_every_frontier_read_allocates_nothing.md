# NFR: Every Frontier Read Allocates Nothing

### Scope

- **Purpose**: Put a measured number on every barrier operation, and show where the barrier side's cost differs from the gating side's.
- **Responsibility**: Give the measurement, the multiplier `wait_for` still applies to the read count, the reason the multiplier no longer reaches the allocator, and what now keeps the number honest.
- **In Scope**: The cost of a barrier read, and the test that measures it.
- **Out of Scope**: The removed allocation's cause and removal, which belong to `ring_cursor` — see [`ring_gating`'s NFR 001](../../../ring_gating/docs/non_functional_requirement/001_every_gating_read_allocates_nothing.md).

### Measured

A counting `GlobalAlloc` wrapped around `std::alloc::System`. It is no longer a
scratch binary and a table typed out by hand: it is
`ring_barrier/tests/allocation_test.rs`, one test, run by every `cargo
nextest` invocation in the family.

| Operation | Allocations, before | Bytes, before | Allocations, now | Bytes, now |
|-----------|--------------------:|--------------:|-----------------:|-----------:|
| `Barrier::over( … )` | 0 | 0 | 0 | 0 |
| `len()` + `is_empty()` | 0 | 0 | 0 | 0 |
| `frontier()` — 1 dependency | 1 | 8 | **0** | **0** |
| `frontier()` — 3 dependencies | 1 | 24 | **0** | **0** |
| `frontier()` — 0 dependencies | 0 | 0 | 0 | 0 |
| `available( … )` | 1 | 8 | **0** | **0** |
| `admits( … )` | 1 | 8 | **0** | **0** |
| `frontier()` ×1000 | 1000 | 8000 | **0** | **0** |
| `wait_for( …, None, 1 )`, satisfied at once | 2 | 16 | **0** | **0** |
| `wait_for( …, Spin, 10_000 )`, budget spent | 10 000 | 80 000 | **0** | **0** |

Every row above zero came from one `collect()` one crate down, and every one of
them went to zero when it was removed. The "before" column is kept because it is
what this document said for its whole life until then, and because the shape of
the mistake matters more than the number: the cost was real, measured, correctly
attributed, and still nothing anywhere would have noticed it changing.

The rows are not restated here from memory. They are the test's own assertion
labels:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the fold every row below goes through --'
awk '/^pub fn slowest\( cursors : &\[ PaddedCursor \] \)/{ f = 1 } f { print } f && /^\}$/{ exit }' ring_cursor/src/lib.rs
echo '  -- every operation this crate asserts allocation-free, from the test itself --'
# joined into one line first: one of these ten assertions is wrapped across
# five source lines, and a line-oriented match would silently find only nine
tr '\n' ' ' < ring_barrier/tests/allocation_test.rs | tr -s ' ' \
  | command grep -oE '\( 0, 0 \), "[^"]+"' | sed 's/( 0, 0 ), /    /'
# `wc -l` and not `grep -c`: the file is one line by this point, so a line
# count would report 1 no matter how many operations the test asserts
printf '    operations asserted allocation-free: %s\n' \
  "$( tr '\n' ' ' < ring_barrier/tests/allocation_test.rs | tr -s ' ' \
      | command grep -oE '\( 0, 0 \), "[^"]+"' | wc -l )"
echo '  -- and the control arm, without which every zero above is unfalsifiable --'
command grep -c 'control_calls >= 1' ring_barrier/tests/allocation_test.rs \
  | sed 's/^/    assertions that a deliberate allocation is seen: /'
```

Live output:

```
  -- the fold every row below goes through --
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  cursors.iter().map( | c | c.load( GATING ) ).min()
}
  -- every operation this crate asserts allocation-free, from the test itself --
    "Barrier::over"
    "len() + is_empty()"
    "frontier() — 1 dependency"
    "frontier() — 3 dependencies"
    "frontier() — 0 dependencies"
    "available( … )"
    "admits( … )"
    "frontier() ×1000"
    "wait_for( …, None, 1 ), satisfied at once"
    "wait_for( …, Spin, 10_000 ), budget spent — 10 000 reads, 0 allocations"
    operations asserted allocation-free: 10
  -- and the control arm, without which every zero above is unfalsifiable --
    assertions that a deliberate allocation is seen: 1
```

The control arm is the row that is easy to leave out and fatal to omit: a
counting allocator that was never installed, or that counts into a static the
test does not read, reports zero for everything — which is precisely the answer
being looked for. So the test allocates a `Vec` on purpose first, through the
same counter, in the same process, and fails if that reads zero.

### The Cause Was One Crate Down

```rust
// ring_cursor/src/lib.rs, before commit b7e075ca
let positions : Vec< Seq > = cursors.iter().map( | c | c.load( GATING ) ).collect();
ring_seqno::slowest( &positions )
```

`ring_seqno::slowest` takes `&[ Seq ]`; `ring_cursor` holds cells. The `Vec` was
the price of that boundary. Removing it —
`cursors.iter().map( | c | c.load( GATING ) ).min()` — was a `ring_cursor`
decision recorded there, not a proposal here, and it collapsed the boundary
along with the allocation: `ring_seqno::slowest` is no longer called from
`ring_cursor` at all. The producer half stopped paying the same 8-byte cost
through the same line at the same moment
([`ring_gating`'s NFR 001](../../../ring_gating/docs/non_functional_requirement/001_every_gating_read_allocates_nothing.md)).

What is *not* shared is the multiplier, and the multiplier is the part of this
document that survived the fix.

### BR13 — `wait_for` Multiplies Whatever One Read Costs

| Caller | Reads per operation |
|--------|--------------------|
| `ring_gating::headroom`, per claim | 1 |
| `Barrier::available`, per query | 1 |
| **`Barrier::wait_for`, per call** | **1 per spin, plus 1** |

That multiplier is a property of `wait_for`'s own loop and is unchanged. What
changed is what it multiplies:

| Call | Reads | Allocations, before | Allocations, now |
|------|------:|--------------------:|-----------------:|
| `wait_for`, predicate holds on attempt 1 | 2 | 2 | **0** |
| `wait_for`, `spins = 10_000`, never satisfied | 10 000 | 10 000 | **0** |

The `+ 1` is the second read `wait_for` performs after the wait succeeds
([`algorithm/002`](../algorithm/002_wait_for_asks_twice.md)); the exhausted case
never reaches it, so it pays exactly the budget in reads. Both rows' allocation
columns are asserted by name in
`ring_barrier/tests/allocation_test.rs`, and both appear in the block at
the top of this file.

**80 kB of 8-byte allocations for one failed wait** is what this said, on the
path whose entire purpose is to wait cheaply, and it was worse with
`WaitKind::Park`, which sleeps 50µs per attempt — the same ten thousand
allocations spread over half a second, where a profiler samples them as nothing
at all ([`item/002`](../item/002_the_five_accessors_and_the_wait.md) § BR12).
That last part is the reason the number was worth measuring rather than
profiling for, and it is the reason the zero is now asserted rather than
observed: a cost that a profiler cannot see is a cost that comes back unnoticed.

The mitigation this section used to propose — hoisting the read out of the
predicate, comparing against one frontier per call rather than one per attempt —
is now moot, and was always the worse of the two options: a hoisted frontier is
a snapshot, so the wait would spin against a value that cannot advance. The
version this section called correct is the one that shipped. It reads once per
attempt and folds without allocating.

**Disposition:** applied — the per-read allocation is gone from the whole
chain, removed in `ring_cursor/src/lib.rs` (commit `b7e075ca`) by folding
the loads in place rather than by either mitigation weighed here, and this
crate's own share of the work is
`ring_barrier/tests/allocation_test.rs`, which asserts every operation in
the table above at zero and carries a control arm that fails when the counter is
not watching. Now prints: `operations asserted allocation-free: 10`

### The Free Case Is No Longer the Only Free One

`frontier()` on an empty barrier allocated nothing even before the fix:
`collect()` from an empty iterator yields `Vec::new()`, which never touches the
allocator. That made the one barrier costing nothing to read the one that always
answers `None` ([`decisions/001`](../decisions/001_zero_for_a_barrier_over_nothing.md)).

It is worth keeping because it is the case that would have stayed green through
the entire regression. A test suite covering only the empty barrier would have
reported zero allocations before the fix and zero after, and the eight rows that
actually moved would have moved unobserved. The zero-dependency row is still in
the table, and still asserted, for exactly that reason — not because it proves
anything about the fold, but because it is what a measurement looks like when it
is measuring nothing.

### Nothing in the Family Benchmarks Any of This

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r -i 'barrier\|gating' ring_bench/src/*.rs \
  || echo '(the bench crate names neither the producer nor the consumer half)'
# control: the identical expression for what it does measure
grep -rhoE 'ring_[a-z_]+' ring_bench/src/*.rs | sort -u | head -6
```

Live output:

```
/// for a torn read (`docs/hard_problem/129_gating_so_the_producer_never_laps.md`):
ring_bench
ring_core
ring_factory
ring_flush
ring_handle
ring_mpsc
```

`ring_bench` benchmarks the two assembled rings, `ring_spsc` and `ring_mpsc`,
and **neither uses a `Barrier`**. They do not even gate the same way as each
other ([`ring_gating`'s workaround/002](../../../ring_gating/docs/workaround/002_the_multi_consumer_path_no_ring_uses.md) § G19):

| | Gates through | Allocated per read, before | Allocated per read, now | Uses `Barrier` | Benchmarked |
|--|---------------|:--------------------------:|:-----------------------:|:--------------:|:-----------:|
| `ring_spsc` | `CursorPair`, two loads | 0 | 0 | ✘ | ✔ |
| `ring_mpsc` | `GatingSet` of one consumer | 1 | **0** | ✘ | ✔ |
| `ring_consume` | `Barrier` | 1 | **0** | ✔ | ✘ |

The **Benchmarked** column is the one that has not moved. `ring_consume` is
still not benchmarked and this crate is still exercised only by tests, so a
green benchmark still says nothing about either. What the fix changed is that
the number is now asserted by a test instead of measured once by hand — a
weaker instrument than a benchmark for questions of speed, and a much stronger
one for this question, which is whether a count is zero.

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [002_a_non_blocking_wait_must_look_exactly_once.md](002_a_non_blocking_wait_must_look_exactly_once.md) | The one wait shape that pays exactly two allocations by contract |

### BR42 — The Per-Attempt Cost Scales With Two Numbers the Caller Passes Separately

A stalled `wait_for` costs `spins × len()` atomic loads, and neither factor is
visible from the other's call site. `spins` is an argument to `wait_for`;
`len()` is a property of the slice handed to `over`, often in a different
function and sometimes a different crate.

At the family's default budget of 1024 and a chained consumer's eight
dependencies that is 8,192 loads to answer *not yet*. This finding is untouched
by the allocation fix and outlived it: the loads were always the larger term,
they are what remains now that the allocations are gone, and nothing in either
signature suggests the two numbers multiply. The crate's own documentation of
`wait_for` still discusses the budget without mentioning the fold underneath it.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '  pub fn wait_for( &self, from : Seq, count : u64, kind : WaitKind, spins : usize )' ring_barrier/src/lib.rs
# the fold, and the default budget it runs against. `-n` is deliberately absent
# on the fold: a line number here goes stale on the next edit anywhere above it
# in ring_cursor, and this section names the expression, never its address
echo '  -- the fold, and the default budget it runs against --'
command grep -A4 "pub fn slowest" ring_cursor/src/lib.rs | command grep -E "iter|map|min"
command grep "DEFAULT_SPINS" ring_wait/src/lib.rs | head -2
```

Live output:

```
  pub fn wait_for( &self, from : Seq, count : u64, kind : WaitKind, spins : usize )
  -> Result< Seq, RingError >
  {
    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
    self.frontier().ok_or( RingError::Empty )
  }
  -- the fold, and the default budget it runs against --
  cursors.iter().map( | c | c.load( GATING ) ).min()
/// assert_eq!( ring_wait::DEFAULT_SPINS, 1024 );
pub const DEFAULT_SPINS : usize = 1024;
```

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_frontier_in_two_delegations.md](../algorithm/001_the_frontier_in_two_delegations.md) | Step 3, where the `Vec` was |
| [../algorithm/002_wait_for_asks_twice.md](../algorithm/002_wait_for_asks_twice.md) | The `+ 1` |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | Why eight methods carry `#[ must_use ]` |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_field_and_a_sixteen_byte_view.md](../data_structure/001_one_field_and_a_sixteen_byte_view.md) | The type that owns no heap of its own |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_zero_for_a_barrier_over_nothing.md](../decisions/001_zero_for_a_barrier_over_nothing.md) | The case that was free before the rest were |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_barrier_readings.md](../item/001_the_three_barrier_readings.md) | The optimization BR18's test guards |
| [../item/002_the_five_accessors_and_the_wait.md](../item/002_the_five_accessors_and_the_wait.md) | `Park`, the variant that hides the cost |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md](../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md) | The full ledger, including the zero rows |
| [../lifecycle/002_a_consumer_draining_behind_a_producer.md](../lifecycle/002_a_consumer_draining_behind_a_producer.md) | Steps 2a and 2c per iteration |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor::slowest` in `ring_cursor/src/lib.rs` | The fold — the `collect()` that was, and the `min()` that is |
| `ring_seqno::slowest` in `ring_seqno/src/lib.rs` | The signature that used to require it |
| `ring_wait::wait_until` in `ring_wait/src/lib.rs` | The loop that repeats the read |
| `ring_bench/Cargo.toml` | What is benchmarked instead |

### Tests

| File | Relationship |
|------|--------------|
| `every_barrier_operation_allocates_nothing` in `tests/allocation_test.rs` | Every row of the measured table, and the control arm under it |
| `a_consumer_waiting_on_a_producer_thread_makes_progress` in `tests/barrier_test.rs` | The 10,000-spin `Yield` call |
| `wait_for_gives_up_with_empty_when_the_budget_runs_out` in `tests/barrier_test.rs` | A budget deliberately exhausted |
