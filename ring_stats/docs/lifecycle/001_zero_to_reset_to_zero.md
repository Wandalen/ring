# Lifecycle: Zero, Counting, Reset, Zero — and the Moment in Between

### Scope

**Purpose:** Record the states a `RingStats` passes through, and measure the one
transition that is observable from outside while it is happening.

**Responsibility:** `new`, the counting phase, `reset`, and what a concurrent reader
sees during the reset.

**In Scope:** `RingStats::new`, `RingStats::counters` and `RingStats::reset` in
`ring_stats/src/lib.rs`; `reset_returns_every_counter_to_the_fresh_state`,
`a_reset_set_counts_again` and
`a_reader_beside_a_reset_sees_only_values_the_writer_wrote` in
`ring_stats/tests/stats_test.rs`.

**Out of Scope:** Why `reset` cannot be atomic is
[`api/001`](../api/001_fourteen_methods_and_no_exclusive_borrow.md) § ST5; the same
two methods as items are
[`item/002`](../item/002_the_two_methods_that_are_not_one_operation.md) § ST28. Who
calls `reset` — nobody outside this crate — is
[`integration/001`](../integration/001_the_write_path_and_two_callers_that_are_not_there.md)
§ ST18.

---

## The Whole Life of a Counter Set

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the field order reset walks, first to last --'
command grep -m1 -A6 -F '  const fn counters( &self ) -> [ &AtomicU64; Self::COUNTERS ]' ring_stats/src/lib.rs
echo '  -- and the loop that stores through it --'
command grep -m1 -A6 -F '  pub fn reset( &self )' ring_stats/src/lib.rs
echo '  -- what the doc now says about the transition, not just the postcondition --'
command grep -m1 -A3 -F '  /// **Not atomic, and the partial state is identifiable.** The counters are' ring_stats/src/lib.rs
echo '  -- every test that observes a reset --'
command grep 'fn reset_returns_every_counter_to_the_fresh_state\|fn a_reset_set_counts_again\|fn a_reader_beside_a_reset_sees_only_values_the_writer_wrote' ring_stats/tests/stats_test.rs
echo '  -- and how many lines of each spawn a second thread --'
for t in reset_returns_every_counter_to_the_fresh_state a_reset_set_counts_again a_reader_beside_a_reset_sees_only_values_the_writer_wrote; do
  printf '    %-56s %s\n' "$t" \
    "$( awk -v t="fn $t()" 'index( $0, t ) { on = 1 } on && /^}$/ { exit } on' ring_stats/tests/stats_test.rs | command grep -c 'spawn\|thread::scope' )"
done
```

Live output:

```
  -- the field order reset walks, first to last --
  const fn counters( &self ) -> [ &AtomicU64; Self::COUNTERS ]
  {
    [
      &self.claimed, &self.published, &self.consumed,
      &self.dropped_newest, &self.dropped_oldest, &self.failed, &self.wait_nanos,
    ]
  }
  -- and the loop that stores through it --
  pub fn reset( &self )
  {
    for counter in self.counters()
    {
      counter.store( 0, Ordering::Relaxed );
    }
  }
  -- what the doc now says about the transition, not just the postcondition --
  /// **Not atomic, and the partial state is identifiable.** The counters are
  /// stored in declaration order through a shared reference —
  /// [`RingStats::COUNTERS`] separate stores, `claimed` first and `wait_nanos`
  /// last — so a concurrent reader can land between any two of them. A fill runs
  -- every test that observes a reset --
fn reset_returns_every_counter_to_the_fresh_state()
fn a_reset_set_counts_again()
fn a_reader_beside_a_reset_sees_only_values_the_writer_wrote()
  -- and how many lines of each spawn a second thread --
    reset_returns_every_counter_to_the_fresh_state           0
    a_reset_set_counts_again                                 0
    a_reader_beside_a_reset_sees_only_values_the_writer_wrote 3
```

---

### ST29 — The Reset Window Is Observable, and Identifiable by Its Signature

`reset` walks the seven counters in a fixed order — `claimed` first, `wait_nanos`
last — storing zero into each. A reader running concurrently can land between any two
of those stores.

That window is detectable without ambiguity, because the two seams have opposite
signatures. A writer filling the counters in the *same* field order sets `claimed`
before `wait_nanos`, so a read caught mid-fill returns `( set, 0 )`. A reset clears
`claimed` before `wait_nanos`, so a read caught mid-reset returns `( 0, set )` — a
combination no fill can produce and no complete state can hold.

**Finding.** One writer looping fill-then-reset, one reader taking two million
`( claimed, wait_nanos )` pairs:

```
  -- one writer looping: fill all seven, then reset() all seven --
    ( claimed, wait_nanos ) pairs read     2000000
    caught inside reset()  ( 0, set )      101
    caught inside the fill ( set, 0 )      1538233
```

A second run:

```
  -- one writer looping: fill all seven, then reset() all seven --
    ( claimed, wait_nanos ) pairs read     2000000
    caught inside reset()  ( 0, set )      5
    caught inside the fill ( set, 0 )      1951579
```

Six consecutive runs landed at 101, 5, 69, 30, 16 and 16 — always present, never
predictable. The reset window is much narrower than the fill window, which is what
the third line shows: seven stores in a tight loop against seven `fetch_add`s
interleaved with a reader, so the great majority of samples land in the fill. That
asymmetry is the point. The state that is merely incomplete is easy to hit; the state
that is *impossible* — some counters zeroed, others not, no recording having occurred
between them — is rare enough to escape any casual test and common enough to happen.

`reset`'s doc called this "Reset every counter to zero", which describes the
postcondition accurately and said nothing about the transition. Its stated purpose —
so a recycled ring "does not carry the previous world's numbers" — is exactly the case
where a reader might still be sampling: a shutdown in progress, a monitor not yet told
to stop. Those two sentences sat one above the other, and between them was the whole
of what a concurrent reader could see.

**Disposition:** applied — `RingStats::reset`'s doc now states that the operation is
`RingStats::COUNTERS` separate stores rather than one, names the `( 0, set )`
signature as the reading no fill and no complete state can produce, and carries the
measured 5-to-101-per-two-million rate from the six runs above. The postcondition
sentence is unchanged and still correct; what is new is the sentence beneath it about
the transition, placed directly above the recycling purpose that creates the exposure.
Now prints: `  /// **Not atomic, and the partial state is identifiable.** The counters are`

---

### ST30 — Nothing Observed a Reset From a Second Thread

The suite tested `reset` twice. `reset_returns_every_counter_to_the_fresh_state`
records into every counter, resets, and asserts all seven are zero.
`a_reset_set_counts_again` resets and then records again, asserting the set is not
poisoned. Both are correct and both matter.

Neither spawns a thread. Across both tests, the thread count was zero.

**Finding.** That is the same shape the corpus finds at every other seam in this
crate — `counts_are_exact_under_contention` reads after joining
([`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) § ST4),
`each_counter_is_monotone_while_writers_run` samples one counter with no composition
([`invariant/001`](../invariant/001_monotone_per_counter_and_only_per_counter.md)
§ ST21) — and here it was sharpest,
because `reset`'s window is the one seam that produces a *structurally impossible*
reading rather than merely a stale one.

The gap was cheap to close and did not require asserting anything about the window.
`a_reader_beside_a_reset_sees_only_values_the_writer_wrote` runs a reader beside a
writer looping fill-then-reset for 20,000 rounds, and on every read asserts the
weaker, always-true property: that `claimed` and `wait_nanos` each hold either `0` or
the fill value, never something neither `new`, the fill, nor the reset put there. It
records whether it caught the `( 0, set )` window and deliberately does not assert
that it did — the measured rate is 5 to 101 per two million paired reads, far too
rare to require without making the suite flaky. Asserting the window is *impossible*
would fail, and should, because it is not.

The two original tests stay single-threaded, which is correct: they check the
postcondition, and the postcondition is a single-threaded property. What was missing
was any test at all in which the transition could be seen, and that is what the third
one supplies.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/002`](002_implemented_tested_and_still_planned.md) | The crate's own lifecycle, against the feature's recorded status |
| [`item/002`](../item/002_the_two_methods_that_are_not_one_operation.md) | `new` and `reset` as two routes to the same state |
| [`api/001`](../api/001_fourteen_methods_and_no_exclusive_borrow.md) | Why the shared reference forces seven stores |
| [`invariant/001`](../invariant/001_monotone_per_counter_and_only_per_counter.md) | Monotonicity, and `reset` as its only exception |

### Sources

| Fact | Where |
|------|-------|
| `reset`'s field order, and the loop that walks it | Census above |
| The window measured, six runs | Probe, two runs quoted above |
| The two original reset tests are single-threaded | Census above |
| The third test runs a reader beside the writer | Census above |
| `reset`'s doc, postcondition then transition | Census above |
| Its stated purpose, recycling a ring | `RingStats::reset`'s doc |

### Tests

| Test | Covers |
|------|--------|
| `reset_returns_every_counter_to_the_fresh_state` | All seven counters zero afterwards, one thread |
| `a_reset_set_counts_again` | That a reset set still records |
| `a_fresh_set_is_all_zero` | The same postcondition via `new` |
| `a_reader_beside_a_reset_sees_only_values_the_writer_wrote` | A reader running through 20,000 fill-then-reset rounds — the only condition in which the window appears |
