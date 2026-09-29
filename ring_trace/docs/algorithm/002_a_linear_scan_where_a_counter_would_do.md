# Algorithm: A Linear Scan Where a Counter Would Do

### Scope

**Purpose:** Record that `count_of` answers a counting question by scanning the
whole log under the producers' lock, measure how long that hold lasts and what it
costs a producer, and set it against the reasoning the crate already applied to
`entries()`.

**Responsibility:** The three read methods and their lock-hold durations, the
documented reason `entries()` copies, the equivalent operation in `ring_stats`,
and a contended measurement of a producer running behind a scanning reader.

**In Scope:** `ring_trace/src/lib.rs:287-290`, `:301-302`, `:339-342`;
`ring_trace/tests/trace_test.rs:229-236`; `ring_stats/src/lib.rs`.

**Out of Scope:** Why the lock is a `Mutex` at all is
[`decisions/002`](../decisions/002_a_mutex_over_a_lock_free_log.md). The
disabled path, which takes no lock, is
[`algorithm/001`](001_the_early_return_that_is_the_whole_feature.md).

---

## Three Reads, One Lock

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the three reads, and how long each holds the lock --'
command grep -m1 -A3 -F '  pub fn len( &self ) -> usize' ring_trace/src/lib.rs
command grep -m1 -A3 -F '  pub fn count_of( &self, op : TraceOp ) -> usize' ring_trace/src/lib.rs
echo '  -- the reason entries() copies --'
command grep -m1 -A2 -F '  /// A copy rather than a borrow: handing out a guard would let a caller hold' ring_trace/src/lib.rs
echo '  -- what the neighbouring crate does for the same question --'
command grep 'pub fn published\|pub fn claimed\|fetch_add\|pub fn counts' ring_stats/src/lib.rs | head -6
echo '  -- and the test that calls count_of five times over --'
command grep -m1 -A7 -F '  for ( i, op ) in TraceOp::ALL.iter().enumerate()' ring_trace/tests/trace_test.rs
```

Live output:

```
  -- the three reads, and how long each holds the lock --
  pub fn len( &self ) -> usize
  {
    self.entries_guard().len()
  }
  pub fn count_of( &self, op : TraceOp ) -> usize
  {
    self.entries_guard().iter().filter( |e| e.op == op ).count()
  }
  -- the reason entries() copies --
  /// A copy rather than a borrow: handing out a guard would let a caller hold
  /// the lock across arbitrary code, and the lock is on the path producers take.
  ///
  -- what the neighbouring crate does for the same question --
//! bound by one `fetch_add` on one cache line, unmoved by adding cores or
    self.claimed.fetch_add( n, Ordering::Relaxed );
    self.published.fetch_add( n, Ordering::Relaxed );
    self.consumed.fetch_add( n, Ordering::Relaxed );
    counter.fetch_add( n, Ordering::Relaxed );
    self.wait_nanos.fetch_add( nanos, Ordering::Relaxed );
  -- and the test that calls count_of five times over --
  for ( i, op ) in TraceOp::ALL.iter().enumerate()
  {
    for _ in 0..=i { trace.record( *op, Seq( 0 ), 1 ); }
  }

  let summed : usize = TraceOp::ALL.iter().map( |op| trace.count_of( *op ) ).sum();
  assert_eq!( summed, trace.len(), "no entry is unaccounted for or double-counted" );
  assert_eq!( summed, 1 + 2 + 3 + 4 + 5 );
```

## What the Hold Costs

*This probe's lock-hold and producer-throughput figures come from a one-off
scratch binary under `-tr_probe/`, run once and swept afterward per this
project's convention for temporary files — it cannot be re-run to
reconfirm. TR3's disposition below only edited a doc comment; `len`,
`count_of`, and `entries()`'s bodies are unchanged, so the numbers below
stand as a recorded measurement rather than a live guarantee.*

```rust
// -tr_probe/src/bin/scan_cost.rs
// How long each read holds the lock, at two log sizes; then a producer running
// for a fixed 200 ms with and without one reader calling `count_of` in a loop.
// The reader is a tight loop and the log is growing under it — this is the
// worst case a polling diagnostic produces, not an average.
for readers in [ 0usize, 1 ]
{
  let trace = filled( 200_000 );
  std::thread::scope( | scope |
  {
    for _ in 0..readers
    {
      scope.spawn( move || { while !stop.load( Relaxed ) { black_box( trace.count_of( TraceOp::Drop ) ); } } );
    }
    scope.spawn( move ||
    {
      let deadline = Instant::now() + Duration::from_millis( 200 );
      let mut n = 0u64;
      while Instant::now() < deadline
      {
        for _ in 0..64 { trace.record( TraceOp::Publish, Seq( n ), 1 ); n += 1; }
      }
      landed.store( n, Relaxed );
      stop.store( true, Relaxed );
    } );
  } );
}
```

Two independent runs, `--release`:

```
  1000 entries:  len 80 ns   count_of 1200 ns   entries 960 ns
  100000 entries:  len 80 ns   count_of 210282 ns   entries 111761 ns
  0 scanning reader(s): producer landed 5970944 records in 200 ms
  1 scanning reader(s): producer landed 2304 records in 200 ms

  1000 entries:  len 40 ns   count_of 760 ns   entries 600 ns
  100000 entries:  len 80 ns   count_of 189641 ns   entries 113521 ns
  0 scanning reader(s): producer landed 5836672 records in 200 ms
  1 scanning reader(s): producer landed 3456 records in 200 ms
```

---

### TR3 — The Crate Refused to Let a Caller Hold the Lock and Then Held It Itself

`entries()` copies rather than borrowing, and says why: "handing out a guard
would let a caller hold the lock across arbitrary code, and the lock is on the
path producers take." That is exactly the right reasoning, and it is applied to
one of the three reads. `count_of` acquires the same lock and holds it across a
full linear scan — at 100,000 entries, about 200 µs, measured twice. `len` is
constant at 40–80 ns because a `Vec`'s length is a field.

The consequence is not theoretical. A producer recording for a fixed 200 ms
alongside a single reader polling `count_of` lands roughly 2,300–3,500 records,
where the same producer alone lands about 5.9 million. The reader is a tight loop
and the log is growing under it, so this is the worst case rather than an
average — but polling a counter while a run proceeds is the ordinary way a
diagnostic gets used, and a producer path is exactly where the crate said it did
not want a caller-held lock.

**Finding.** The concern `entries()` documents applies with more force to
`count_of`, which is the method whose hold time grows without bound. Two repairs
are available and they differ in cost. The cheap one is a sentence on `count_of`
saying it is O(n) under the producers' lock and should not be polled during a
measured run. The thorough one is five `usize` counters incremented inside
`record`'s already-held guard, turning `count_of` into a field read — which is
what `ring_stats` does for the same shape of question, in `u64` atomics with no
lock at all.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A1 -F "O(n) in the" ring_trace/src/lib.rs
```

Live output:

```
  /// O(n) in the log's current length, and the scan runs under the same lock
  /// `record` takes — unlike [`len`](Self::len), which is a field read. Do not
```

**Disposition:** applied — the cheap repair: `count_of`'s doc now states its
O(n) cost, that it shares `record`'s lock rather than being a field read like
`len`, and the two-orders-of-magnitude producer-throughput cost of polling it.
The thorough repair (atomic per-kind counters plus a `counts()` method,
mirroring `ring_stats`) is new API surface rather than a disposition of this
finding — left to a feature task if the O(n) doc caveat proves insufficient in
practice. Now prints: `O(n) in the log's current length`

---

### TR4 — The Suite's Own Consistency Check Scans the Log Five Times

`the_per_kind_counts_sum_to_the_total` is the test that proves no entry is
unaccounted for or double-counted, and it does so by calling `count_of` once per
discriminant and summing. That is five lock acquisitions and five full scans to
answer a question one pass could answer, over a log of fifteen entries where it
costs nothing.

It costs nothing *there*. The pattern is what matters: the crate's own test file
demonstrates the natural way to use `count_of` — once per kind, in a loop over
`TraceOp::ALL` — and that natural way is quadratic in the number of
discriminants against a log whose length is unbounded. A reader who copies the
idiom onto a real trace has written a five-times-200-µs lock hold on the
producers' path and has no way to know it from the documentation, which says
nothing about `count_of`'s complexity.

**Finding.** This is a doc gap rather than a defect: the loop is correct, brief,
and reads well, and at fifteen entries the alternative would be worse prose. But
it is the one worked example of `count_of` the crate ships, so it is the idiom
that propagates. Either `count_of`'s doc should carry its complexity and its lock
behaviour, or the crate should offer a single-pass `counts()` returning all five
at once — the shape `ring_stats` already uses for exactly this reason, and the
one that would make the test's own assertion a single acquisition.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/002`](../decisions/002_a_mutex_over_a_lock_free_log.md) | Why there is a lock to hold |
| [`api/002`](../api/002_shared_reference_everywhere_and_what_it_forces.md) | The `&self` that makes interior mutability necessary |
| [`pattern/001`](../pattern/001_one_accessor_for_five_lock_sites.md) | The single accessor all three reads go through |
| [`integration/001`](../integration/001_a_vocabulary_for_crates_that_never_call_it.md) | The `ring_stats` comparison, as a boundary rather than a cost |

### Sources

| Fact | Where |
|------|-------|
| `len`'s constant-time read | `ring_trace/src/lib.rs:287-290` |
| `count_of`'s scan under the guard | `ring_trace/src/lib.rs:339-342` |
| `entries()`'s documented reason for copying | `ring_trace/src/lib.rs:301-302` |
| `ring_stats` answering the same question in atomics | `ring_stats/src/lib.rs:239-319` |
| The five-scan idiom in the suite | `ring_trace/tests/trace_test.rs:229-236` |
| 200 µs holds and the producer collapse | Probe above, two runs |

### Tests

| Test | Covers |
|------|--------|
| `the_per_kind_counts_sum_to_the_total` | The five-scan idiom itself |
| `count_of_counts_only_its_own_kind` | What the scan is for |
| `concurrent_recorders_lose_no_entry` | The producers' side of the same lock |
