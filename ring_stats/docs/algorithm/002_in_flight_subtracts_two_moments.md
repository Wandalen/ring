# Algorithm: `in_flight` Subtracts One Moment From Another

### Scope

**Purpose:** Record what `in_flight` computes, which direction its seam fails in,
and why that direction is the one without a tell.

**Responsibility:** The two loads, their order, the `saturating_sub` between them,
and every assertion made about the result.

**In Scope:** `RingStats::in_flight` and its doc block in
`ring_stats/src/lib.rs`; the three single-threaded `in_flight` tests and
`counts_are_exact_under_contention` in `ring_stats/tests/stats_test.rs`;
the 240-on-16 write-up in `ring_bench/tests/bench_test.rs`.

**Out of Scope:** The caller-facing consequence — a monitor that reads zero and
concludes healthy — is
[`pitfall/001`](../pitfall/001_the_leak_in_flight_cannot_see.md). The other two
compositions are [`algorithm/001`](001_eleven_operations_and_three_compositions.md)
§ ST2.

---

## Two Loads, One Subtraction, and Who Reads It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the composition, and the claim above it --'
command grep -m1 -A1 -F '  /// Slots claimed but not yet published — a nonzero reading here at rest means' ring_stats/src/lib.rs
command grep -m1 -A4 -F '  pub fn in_flight( &self ) -> u64' ring_stats/src/lib.rs
echo '  -- and what the doc now says about the direction of the error --'
command grep -m1 -A4 -F '  /// **Two loads at two moments, and the error has a direction.** `claimed` is' ring_stats/src/lib.rs
echo '  -- where it is called, checked_in_flight and doc comments excluded --'
printf '    production call sites  : %s\n' \
  "$( awk '/[^_]in_flight\(\)/ && !/\/\/\// { n++ } END { print n + 0 }' ring_*/src/*.rs )"
printf "    ring_stats' own tests  : %s\n" \
  "$( awk '/[^_]in_flight\(\)/ && !/\/\/\// { n++ } END { print n + 0 }' ring_stats/tests/*.rs )"
printf '    every other test file  : %s\n' \
  "$( awk '/[^_]in_flight\(\)/ && !/\/\/\// && FILENAME !~ /ring_stats/ { n++ } END { print n + 0 }' ring_*/tests/*.rs )"
echo '  -- and the one assertion made under contention --'
command grep -m1 -A3 -F '  assert_eq!( stats.claimed() as usize, PRODUCERS * PER_PRODUCER );' ring_stats/tests/stats_test.rs
```

Live output:

```
  -- the composition, and the claim above it --
  /// Slots claimed but not yet published — a nonzero reading here at rest means
  /// a producer took a slot and abandoned it, which is a leak of ring capacity.
  pub fn in_flight( &self ) -> u64
  {
    self.claimed().saturating_sub( self.published() )
  }

  -- and what the doc now says about the direction of the error --
  /// **Two loads at two moments, and the error has a direction.** `claimed` is
  /// read first and `published` second, and both only ever climb, so a publish
  /// landing between the two is subtracted from a `claimed` that predates it:
  /// the result comes out at or below the truth and never above it.
  /// `saturating_sub` then floors that error at zero — which is also the reading
  -- where it is called, checked_in_flight and doc comments excluded --
    production call sites  : 0
    ring_stats' own tests  : 14
    every other test file  : 2
  -- and the one assertion made under contention --
  assert_eq!( stats.claimed() as usize, PRODUCERS * PER_PRODUCER );
  assert_eq!( stats.published() as usize, PRODUCERS * PER_PRODUCER );
  assert_eq!( stats.in_flight(), 0, "every claim was published" );
}
```

---

### ST3 — The Seam Can Only Subtract, Because the Later Load Is the One That Grows

`claimed` is loaded first and `published` second. Both counters climb and neither
ever falls except through `reset`. So any publish landing between the two loads is
subtracted from a `claimed` that predates it, and the result is understated by
exactly the traffic that passed through the window.

Formally: for reads at `t₁ < t₂`, the returned value is
`claimed( t₁ ) − published( t₂ )`, and since `published( t₂ ) ≥ published( t₁ )`,
that is at most `claimed( t₁ ) − published( t₁ )`. The error is one-directional. The
method can report fewer slots in flight than there are; it can never report more.

Then `saturating_sub` removes the only remaining signal. Without it, an interleaving
that read `published` far ahead of `claimed` would underflow, and a `u64` near
`u64::MAX` is a value no reader could mistake for healthy. With it, the same
interleaving returns `0` — which is not merely plausible but is the *expected*
value for a ring behaving correctly.

**Finding.** Hold a real leak fixed and put ordinary matched traffic beside it. Eight
slots claimed and never published, one producer doing balanced claim-then-publish,
two million readings:

```
  -- 8 slots leaked, plus one matched claim/publish producer --
    in_flight() readings taken            2000000
    reporting fewer than 8 in flight      13099
    reporting 0 — no leak at all          27
    lowest reading                        0
    highest reading                       9
  -- with the churn stopped, the leak is still there --
    claimed()                             190086
    published()                           190078
    in_flight()                           8
```

A second run:

```
  -- 8 slots leaked, plus one matched claim/publish producer --
    in_flight() readings taken            2000000
    reporting fewer than 8 in flight      11267
    reporting 0 — no leak at all          5
    lowest reading                        0
    highest reading                       9
  -- with the churn stopped, the leak is still there --
    claimed()                             359947
    published()                           359939
    in_flight()                           8
```

About one reading in 170 understates the leak, and a handful per two million erase
it entirely — while the counters, read at rest a moment later, hold the leak
perfectly. The masking is proportional to traffic: the busier the ring, the more
publishes fall inside the window, and the harder a genuine leak is to see. That is
the inverse of what a diagnostic should do.

The highest reading of 9 is the churn producer's own open claim added to the eight
— a truthful reading, and the reason the range is `0..=9` rather than `0..=8`.

The code is unchanged and correct: there is no ordering of two independent loads
that fixes this ([`pitfall/001`](../pitfall/001_the_leak_in_flight_cannot_see.md)
§ ST42 measures the alternative and it is worse). What was missing was that the
method said none of it. `in_flight`'s doc scoped its claim carefully — "a nonzero
reading **here at rest**" — and left the converse unwritten, so a reader had the
half that carries information and not the half that does not. The doc now carries
both directions, the measured frequency, and the fact that reversing the loads is
not the repair; and [`RingStats::snapshot`] is the reading to take when the
numbers have to agree with each other.

**Disposition:** applied — `in_flight`'s doc extended with the direction of the
error (understates, never overstates), the floor `saturating_sub` puts under it,
the measured rate against a permanent eight-slot leak, and a pointer to
`RingStats::snapshot` and `StatsCounts::checked_in_flight`. The
`*(to create)*` row below is closed by
`a_snapshot_agrees_with_itself_while_writers_run`, which reads a derived value
beside four writer threads for 50,000 samples — proven able to fail rather than
assumed: making `snapshot` re-load through `self.dropped_total()` and
`self.in_flight()` instead of deriving from its own locals turns it red at
`stats_test.rs:417` with a torn reading of
`dropped_newest: 8442, dropped_oldest: 6603, failed: 1771` against a total read a
moment later. Now prints: `  /// the result comes out at or below the truth and never above it.`

---

### ST4 — The Direction That Was Caught Is the Direction This Cannot Produce

The family already has one recorded case of `in_flight` returning an impossible
number, and it is written up in `ring_bench`'s own test file:

> `claimed` and `published` are *slot* lifecycle counters — `RingStats::in_flight()`
> is defined as `claimed - published`, so feeding them workload totals
> (`claimed = offered`, `published = accepted`) made 240 refused records read as 240
> leaked slots on a ring that had only 16.

That bug was found, diagnosed and fixed, and the reason is in the sentence: **240 on
a ring of 16**. The reading exceeded a bound the reader knew, so it announced itself.
The fix is recorded in a nine-line comment above the test.

**Finding.** Every mechanism that made that bug findable is absent from the
under-reporting direction. There is no upper bound to violate, because the wrong
answer is *smaller*; there is no impossible value, because `saturating_sub` floors
it at zero; and zero is what a correct ring reports, so it triggers no second look.
The one bug the family caught was caught by luck of sign.

The suite reflects the same asymmetry. Fourteen of the sixteen call sites are in
`ring_stats`' own tests, two in `ring_bench`, and none in production code anywhere
in the 33 crates. The only assertion made under contention —
`counts_are_exact_under_contention` — spawns four producers, joins them all, and
*then* reads:

```
  assert_eq!( stats.claimed() as usize, PRODUCERS * PER_PRODUCER );
  assert_eq!( stats.published() as usize, PRODUCERS * PER_PRODUCER );
  assert_eq!( stats.in_flight(), 0, "every claim was published" );
```

Every value it checks is read from a quiesced set, which is the one condition in
which the seam cannot appear. So the method's one concurrency test asserts the
property that holds, in the state where it always holds, and the state a monitor
would actually read it in is untested — the same shape as `ring_atomic`'s
`counts_are_exact_under_contention`
([`ring_atomic` § AT44](../../../ring_atomic/docs/pitfall/001_the_snapshot_that_never_happened.md)),
down to the test's name.

The cheap correction is not to the code. `in_flight`'s doc already scopes its claim
correctly — "a nonzero reading **here at rest**" — and the missing half is the
converse: a zero reading under traffic means nothing at all.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_stats
command grep -F 'reading taken under traffic is evidence of nothing' src/lib.rs
```

Live output:

```
    /// reading taken under traffic is evidence of nothing. Held against a
```

**Disposition:** applied — closed by the same doc edit ST3 records: `in_flight`'s
comment now states the converse this finding names explicitly — "a *zero*
reading taken under traffic is evidence of nothing" — immediately beside the
understatement direction, so the missing half ST4 identifies is no longer
missing.
Now prints: `reading taken under traffic is evidence of nothing`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/001`](../pitfall/001_the_leak_in_flight_cannot_see.md) | The same seam as a caller reaches it, and the four artifacts that read as reassurance |
| [`algorithm/001`](001_eleven_operations_and_three_compositions.md) | The other two compositions, and the ten methods that are one instruction |
| [`invariant/002`](../invariant/002_claimed_never_trails_published.md) | The ordering `in_flight` assumes and nothing enforces |
| [`item/002`](../item/002_the_two_methods_that_are_not_one_operation.md) | `in_flight` and `reset` read as contracts |

### Sources

| Fact | Where |
|------|-------|
| The two loads and the saturation | Census above, `RingStats::in_flight`'s body |
| The doc's "at rest" scoping, and the direction now stated beside it | Census above |
| 0 production call sites, 15 in tests | Census above |
| A leak masked under traffic | Probe, two runs quoted above |
| The 240-on-16 incident and its fix | `ring_bench`'s `the_counters_are_the_runs_own_totals` |
| The contention assertion reading after join | `counts_are_exact_under_contention` |

### Tests

| Test | Covers |
|------|--------|
| `in_flight_is_claimed_minus_published` | The definition, on one thread |
| `in_flight_saturates_rather_than_wrapping` | That the floor is 0 — the behaviour that removes the tell |
| `consuming_does_not_affect_in_flight` | That consumption cannot close an open claim |
| `counts_are_exact_under_contention` | `in_flight() == 0`, read after every producer has joined |
| `ring_bench`'s `the_counters_are_the_runs_own_totals` | `ring_bench`'s mapping, fixed after the 240-on-16 reading |
| `a_snapshot_agrees_with_itself_while_writers_run` | A derived reading taken beside four live writers — the condition the seam appears in |
