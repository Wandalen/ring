# Workaround: A Floor That Absorbs More Than It Was Built For

### Scope

**Purpose:** Record the crate's one defensive construct — `saturating_sub` in
`in_flight` — the constraint it works around, and measure what it actually absorbs.

**Responsibility:** The floor inside `in_flight`, the caller-bug rationale the suite
gives for it, how often it fires on a ring with no caller bug at all, and the checked
alternative now shipped beside it.

**In Scope:** `RingStats::in_flight` and `StatsCounts::checked_in_flight` in
`ring_stats/src/lib.rs`; `in_flight_saturates_rather_than_wrapping` in
`ring_stats/tests/stats_test.rs`.

**Out of Scope:** That the floored value is also the healthy value, and what that does
to leak detection, is
[`pitfall/001`](../pitfall/001_the_leak_in_flight_cannot_see.md). The invariant it is
defending is
[`invariant/002`](../invariant/002_claimed_never_trails_published.md).

---

## The Constraint and the Guard

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the workaround, and the reading it prevents --'
command grep -m1 -F '    self.claimed().saturating_sub( self.published() )' ring_stats/src/lib.rs
echo '  -- the rationale, as the suite states it --'
command grep -m1 -A3 -F '/// In-flight floors at zero rather than wrapping. Publishing more than was' ring_stats/tests/stats_test.rs
echo '  -- and what that test actually constructs --'
command grep -m1 -A10 -F 'fn in_flight_saturates_rather_than_wrapping()' ring_stats/tests/stats_test.rs
echo '  -- every saturating, checked or wrapping operation in the crate, doc comments excluded --'
command grep 'saturating_\|checked_\|wrapping_' ring_stats/src/lib.rs | command grep -v '^\s*///'
echo '  -- and the alternative the guard did not take, now shipped beside it --'
command grep -m1 -B20 -A5 -F '  pub const fn checked_in_flight( &self ) -> Option< u64 >' ring_stats/src/lib.rs
```

Live output:

```
  -- the workaround, and the reading it prevents --
    self.claimed().saturating_sub( self.published() )
  -- the rationale, as the suite states it --
/// In-flight floors at zero rather than wrapping. Publishing more than was
/// claimed is a caller bug, and `u64` subtraction would turn it into an
/// 18-quintillion-slot reading that looks like catastrophic leakage. The floor
/// is not only a backstop for that bug: on a busy, correct ring it also
  -- and what that test actually constructs --
fn in_flight_saturates_rather_than_wrapping()
{
  let stats = RingStats::new();
  stats.record_claim( 2 );
  stats.record_publish( 5 );
  assert_eq!( stats.in_flight(), 0 );

  let never_claimed = RingStats::new();
  never_claimed.record_publish( 1 );
  assert_eq!( never_claimed.in_flight(), 0 );
}
  -- every saturating, checked or wrapping operation in the crate, doc comments excluded --
  pub const fn checked_in_flight( &self ) -> Option< u64 >
    self.claimed.checked_sub( self.published )
    OverflowPolicy::ALL.iter().map( | p | self.dropped( *p ) ).fold( 0, u64::saturating_add )
    self.claimed().saturating_sub( self.published() )
      dropped_total : dropped_newest.saturating_add( dropped_oldest ).saturating_add( failed ),
      in_flight : claimed.saturating_sub( published ),
  -- and the alternative the guard did not take, now shipped beside it --
{
  /// In flight, with the caller bug kept apart from a balanced ring.
  ///
  /// `None` means `published` exceeds `claimed` — which the counters permit,
  /// which no correct caller produces, and which `in_flight` floors to zero, the
  /// same value a healthy ring gives. `checked_sub` is one word different from
  /// `saturating_sub` and is the only reading in the crate that tells the two
  /// apart.
  ///
  /// ```
  /// use ring_stats::RingStats;
  ///
  /// let s = RingStats::new();
  /// s.record_claim( 2 );
  /// s.record_publish( 5 );
  ///
  /// assert_eq!( s.snapshot().in_flight, 0 );
  /// assert_eq!( s.snapshot().checked_in_flight(), None );
  /// ```
  #[ must_use ]
  pub const fn checked_in_flight( &self ) -> Option< u64 >
  {
    self.claimed.checked_sub( self.published )
  }
}
```

---

### ST49 — The Guard Fires Constantly on a Ring With No Caller Bug

The constraint is ordinary: `u64` has no negative values, so `claimed - published`
wraps to something near eighteen quintillion the moment `published` exceeds `claimed`.
The workaround is `saturating_sub`, which was at the time the crate's only saturating,
checked or wrapping operation anywhere. There are three now, and all three are this
same subtraction: `in_flight`'s floor, the identical floor inside `RingStats::snapshot`,
and the `checked_sub` in `StatsCounts::checked_in_flight` — no other arithmetic in the
crate guards against anything.

**Correction (2026-09-28):** this paragraph read "There are three now, and all
three are this same subtraction … no other arithmetic in the crate guards
against anything." `Fix(ring_stats_dropped_total_overflow)` (see
[`pitfall/002`](../pitfall/002_the_total_that_counts_a_refusal_as_a_loss.md))
gave `dropped_total` its own overflow guard — `fold( 0, u64::saturating_add )` —
and `RingStats::snapshot` the identical guard on its own copy of that field. The
census above now shows four guarded sites, not three, and two of them —
`dropped_total`'s fold and `snapshot`'s matching field — guard an addition
against overflow rather than a subtraction against underflow. This paragraph's
own subject, the read-order race in `in_flight` and `checked_in_flight`, is
untouched by that fix; only the "three, all the same subtraction, nothing else
guarded" count is no longer accurate.

The suite says what it is for: "Publishing more than was claimed is a caller bug, and
`u64` subtraction would turn it into an 18-quintillion-slot reading that looks like
catastrophic leakage." The test builds that bug twice by hand — record two claims and
five publishes, then publish with nothing claimed — and asserts zero both times, on one
thread.

**Finding.** Drive a ring where no caller bug exists at all — every producer claims
before it publishes, always — and read the two counters in `in_flight`'s own order.
Two million reads at each of three producer counts, twice:

```
  -- every producer claims before it publishes; no caller bug anywhere --

    producers   reads      floored at 0 by saturating_sub   widest underflow
            1   2000000                            25695   3770
            3   2000000                            36031   22053
            6   2000000                            44694   5609
```

```
  -- every producer claims before it publishes; no caller bug anywhere --

    producers   reads      floored at 0 by saturating_sub   widest underflow
            1   2000000                            34996   36141
            3   2000000                            25887   5133
            6   2000000                            22678   1144
```

One to two percent of readings would underflow, by as much as 36,141, and not one of
them is the condition the guard is documented against. `claimed` is loaded first and
`published` second, so a producer running between the two loads can publish past the
`claimed` value already in hand — the ordinary, correct behaviour of a correct ring
([`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) § ST3).

So the floor is not a rare backstop against misuse. It is load-bearing on every busy
ring, absorbing tens of thousands of readings per two million that the read order
itself produced. Remove it and a healthy ring reports catastrophic leakage one time in
fifty; keep it and those readings become zero — which the rest of the API treats as
health.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_stats
command grep -F 'absorbs the ordinary read-order race between the two loads, one to two' tests/stats_test.rs
```

Live output:

```
/// absorbs the ordinary read-order race between the two loads, one to two
```

**Disposition:** applied — the test's doc comment no longer scopes the floor to
only the caller-bug case it builds by hand; it now states, beside that case,
that the same floor absorbs the ordinary read-order race on a busy correct ring
one to two percent of the time, and that without it a healthy ring would report
the identical catastrophic reading — matching what this finding measures.
Now prints: `absorbs the ordinary read-order race between the two loads, one to two`

---

### ST50 — The Workaround Erases the Distinction It Was Introduced to Preserve

The rationale is sound and the choice of `saturating_sub` over a bare subtraction is
clearly right — eighteen quintillion is worse than zero by any measure. What the
workaround gives up is the ability to tell the two apart afterwards.

Three distinct conditions all arrive at the caller as `0`:

- The ring is healthy and nothing is outstanding. The common case, and correct.
- The read was taken across a seam and the true value is positive. A miss, measured at
  seven to sixteen readings per two million against a permanent leak
  ([`pitfall/001`](../pitfall/001_the_leak_in_flight_cannot_see.md) § ST41).
- `published` genuinely exceeded `claimed`. Impossible without a caller bug at rest,
  and produced by the read order one to two percent of the time in flight, per the
  measurement above.

**Finding.** `checked_sub` returns `Option< u64 >` and distinguishes the third from the
other two for the cost of one word at the definition and a match at each call site. It
would leave `None` meaning "these two loads cannot both be true", which is exactly the
signal a diagnostic wants — and it would have made the third row above visible instead
of silently folded into the first.

The crate did not take it, and nothing recorded that the choice had been made. The
suite's own comment reasons only about the wrap, which is the alternative
`saturating_sub` beats; it does not reason about `checked_sub`, which is the
alternative it does not. This is the shape the corpus keeps finding in `ring_stats`: a
real cost weighed carefully against the wrong comparison
([`decisions/001`](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md)
§ ST14).

`StatsCounts::checked_in_flight` is that one word, and the census above shows it: the
same subtraction, `checked_sub` instead of `saturating_sub`, returning `None` on the
third condition and `Some( n )` on the other two. Its own doc says which two it does
*not* separate, and its doctest asserts both halves of the pair — `in_flight` reading
`0` and `checked_in_flight` reading `None` on the same value.

`RingStats::in_flight` is unchanged, and deliberately: it is the reading a monitor
samples, `Option< u64 >` at that call site is friction for a case that never fires on a
correct caller, and every existing caller reads a `u64`. So the distinction is now
*available* rather than *enforced* — the second and third conditions above still arrive
as `0` on the route almost everyone takes, and only a caller that already holds a
snapshot can ask which one it was. That is a smaller claim than the finding wanted, and
it is the honest one.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/002`](002_seven_counters_enumerated_four_times_by_hand.md) | The crate's other workaround, and the check the family applies to it elsewhere |
| [`pitfall/001`](../pitfall/001_the_leak_in_flight_cannot_see.md) | What a floored reading costs a leak detector |
| [`invariant/002`](../invariant/002_claimed_never_trails_published.md) | The invariant the floor defends, and where it actually holds |
| [`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) | Why the read order produces the underflow |

### Sources

| Fact | Where |
|------|-------|
| Every saturating or checked operation in the crate | Census above |
| The caller-bug rationale | Census above |
| The two hand-built caller bugs it tests | Census above |
| The checked alternative, and what its doc claims | Census above |
| Underflow frequency on a correct ring, two runs | Probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `in_flight_saturates_rather_than_wrapping` | Both caller-bug shapes, single-threaded |
| `in_flight_is_claimed_minus_published` | The subtraction where the floor never engages |
| `counts_are_exact_under_contention` | Concurrent recording, read after joining |
| `a_snapshot_agrees_with_itself_while_writers_run` | A derived reading taken while producers run — the condition under which the floor actually engages |
| `checked_in_flight_tells_a_caller_bug_from_a_balanced_ring` | All three conditions side by side: `0` from a balanced ring, `0` from the caller bug, and `Some( 0 )` against `None` telling them apart |
| `StatsCounts::checked_in_flight`'s own doctest | That the two readings disagree on exactly the third condition: `0` against `None` |
