# Invariant: `claimed` Never Trails `published` — Assumed, Named, Enforced Nowhere

### Scope

**Purpose:** Record the one relation between two counters that the crate depends on,
who is responsible for it, and what happens when it does not hold.

**Responsibility:** `claimed >= published`, the `saturating_sub` that assumes it, and
every place the crate could detect a violation and does not.

**In Scope:** `RingStats::in_flight`'s subtraction and
`StatsCounts::checked_in_flight` in `ring_stats/src/lib.rs`;
`in_flight_saturates_rather_than_wrapping` and
`checked_in_flight_tells_a_caller_bug_from_a_balanced_ring` in
`ring_stats/tests/stats_test.rs`; the four recorder calls in
`ring_bench/src/lib.rs`.

**Out of Scope:** Monotonicity of each counter on its own is
[`invariant/001`](001_monotone_per_counter_and_only_per_counter.md). The seam that
makes the subtraction understate is
[`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) § ST3.

---

## An Invariant Named in a Test, Guarded by Nothing

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the subtraction, and the floor under it --'
command grep -m1 -F '    self.claimed().saturating_sub( self.published() )' ring_stats/src/lib.rs
echo '  -- what the suite says a violation is --'
command grep -m1 -A13 -F '/// In-flight floors at zero rather than wrapping. Publishing more than was' ring_stats/tests/stats_test.rs
echo '  -- how many assertions guard it in the crate --'
printf '    debug_assert in src: %s\n' "$( command grep -c 'debug_assert' ring_stats/src/lib.rs || true )"
echo '  -- and the reading that reports the violation as itself --'
command grep -m1 -A3 -F '  pub const fn checked_in_flight( &self ) -> Option< u64 >' ring_stats/src/lib.rs
echo '  -- and the one production writer that keeps the two equal on purpose --'
sed -n '/^  \/\/ Every counter is the \*drained\* count, never the reported one\. A record that$/,/^  \/\/ mapping is what keeps `in_flight` at zero here, so a nonzero reading would$/p;/^  stats\.record_claim( received as u64 );$/,/^  stats\.record_publish( received as u64 );$/p' ring_bench/src/lib.rs
```

Live output:

```
  -- the subtraction, and the floor under it --
    self.claimed().saturating_sub( self.published() )
  -- what the suite says a violation is --
/// In-flight floors at zero rather than wrapping. Publishing more than was
/// claimed is a caller bug, and `u64` subtraction would turn it into an
/// 18-quintillion-slot reading that looks like catastrophic leakage. The floor
/// is not only a backstop for that bug: on a busy, correct ring it also
/// absorbs the ordinary read-order race between the two loads, one to two
/// percent of the time — without it, a healthy ring would report the same
/// catastrophic reading this test builds by hand.
#[ test ]
fn in_flight_saturates_rather_than_wrapping()
{
  let stats = RingStats::new();
  stats.record_claim( 2 );
  stats.record_publish( 5 );
  assert_eq!( stats.in_flight(), 0 );
  -- how many assertions guard it in the crate --
    debug_assert in src: 0
  -- and the reading that reports the violation as itself --
  pub const fn checked_in_flight( &self ) -> Option< u64 >
  {
    self.claimed.checked_sub( self.published )
  }
  -- and the one production writer that keeps the two equal on purpose --
  // Every counter is the *drained* count, never the reported one. A record that
  // came back out claimed exactly one slot and published it; a record that did
  // not claimed none — a `DropNewest` discard never reaches a slot at all. That
  // mapping is what keeps `in_flight` at zero here, so a nonzero reading would
  stats.record_claim( received as u64 );
  stats.record_publish( received as u64 );
```

---

### ST23 — The Crate Names the Caller Bug and Then Makes It Unobservable

`in_flight` is a subtraction, so it depends on a relation the seven counters do not
enforce among themselves: every publish must correspond to a claim already recorded.
Nothing in `RingStats` checks this. `record_publish` takes a `u64` and adds it;
`record_claim` is a separate call the caller may or may not have made. The crate
contains zero `debug_assert` of any kind.

The suite knows the invariant exists and states it precisely — "Publishing more than
was claimed is **a caller bug**" — and then pins the behaviour that follows from
violating it. Two cases are asserted: claim 2 then publish 5 reads `0`, and publish 1
having claimed nothing reads `0`.

**Finding.** The reasoning for `saturating_sub` is given in the same sentence and it
is sound as far as it goes: plain `u64` subtraction would produce a value near
`u64::MAX`, "an 18-quintillion-slot reading that looks like catastrophic leakage".
Turning a caller bug into a false catastrophe is a bad trade.

But saturation is not the only alternative to wrapping. `checked_sub` returns
`Option< u64 >`, where `None` is precisely and only "published exceeds claimed" —
the caller bug, reported as itself, with no 18-quintillion reading and no floor.
`Some( 0 )` would still mean a balanced ring. The crate had a way to keep the two
apart for one word of difference, and had chosen the one that merges them.

So the invariant was: named in a test doc, depended on by a public method, enforced
by no assertion, and reported — when broken — as the value a healthy ring returns.

`StatsCounts::checked_in_flight` is now that one word of difference. It lives on the
snapshot rather than on `RingStats` so that the crate has one derived-reading route
rather than two, and `in_flight`'s own saturating behaviour is unchanged — the
18-quintillion reading is still not a thing this crate can produce. What changed is
that the caller bug is no longer indistinguishable from health: `None` means it and
means nothing else. The invariant is still enforced by no assertion, because nothing
in `RingStats` can know whether a claim was recorded before the publish it is handed;
what it now has is a reading that reports the violation instead of absorbing it.

**Disposition:** applied — `StatsCounts::checked_in_flight` added, returning `None`
exactly when `published` exceeds `claimed`, with `RingStats::in_flight` left
saturating and its doc pointing at the checked form.
`checked_in_flight_tells_a_caller_bug_from_a_balanced_ring` asserts the three sets
the suite already pinned at `0` — balanced, claim-2-publish-5, and publish-having-
never-claimed — read `Some( 0 )`, `None`, `None` through the new route while still
reading `0` through the old one. Now prints: `    self.claimed.checked_sub( self.published )`

---

### ST24 — Zero Now Means Three Different Things, and the Doc Scopes Only One

`in_flight() == 0` is reachable by three distinct routes:

1. **Healthy.** Every claim has been published. This is what the reading is for, and
   what `ring_bench` arranges deliberately: `claimed` and `published` are both fed
   `received`, with nine lines of comment explaining that the mapping "is what keeps
   `in_flight` at zero here".
2. **Caller bug.** More publishes were recorded than claims, floored by
   `saturating_sub` — the case ST23 covers, asserted twice in the suite.
3. **Masked leak.** A genuine leak of open claims, read across the window between the
   two loads, floored at zero by the same operation — measured at 27 occurrences in
   two million samples with eight slots leaked
   ([`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) § ST3).

The method's doc opened by addressing one of the three: "a nonzero reading here at
rest means a producer took a slot and abandoned it, which is a leak of ring
capacity." That sentence is correct and carefully hedged — *nonzero*, *at rest*. It
said nothing about zero, and nothing about not-at-rest, which between them are cases
2 and 3.

**Finding.** The overloading is not an accident of implementation; two of the three
routes were chosen. `saturating_sub` was selected over wrapping on purpose and over
`checked_sub` by omission, and `ring_bench` arranges route 1 explicitly so its
assertion means something. What was missing was any statement that the same value
arrives by three roads.

Each road is now named, though in two places rather than one. Route 3 is on
`in_flight` itself — that a zero taken under traffic is evidence of nothing, with the
measured rate beside it (§ ST3's disposition). Route 2 is on
`StatsCounts::checked_in_flight`, whose whole contract is that `None` is the caller
bug and `Some( 0 )` is a balanced ring (§ ST23's disposition). Route 1 is the
original hedged sentence, unchanged. What no single line yet says is that the three
converge — that this one value arrives by three roads and only a nonzero reading at
rest picks one out — which is a sentence on `in_flight` and belongs to this finding
rather than to either of the two that supplied its halves.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_stats
command grep -F 'is where three separate roads end' src/lib.rs
```

Live output:

```
    /// **Zero is where three separate roads end.** A healthy ring reads it
```

**Disposition:** applied — `in_flight`'s doc comment now names the convergence
directly: a healthy ring, a leak sampled at the wrong moment, and
`published > claimed` all floor to the same zero, with only
`checked_in_flight`'s `None` separating the caller-bug road from the other two —
closing the gap ST23 and the leak paragraph each left unaddressed on their own.
Now prints: `is where three separate roads end`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) | The seam that supplies the third route to zero, measured |
| [`invariant/001`](001_monotone_per_counter_and_only_per_counter.md) | The property each counter has on its own, and which readings inherit it |
| [`pitfall/001`](../pitfall/001_the_leak_in_flight_cannot_see.md) | What a monitor concludes from a zero |
| [`integration/002`](../integration/002_the_read_path_and_the_removed_edge.md) | `ring_bench` arranging route 1 on purpose |

### Sources

| Fact | Where |
|------|-------|
| The subtraction and its floor | Census above |
| "Publishing more than was claimed is a caller bug" | Census above |
| Both violation cases asserted as `0` | Census above |
| Zero `debug_assert` in the crate | Census above |
| `ring_bench` keeping the two equal on purpose | Census above |
| `in_flight`'s hedged doc, and the checked reading beside it | Census above |
| A leak floored to zero under traffic | `algorithm/002` § ST3 |

### Tests

| Test | Covers |
|------|--------|
| `in_flight_saturates_rather_than_wrapping` | Both caller-bug cases, asserted to read `0` |
| `in_flight_is_claimed_minus_published` | The relation when it does hold |
| `consuming_does_not_affect_in_flight` | That no third counter enters the relation |
| `ring_bench`'s `the_counters_are_the_runs_own_totals` | `ring_bench`'s deliberate arrangement of route 1 |
| `checked_in_flight_tells_a_caller_bug_from_a_balanced_ring` | Routes 1 and 2 separated — the same three sets, read both ways |
