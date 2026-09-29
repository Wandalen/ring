# Pitfall: The Leak `in_flight` Cannot See

### Scope

**Purpose:** Record that the crate's leak detector fails by reporting health, measure
how often, and show — by measurement — that the obvious one-line repair does not work.

**Responsibility:** `in_flight`'s two loads, the direction of its error, the floor
`saturating_sub` puts under it, and both orderings of the same subtraction.

**In Scope:** `RingStats::in_flight` and its doc block in
`ring_stats/src/lib.rs`; the three `in_flight` tests and
`a_snapshot_agrees_with_itself_while_writers_run` in
`ring_stats/tests/stats_test.rs`.

**Out of Scope:** What `in_flight` executes and why the error is one-directional is
[`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md). What a zero
reading establishes about ordering is
[`invariant/002`](../invariant/002_claimed_never_trails_published.md).

---

## The Trap

A leak detector should fail loudly. This one fails silently, and it fails into the
exact value that means "nothing is wrong".

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the crate says a nonzero reading means --'
command grep -m1 -A1 -F '  /// Slots claimed but not yet published — a nonzero reading here at rest means' ring_stats/src/lib.rs
command grep -m1 -A4 -F '  pub fn in_flight( &self ) -> u64' ring_stats/src/lib.rs
echo '  -- and what it now says about a reading of zero --'
command grep -m1 -A5 -F '  /// `saturating_sub` then floors that error at zero — which is also the reading' ring_stats/src/lib.rs
echo '  -- every test that touches in_flight or a derived reading, and the threads each spawns --'
for t in in_flight_is_claimed_minus_published in_flight_saturates_rather_than_wrapping consuming_does_not_affect_in_flight a_snapshot_agrees_with_itself_while_writers_run; do
  printf '    %-48s %s\n' "$t" \
    "$( awk -v t="fn $t()" 'index( $0, t ) { on = 1 } on && /^}$/ { exit } on' ring_stats/tests/stats_test.rs | command grep -c 'spawn\|thread::scope' )"
done
```

Live output:

```
  -- what the crate says a nonzero reading means --
  /// Slots claimed but not yet published — a nonzero reading here at rest means
  /// a producer took a slot and abandoned it, which is a leak of ring capacity.
  pub fn in_flight( &self ) -> u64
  {
    self.claimed().saturating_sub( self.published() )
  }

  -- and what it now says about a reading of zero --
  /// `saturating_sub` then floors that error at zero — which is also the reading
  /// a healthy ring gives. So a *nonzero* reading is evidence, and a *zero*
  /// reading taken under traffic is evidence of nothing. Held against a
  /// permanent eight-slot leak with one matched producer beside it, about one
  /// reading in a hundred understated the leak and a handful per two million
  /// reported no leak at all.
  -- every test that touches in_flight or a derived reading, and the threads each spawns --
    in_flight_is_claimed_minus_published             0
    in_flight_saturates_rather_than_wrapping         0
    consuming_does_not_affect_in_flight              0
    a_snapshot_agrees_with_itself_while_writers_run  4
```

---

### ST41 — The Detector's Failure Mode Is a Reading of Zero

`claimed` is loaded first, `published` second. Both only climb, so every publish that
lands between the two loads is subtracted from a `claimed` that predates it, and the
result comes out at or below the truth — never above it. `saturating_sub` then puts a
floor under the error at zero.

Zero is the reading a healthy ring gives.

**Finding.** Hold a real, permanent leak of eight slots and run one matched
claim-then-publish producer beside it, so a truthful `in_flight()` can never read below
eight. Two million readings, twice:

```
  -- matched churn throughout, 2000000 readings per row --

    leaked   load order         understates   reads 0   overstates   range
         8   claimed first           25007        16            0   0..9
         8   published first             0         0        61306   8..664
         0   claimed first               0   1130519            0   0..1
         0   published first             0   1360410        57665   0..832
```

```
    leaked   load order         understates   reads 0   overstates   range
         8   claimed first           20687         7            0   0..9
         8   published first             0         0        58373   8..468
         0   claimed first               0   1074275            0   0..1
         0   published first             0    968946        60107   0..714
```

Read the first row of each. About one reading in a hundred understates the leak, and
seven to sixteen readings in two million report exactly zero — the crate's own
documented signal for *no leak at all*, returned while eight slots are permanently
outstanding.

The frequency is what makes it a pitfall rather than a caveat. A monitor sampling once
a second would take four hundred years to hit that; a monitor sampling in a tight loop
hits it in seconds. Neither is wrong to sample the way it does, and the reading gave
no indication which case it was in. Every one of those zeros is indistinguishable from
a correct reading of a healthy ring, because it *is* the reading of a healthy ring.

The subtraction cannot be repaired — § ST42 measures the only other ordering and it is
worse. What could be repaired is that the method said nothing about the half of its
own contract that carries no information.

**Disposition:** applied — `RingStats::in_flight`'s doc now states the direction of
the error, that `saturating_sub` floors it onto the healthy reading, and that a *zero*
taken under traffic is therefore evidence of nothing, with the measured rate from the
runs above beside it. `RingStats::snapshot` and `StatsCounts::checked_in_flight` are
named as the two readings to take when self-consistency or the caller-bug case matters,
and `a_snapshot_agrees_with_itself_while_writers_run` is the crate's first test to read
a derived value beside live writers rather than after joining them. Now prints: `  /// reading in a hundred understated the leak and a handful per two million`

### ST42 — Swapping the Two Loads Removes Every Miss and Adds a False Alarm

The obvious repair is to reverse the order: read `published` first, `claimed` second.
Then every claim landing between the loads is added to a `published` that predates it,
so the error runs the other way — the reading comes out at or above the truth. A leak
detector that errs toward reporting a leak sounds strictly better than one that errs
toward reporting health.

Measured, it is not.

**Finding.** Read the second and fourth rows of each run above. With the leak present,
the swapped order behaves exactly as predicted: zero understatements, zero readings of
zero, a floor at the true leak of eight. It never once misses.

With *no* leak at all — a perfectly healthy ring, one matched producer, at most one
slot ever outstanding — it reports more than one outstanding slot 57,665 and 60,107
times per two million readings, ranging as high as 832. Around three percent of
readings on a healthy ring announce a leak of hundreds of slots that does not exist.
The shipped order, on the same healthy ring, reads `0..1` and never overstates once.

So neither ordering is correct, and they are not correct in opposite directions: one
misses real leaks rarely, the other invents leaks often. There is no ordering of two
independent loads that gets this right, because the problem is not the order — it is
that the two values are read at two moments and the difference between two moments is
not a state.

What does work is reading each counter once and deriving afterwards, which is the
shape `ring_atomic::CountingSeq::counts` already used in this workspace and
`ring_stats` did not have
([`pattern/002`](../pattern/002_the_derived_reading_from_separate_loads.md) § ST39,
[`api/002`](../api/002_seven_readers_and_no_way_to_read_the_set.md) § ST8).
`RingStats::snapshot` is now that shape here. It buys internal consistency, not
atomicity — the numbers still come from seven loads at seven moments, so a snapshot
taken under traffic can still hold a combination the ring never had; what it
guarantees is that its own `in_flight` is its own `claimed - published` rather than
two figures read a moment apart. Recording the swap here still matters, because it is
the repair a reader of `algorithm/002` would reach for first and it makes the crate
worse on the case that is overwhelmingly more common — a ring that is fine.

The three tests that exercise `in_flight` are all still single-threaded, and that is
appropriate: they check the definition, which is a single-threaded property.
`a_snapshot_agrees_with_itself_while_writers_run` is the one that reads a derived
value beside four live writers, and it spawns five threads to do it — so the subject
of this instance is now inside what the suite observes, through the derived route
rather than through `in_flight` itself.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_stats
command grep -F 'pub fn snapshot( &self ) -> StatsCounts' src/lib.rs
```

Live output:

```
  pub fn snapshot( &self ) -> StatsCounts
```

**Disposition:** applied — `RingStats::snapshot` already ships as the shape this
finding calls for: seven loads taken together and folded into one `StatsCounts`
value, so a derived reading built from it disagrees with itself only if the
struct's own arithmetic is wrong, never because two of its inputs were read a
moment apart. This finding's own conclusion and ST41's disposition both point at
the same method; no further edit is needed beyond what those two already record.
Now prints: `pub fn snapshot( &self ) -> StatsCounts`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) | The two loads, and why the error has a direction |
| [`pattern/002`](../pattern/002_the_derived_reading_from_separate_loads.md) | The consistent-read shape that does work |
| [`invariant/002`](../invariant/002_claimed_never_trails_published.md) | What a zero reading does and does not establish |
| [`pitfall/002`](002_the_total_that_counts_a_refusal_as_a_loss.md) | The other reading a monitor displays, and its own defect |

### Sources

| Fact | Where |
|------|-------|
| "a nonzero reading here at rest means […] a leak" | Census above |
| `claimed` loaded before `published`, floored by `saturating_sub` | Census above |
| What the doc now says about a zero reading | Census above |
| Both orderings, leak and no-leak, two runs | Probe, quoted above |
| All three `in_flight` tests are single-threaded | Census above |
| The derived reading, read beside live writers | Census above |
| The consistent-read alternative | `ring_atomic::CountingSeq::counts` |

### Tests

| Test | Covers |
|------|--------|
| `in_flight_is_claimed_minus_published` | The subtraction at rest, one thread |
| `in_flight_saturates_rather_than_wrapping` | The floor, asserted as desirable |
| `consuming_does_not_affect_in_flight` | That the derivation reads the right two counters |
| `a_snapshot_agrees_with_itself_while_writers_run` | The derived reading taken beside four live writers — the condition under which the subtraction is wrong |
