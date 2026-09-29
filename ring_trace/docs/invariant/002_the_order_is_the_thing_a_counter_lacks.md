# Invariant: The Order Is the Thing a Counter Lacks

### Scope

**Purpose:** Test the crate's stated reason for existing — that a trace answers
"which operations, in what order" where a counter cannot — against what the log's
order actually is under more than one producer, and record that the suite already
knows the answer while the documentation does not.

**Responsibility:** The module doc's claim, the accessor's more careful wording,
a measurement of how far recorded order departs from sequence order under four
producers, and the one line in the suite that acknowledges the difference.

**In Scope:** `ring_trace/src/lib.rs:16-19`, `:299`;
`ring_trace/tests/trace_test.rs:108-124`, `:277-307`.

**Out of Scope:** The invariant that does hold unconditionally is
[`invariant/001`](001_disabled_means_zero_forever.md). The lock producing this
order is
[`decisions/002`](../decisions/002_a_mutex_over_a_lock_free_log.md).

---

## Two Statements of the Same Property

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the crate claims a trace answers that a counter cannot --'
command grep -m1 -A3 -F '//! `ring_stats` counts; this records. A counter answers "how many publishes"' ring_trace/src/lib.rs
echo '  -- and the precise version, at the accessor --'
command grep -m1 -F '  /// Every entry, in the order recorded.' ring_trace/src/lib.rs
echo '  -- the ordering test, and the thread count it runs at --'
command grep 'fn entries_come_back_in_the_order_recorded()' ring_trace/tests/trace_test.rs | sed 's/^/    /'
printf '    threads it spawns: %s\n' \
  "$( command grep -A 16 'fn entries_come_back_in_the_order_recorded()' ring_trace/tests/trace_test.rs | command grep -c 'spawn' || true )"
echo '  -- and what the concurrent test does before it compares --'
command grep -A 31 'fn concurrent_recorders_lose_no_entry()' ring_trace/tests/trace_test.rs \
  | command grep 'assert_eq\|sort_unstable' | sed 's/^/    /'
```

Live output:

```
  -- what the crate claims a trace answers that a counter cannot --
//! `ring_stats` counts; this records. A counter answers "how many publishes"
//! in constant space and tells you nothing about which sequences or in what
//! order; a trace answers "which operations, in what order" and grows without
//! bound — "order" here is the order producers reached the log's lock, which
  -- and the precise version, at the accessor --
  /// Every entry, in the order recorded.
  -- the ordering test, and the thread count it runs at --
    fn entries_come_back_in_the_order_recorded()
    threads it spawns: 0
  -- and what the concurrent test does before it compares --
      assert_eq!( trace.len(), THREADS * EACH );
      seqs.sort_unstable();
      assert_eq!( seqs, ( 0..( THREADS * EACH ) as u64 ).collect::< Vec< _ > >() );
```

## How Far the Two Orders Diverge

*This probe's inversion counts come from a one-off scratch binary under
`-tr_probe/`, run once and swept afterward per this project's convention
for temporary files — it cannot be re-run to reconfirm. TR23's
disposition below only edited a doc comment; `record`'s lock-acquisition
order is unchanged, so the figures below stand as a recorded measurement
rather than a live guarantee.*

```rust
// -tr_probe/src/bin/log_order.rs
// The real pattern: a shared atomic hands out sequences the way `ring_claim`
// would, each producer then records the claim it won. The log's order is
// lock-acquisition order; the seq column is claim order.
let seq = Seq( next.fetch_add( 1, Ordering::Relaxed ) );
trace.record( TraceOp::Claim, seq, 1 );

// afterwards, count adjacent pairs whose seq goes backwards
for pair in entries.windows( 2 )
{
  if pair[ 1 ].seq.0 < pair[ 0 ].seq.0 { inversions += 1; }
}
```

Two independent runs, `--release`, 20,000 records per producer:

```
  1 producer(s): 20000 entries, 0 out of order, worst backward jump 0
  4 producer(s): 80000 entries, 3580 out of order, worst backward jump 4654
  single-threaded seq column: [0, 1, 2, 3, 4, 5, 6, 7]

  1 producer(s): 20000 entries, 0 out of order, worst backward jump 0
  4 producer(s): 80000 entries, 4828 out of order, worst backward jump 2931
  single-threaded seq column: [0, 1, 2, 3, 4, 5, 6, 7]
```

---

### TR23 — "In What Order" Is Lock Order, and Under Four Producers They Differ 5% of the Time

The module doc's contrast with `ring_stats` is the crate's statement of purpose:
a counter "tells you nothing about which sequences or in what order; a trace
answers 'which operations, in what order'". A reader takes from that — reasonably,
because it is the only reason to pay for a log instead of a counter — that the
entries reconstruct the order the operations happened in.

They reconstruct the order the producers won the lock. Those are the same order
with one producer and not with more. Measured against a shared atomic issuing
sequences the way a real claim path would, four producers put 3,580 and 4,828 of
80,000 adjacent pairs out of sequence order across two runs — roughly one pair in
twenty — and the log steps backwards by as much as 4,654 sequences at the worst
point. One producer gives zero inversions in both runs.

The gap between claiming a sequence and recording it is the whole cause, and it
cannot be closed: `record` takes the sequence as an argument, so the claim has
already happened by the time the lock is contended for. Nothing about that is a
bug — a log of a concurrent system in real-time order is exactly what a lock
gives you, and the alternative would require a trace inside the claim's own
atomic.

**Finding.** The claim as written is contradicted by the crate's own concurrency
story, and the accessor already carries the correct version: `entries()` says
"every entry, in the order recorded", which is exact and load-bearing precisely
because "recorded" is not "happened". The module doc's sentence is the one that
overreaches, and it is the sentence a reader chooses the crate on. One clause —
that the order is the order producers reached the lock, which is the operation
order for a single producer and an interleaving for more — makes the crate's
purpose statement true without weakening it.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A2 -F 'is operation order for one producer' ring_trace/src/lib.rs
```

Live output:

```
//! is operation order for one producer and an interleaving for more, not
//! reconstructed sequence order under contention (→ `docs/invariant/002`).
//! Unbounded is not abstract: one producer recording as fast as it can has
```

**Disposition:** applied — the module doc's "Trace against stats" section now
states that "order" is lock-acquisition order, exact for one producer and an
interleaving for more, not reconstructed sequence order under contention —
the clause the finding asks for, pointing back at this doc. `cargo test
--release -p ring_trace --doc` confirms 9/9 doctests still pass. Now prints: `is operation order for one producer`

---

### TR24 — The Suite Already Knows, in One Uncommented Call

`entries_come_back_in_the_order_recorded` spawns no threads. It records four
operations from one thread and asserts the four come back in that order, with the
comment "order is the only thing a trace has that a counter does not" — the
strongest possible statement of the property, made in the only setting where it
is unconditionally true.

`concurrent_recorders_lose_no_entry` runs four threads at 2,000 records each and
asserts two things: that the count is exactly 8,000, and that the sequence column
— **sorted first** — is exactly `0..8000`. The `sort_unstable()` on line 299 is
the crate's only acknowledgement anywhere that the log's order is not the
sequence order. Without it the assertion would fail on almost every run, so the
author knew; the knowledge went into a method call and not into a sentence.

What the sorted assertion states is the invariant that actually holds under
contention, and it is a good one: every claimed sequence appears exactly once,
none lost, none duplicated, regardless of interleaving. That is worth having and
worth naming.

**Finding.** The suite is more honest than the documentation, which is the
diagnostic-shaped version of a familiar problem — the fact is preserved in a form
only someone reading the test body will find. Two lines fix it: a comment above
the `sort_unstable()` saying the log is in lock order so the multiset, not the
sequence, is what holds; and the same fact as a sentence at `entries()`, where a
caller about to interpret a log will meet it. The invariant already asserted
deserves the name it does not have.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/001`](001_disabled_means_zero_forever.md) | The invariant that holds without qualification |
| [`decisions/002`](../decisions/002_a_mutex_over_a_lock_free_log.md) | The lock whose acquisition order this is |
| [`data_structure/002`](../data_structure/002_three_public_fields_and_the_range_they_imply.md) | The `seq` field the inversions are counted on |
| [`pitfall/001`](../pitfall/001_a_range_that_reads_backwards.md) | The other way an entry's numbers mislead |

### Sources

| Fact | Where |
|------|-------|
| The "which operations, in what order" claim | `ring_trace/src/lib.rs:16-19` |
| The accurate wording at the accessor | `ring_trace/src/lib.rs:299` |
| The ordering test spawning no threads | `ring_trace/tests/trace_test.rs:108-124` |
| The sort before the comparison | `ring_trace/tests/trace_test.rs:305` |
| 3,580 and 4,828 inversions in 80,000 | Probe above, two runs |
| Worst backward jump 4,654 and 2,931 | Probe above, two runs |

### Tests

| Test | Covers |
|------|--------|
| `entries_come_back_in_the_order_recorded` | The order, single-threaded, where it is exact |
| `concurrent_recorders_lose_no_entry` | The multiset, which is what survives contention |
| `a_disabled_trace_stays_empty_under_contention` | The same four-thread shape with nothing recorded |
