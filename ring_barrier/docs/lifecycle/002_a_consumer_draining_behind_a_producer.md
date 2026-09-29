# Lifecycle: A Consumer Draining Behind a Producer

### Scope

- **Purpose**: Follow the sequence the crate exists to serve, from a producer's store to a consumer's next position, and name what each step actually costs.
- **Responsibility**: Give the drain loop step by step, place the barrier in the four-operation handshake, and record what the loop does when the wait fails.
- **In Scope**: The barrier's role in a running producer/consumer pair.
- **Out of Scope**: The `Barrier` value's own life — see [`001`](001_a_barrier_from_over_to_the_end_of_a_borrow.md).

### The Drain, Step by Step

```rust
// tests/barrier_test.rs:341-370, condensed
const TOTAL : u64 = 512;
let deps = deps_zeroed( 1 );
let barrier = Barrier::over( &deps );

// thread A
for published in 1..=TOTAL { deps[ 0 ].store( Seq( published ), Ordering::Release ); }

// thread B
let mut position = Seq::ZERO;
while position.0 < TOTAL
{
  if let Ok( frontier ) = barrier.wait_for( position, 1, WaitKind::Yield, 10_000 )
  {
    assert!( frontier.0 <= TOTAL, "read past what was ever published" );
    position = frontier;
  }
}
assert_eq!( position, Seq( TOTAL ) );
```

| Step | Who | Does | Costs |
|-----:|-----|------|-------|
| 1 | producer | `deps[ 0 ].store( Seq( n ), Release )` | one release store |
| 2 | consumer | `wait_for( position, 1, Yield, 10_000 )` | — |
| 2a | ↳ `ring_wait::wait_until` | calls `admits( position, 1 )` | 1 allocation, 1 acquire load |
| 2b | ↳ on refusal | `thread::yield_now()`, retry | up to 10,000 times |
| 2c | ↳ on success | `frontier()` **again** | 1 more allocation, 1 more load |
| 3 | consumer | `position = frontier` | — |

The consumer asked for **one** sequence and assigns itself the **whole**
frontier. That is the point of the second read: over 512 published items the
loop runs far fewer than 512 times, because each successful wait hands back
however much the producer got done in the meantime
([`algorithm/002`](../algorithm/002_wait_for_asks_twice.md)).

The number of iterations is therefore scheduler-dependent and unasserted — the
test checks only that the final position is exactly `Seq( 512 )`, never that
every intermediate value was seen. A consumer built on this contract must be
able to take a batch; one that assumed `position += 1` would silently skip.

### BR19 — The Loop Has No Failure Path

`if let Ok( frontier ) = …` — an `Err` falls through to the next iteration with
nothing recorded and nothing counted:

| `wait_for` returns | Loop does | After |
|--------------------|-----------|-------|
| `Ok( frontier )` | advances | continues |
| `Err( Empty )` — budget spent | nothing | **retries, unbounded** |

That is correct for the shape being tested — a slow producer is not an error,
and the whole point of a bounded `spins` budget is to return control rather than
to signal failure. It does mean the loop's own termination depends entirely on
the producer finishing:

- If the producer thread panics, `thread::scope` cannot join it until the main
  closure returns, and the main closure is waiting for a frontier that will
  never advance. **The test hangs rather than fails.**
- With `Yield` and a 10,000-spin budget, each failed round is ten thousand
  `yield_now` calls and ten thousand allocations before the loop even gets to
  decide to try again.

Neither has bitten — the producer is a `for` loop over stores and cannot fail —
and both are cheap to remove: an outer attempt counter that breaks with a
message turns a hang into a diagnosis. Recorded because a hanging test is the
one failure mode that costs more to investigate than the assertion it replaced,
and because the manual plan's B6 already treats this test's structure as
load-bearing.

### Where the Barrier Sits in the Handshake

The four-operation handshake, and the one the barrier answers:

| # | Operation | Crate | Cursor touched |
|--:|-----------|-------|----------------|
| 1 | claim | `ring_claim` | claimed — private, nobody reads it |
| 2 | publish | `ring_publish` | **published — what the barrier waits on** |
| 3 | available | `ring_consume` → **`ring_barrier`** | reads the published cursor |
| 4 | commit | `ring_consume` | the consumer position, which the producer's `GatingSet` gates on |

```rust
// ring_publish/tests/handshake_test.rs:157-159
let barrier = Barrier::over( core::slice::from_ref( publisher.cursor() ) );
let fresh = ring_cursor::PaddedCursor::default();
assert_eq!( Consumer::new( &fresh, barrier ).available().len(), 1 );
```

Step 3 is where this crate appears, and it appears as a *parameter* — the
`Consumer` is handed a barrier, never builds one
([`api/001`](../api/001_nine_methods_over_one_borrowed_slice.md) § BR1). The
loop closes at step 4: the consumer's commit advances a cursor in the producer's
`GatingSet`, which is what lets the producer claim again, which is
`ring_gating`'s complementary half.

So `ring_gating` and `ring_barrier` sit on opposite sides of one cycle:

```
   ring_gating: may the producer claim?   ◀── commit ── consumer
        │                                                  ▲
      claim                                                │
        ▼                                                  │
      publish ──▶ ring_barrier: may the consumer read? ─────┘
```

Neither crate knows about the other. The cycle is closed by the wiring, which is
why `handshake_test.rs` asserts the wiring itself
([`decisions/002`](../decisions/002_a_slice_rather_than_an_aggregate.md)).

### BR41 — The Only Two-Thread Test Waits With a Strategy the Production Consumer Never Picks

`a_consumer_waiting_on_a_producer_thread_makes_progress` is the crate's single
test with a real publisher on another thread. Every other assertion about
`wait_for` is made against cursors a test wrote by hand.

`ring_consume`, the one library consumer, never calls `wait_for` at all — it
composes `available` itself and leaves waiting to its own caller. So the wait
path has one test with a real race, six with a synthetic one, and zero
production callers, and the strategy that test picks is not a strategy any
shipped code passes.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c "wait_for" ring_barrier/tests/barrier_test.rs
grep -r "wait_for" --include=*.rs ring_*/src/ | sed 's|ring/||' \
  || echo '(no library call site anywhere in the family)'
# control: the identical expression for the reading consumers do reach
grep -rl "\.available(\|\.admits(" --include=*.rs ring_*/src/ | sed 's|ring/||'
```

Live output:

```
12
ring_barrier/src/lib.rs:  /// assert!( barrier.wait_for( Seq::ZERO, 1, WaitKind::None, 1 ).is_err() );
ring_barrier/src/lib.rs:  /// assert_eq!( barrier.wait_for( Seq::ZERO, 1, WaitKind::None, 1 ), Ok( Seq( 6 ) ) );
ring_barrier/src/lib.rs:  pub fn wait_for( &self, from : Seq, count : u64, kind : WaitKind, spins : usize )
ring_shutdown/src/lib.rs:  /// non-empty — `ring_wait`'s budget-exhausted report, which [`wait_for_close`]
ring_shutdown/src/lib.rs:/// use ring_shutdown::{ wait_for_close, Shutdown };
ring_shutdown/src/lib.rs:/// assert!( wait_for_close( &shutdown, WaitKind::None, 1 ).is_err() );
ring_shutdown/src/lib.rs:/// assert!( wait_for_close( &shutdown, WaitKind::None, 1 ).is_ok() );
ring_shutdown/src/lib.rs:pub fn wait_for_close( shutdown : &Shutdown, kind : WaitKind, spins : usize )
ring_barrier/src/lib.rs
ring_consume/src/lib.rs
ring_core/src/lib.rs
ring_gating/src/lib.rs
ring_mpsc/src/lib.rs
ring_spsc/src/lib.rs
```

### BR52 — The Drain Loop's Bound Is the Item Count, Not the Number of Attempts

The two-thread test drains until it has seen `TOTAL` items, retrying whenever
`wait_for` returns `Empty`. The loop is bounded by what it received rather than
by how many times it asked, which BR19 records as a hang risk against a dead
producer.

What is worth separating from that is why the shape is tempting: it is the only
form that does not need a second, arbitrary constant. Bounding by attempts means
choosing a number large enough for a slow machine and small enough to fail
usefully, and getting it wrong turns a correctness test into a flaky one. The
crate chose the version that cannot be flaky and can hang, in a suite whose
runner has its own timeout — which makes the choice defensible and leaves the
failure mode entirely outside the crate.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -B2 -A10 "const TOTAL" ring_barrier/tests/barrier_test.rs
```

Live output:

```
fn a_consumer_waiting_on_a_producer_thread_makes_progress()
{
  const TOTAL : u64 = 512;
  let deps = deps_zeroed( 1 );
  let barrier = Barrier::over( &deps );

  std::thread::scope( | scope |
  {
    scope.spawn( ||
    {
      for published in 1..=TOTAL
      {
        deps[ 0 ].store( Seq( published ), Ordering::Release );
--
  // dependency was at least there. A `Relaxed` read could report a position
  // the consumer has no happens-before edge to.
  const TOTAL : u64 = 2_000;
  let deps = deps_zeroed( 2 );
  let barrier = Barrier::over( &deps );

  std::thread::scope( | scope |
  {
    scope.spawn( ||
    {
      for published in 1..=TOTAL
      {
        deps[ 0 ].store( Seq( published ), Ordering::Release );
```

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_a_barrier_from_over_to_the_end_of_a_borrow.md](001_a_barrier_from_over_to_the_end_of_a_borrow.md) | The value's own life, which has no steps |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_wait_for_asks_twice.md](../algorithm/002_wait_for_asks_twice.md) | Why step 2c exists and what it buys |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | The barrier as a parameter, never a construction |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_slices_three_provenances.md](../data_structure/002_the_slices_three_provenances.md) | Where `publisher.cursor()` fits among the three |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_slice_rather_than_an_aggregate.md](../decisions/002_a_slice_rather_than_an_aggregate.md) | The signature that makes step 3 wireable |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_three_dependencies_and_one_dependent.md](../integration/001_three_dependencies_and_one_dependent.md) | `ring_consume` at step 3, and what it recomputes |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_frontier_never_exceeds_a_dependency.md](../invariant/001_the_frontier_never_exceeds_a_dependency.md) | Why step 3 can never outrun step 2 |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_frontier_read_allocates_nothing.md](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md) | Steps 2a and 2c, per iteration |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:243-287` | Steps 2 through 2c |
| `ring_consume/src/lib.rs:336-345` | Step 3 as the family performs it |
| `ring_publish/src/lib.rs:117` | The cursor step 2 advances |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:341-370` | The drain loop, 512 items |
| `ring_publish/tests/handshake_test.rs:157-159` | Step 3 in the four-operation handshake |
| `ring_publish/tests/handshake_test.rs:167-210` | The cycle closing — a full ring stopped until the consumer commits |
| `tests/manual/readme.md` § B6 | The check that keeps the producer thread in place |
