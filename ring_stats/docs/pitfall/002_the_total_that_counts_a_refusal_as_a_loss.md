# Pitfall: The Total That Counts a Refusal as a Loss

### Scope

**Purpose:** Record what an operator reads off `dropped_total`, why the safest overflow
policy produces the most alarming number, and why the breakdown they would check
instead is not internally consistent.

**Responsibility:** `dropped_total`'s contract and doctest, the `Fail` policy's
presence in its fold, and the three-load breakdown beside it.

**In Scope:** `RingStats::dropped_total`, `RingStats::dropped`,
`RingStats::snapshot` and `StatsCounts` in `ring_stats/src/lib.rs`;
`OverflowPolicy::drops_silently` and `OverflowPolicy::reports_failure` in
`ring_types/src/policy.rs`;
`refusals_are_counted_even_though_nothing_is_lost` and
`a_snapshot_agrees_with_itself_while_writers_run` in
`ring_stats/tests/stats_test.rs`.

**Out of Scope:** The three counters as a data structure, and the unused filter as a
structural gap, are
[`data_structure/002`](../data_structure/002_three_drop_counters_behind_one_enum.md).
That the naming came from the crate's own pattern is
[`pattern/001`](../pattern/001_the_record_and_read_pair.md) § ST38.

---

## The Trap

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the total says it is, and what it folds --'
command grep -m1 -A12 -F '  /// Items lost across every policy' ring_stats/src/lib.rs
echo '  -- what ring_types already knows about the three policies --'
command grep 'pub const fn drops_silently\|pub const fn reports_failure' ring_types/src/policy.rs
echo '  -- and the test that pins the conflation --'
command grep -m1 -A7 -F '/// would make a saturated ring look idle.' ring_stats/tests/stats_test.rs | tail -n 7
echo '  -- and the reading whose breakdown is its own sum --'
command grep -m1 -A11 -F '/// **This is not an atomic snapshot, and no such thing is available here.** The' ring_stats/src/lib.rs
```

Live output:

```
  -- what the total says it is, and what it folds --
  /// Items lost across every policy — except `OverflowPolicy::Fail`, whose count
  /// is refusals folded in here anyway: the record-and-read pattern gives every
  /// policy the same drop verb, so this sum cannot tell a loss from a refusal.
  ///
  /// ```
  /// use ring_stats::RingStats;
  /// use ring_types::OverflowPolicy;
  /// let s = RingStats::new();
  /// for p in OverflowPolicy::ALL { s.record_drop( p, 1 ); }
  /// assert_eq!( s.dropped_total(), 3 );
  /// ```
  // Fix(ring_stats_dropped_total_overflow): `dropped_total` folded the three
  // per-policy counters with `Iterator::sum`, plain `u64` addition. `record_drop`
  -- what ring_types already knows about the three policies --
  pub const fn reports_failure( self ) -> bool
  pub const fn drops_silently( self ) -> bool
  -- and the test that pins the conflation --
#[ test ]
fn refusals_are_counted_even_though_nothing_is_lost()
{
  let stats = RingStats::new();
  for _ in 0..4 { stats.record_drop( OverflowPolicy::Fail, 1 ); }

  assert_eq!( stats.dropped( OverflowPolicy::Fail ), 4 );
  -- and the reading whose breakdown is its own sum --
/// **This is not an atomic snapshot, and no such thing is available here.** The
/// numbers still come from [`RingStats::COUNTERS`] `Relaxed` loads taken at that
/// many moments, so a set under traffic can return a combination the ring never
/// held. What the type does guarantee is that the reading is *internally*
/// consistent: `dropped_total` is the sum of the three drop fields **of this
/// value**, and `in_flight` is **this value's own** `claimed - published`.
/// Reading those through [`RingStats::dropped_total`] and
/// [`RingStats::in_flight`] instead re-loads the counters, so the total a caller
/// prints need not be the sum of the breakdown printed beside it — driven from
/// one writer that kept the three drop counters within one of each other, three
/// to five percent of separately-read breakdowns showed a spread the ring never
/// had, the widest running to 3,325.
```

---

### ST43 — Choosing the Policy That Loses Nothing Produces the Largest Loss Number

Three overflow policies. `DropNewest` and `DropOldest` discard an item and tell nobody
— `ring_types::drops_silently` is true for exactly those two. `Fail` refuses the push
and hands the item back, so the caller knows immediately and decides what to do; nothing
is lost. That is the whole reason to pick it.

`dropped_total` is documented "Items lost across every policy" and folds all three. Its
own doctest records one drop under each policy and asserts the total is three — two
losses and one refusal, counted alike, in the contract.

**Finding.** A ring configured `Fail` under sustained pressure produces a
`dropped_total` that climbs at the full refusal rate, and every one of those counts is
an item that was handed back intact. A ring configured `DropOldest` under the same
pressure produces the same number, and every one of those is gone. The two rings report
identically and mean opposite things, and the number carries nothing that distinguishes
them.

The direction of the error is what makes this a pitfall rather than a rounding
complaint. The operator who chose the safe policy — the one where the application is
told and can retry, buffer, or shed deliberately — is the operator whose dashboard
shows the biggest loss figure. The incentive runs backwards: the configuration that
loses nothing looks worst.

`ring_types` already exposes the discriminator as a `const fn` with its own doctests.
Filtering the fold on `drops_silently()` is one clause, and the crate does not depend on
anything it does not already declare.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_stats
command grep -F 'so this sum cannot tell a loss from a refusal' src/lib.rs
```

Live output:

```
    /// policy the same drop verb, so this sum cannot tell a loss from a refusal.
```

**Disposition:** applied — closed by the same doc edit `pattern/001` § ST38
records: `dropped_total`'s doc comment no longer reads as an unqualified "Items
lost across every policy" and now names the exact backwards-incentive case this
finding traces, that `OverflowPolicy::Fail` refusals are folded in as if lost.
The one-clause `drops_silently()` filter this finding also proposes would change
`dropped_total`'s actual return value, which `data_structure/002` § ST12 records
is pinned by `refusals_are_counted_even_though_nothing_is_lost`'s own assertion
— a behavioral change requiring a test change first, left undone here for the
same reason `data_structure/001` § ST9 declines its own structural remedy.
Now prints: `so this sum cannot tell a loss from a refusal`

---

### ST44 — The Breakdown That Would Disambiguate It Is Not Internally Consistent

The natural response to ST43 is to stop reading the total and read the three
per-policy counters instead. `dropped( DropNewest )`, `dropped( DropOldest )`,
`dropped( Fail )` — three calls, three separate `Relaxed` loads inside `dropped`,
three different moments.

**Finding.** Drive all three counters from one writer in a fixed order, so the true
spread between them is never more than one, and ask what a reader sees. Two million
breakdowns, twice:

```
  -- three policy counters, one writer, never more than 1 apart --
    per-policy breakdowns read             2000000
    showing a spread the ring never had    65023
    widest impossible spread               3325
  -- at rest, the three agree exactly --
    dropped( DropNewest ) = 314425
    dropped( DropOldest ) = 314425
    dropped( Fail ) = 314425
    dropped_total()          = 943275
```

```
  -- three policy counters, one writer, never more than 1 apart --
    per-policy breakdowns read             2000000
    showing a spread the ring never had    107580
    widest impossible spread               794
  -- at rest, the three agree exactly --
    dropped( DropNewest ) = 415133
    dropped( DropOldest ) = 415133
    dropped( Fail ) = 415133
    dropped_total()          = 1245399
```

Three to five percent of breakdowns show a spread the ring never held, the widest
running to 3,325 on counters that were never more than one apart. At rest the three
agree exactly, which is the condition every test reads them in.

The total itself cannot be caught out this way — its three loads all climb, so the sum
always lands between its value at the first load and at the last
([`algorithm/001`](../algorithm/001_eleven_operations_and_three_compositions.md)
§ ST2). It is the *breakdown* that tears, and the breakdown is what someone reaches for
precisely when the total looks wrong.

So both readings failed at the same moment and for opposite reasons. Under pressure
the total is semantically wrong if any refusals are in it — that half is ST43's, and
it is unaddressed here. The other half was that the breakdown which would separate
them was numerically inconsistent: three numbers true at three instants, presented as
a decomposition of a fourth. Nothing in the API offered a consistent read of the three
([`api/002`](../api/002_seven_readers_and_no_way_to_read_the_set.md) § ST8), and the
one test that exercises the three under contention,
`distinct_policy_counters_do_not_interfere_under_contention`, asserts after joining.

**Disposition:** applied — `RingStats::snapshot` returns a `StatsCounts` whose
`dropped_total` is the sum of that same value's own three drop fields, so the total
and the breakdown printed beside it are one reading rather than four. This does not
make the reading atomic, and the type says so in its own first paragraph: the numbers
are still `RingStats::COUNTERS` `Relaxed` loads at that many moments, and a set under
traffic can still return a combination the ring never held. What it removes is the
narrower failure this finding measured — a total that disagrees with the decomposition
presented as its parts. `a_snapshot_agrees_with_itself_while_writers_run` asserts that
relation across 50,000 reads taken beside four writers, and was proven able to fail:
making `snapshot` re-load through `self.dropped_total()` turns it red with a torn
reading of `dropped_newest: 8442, dropped_oldest: 6603, failed: 1771`. Reading through
`RingStats::dropped_total` still re-loads and still tears, which is why the type
documents that route as the one to avoid. Now prints: `/// to five percent of separately-read breakdowns showed a spread the ring never`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/002`](../data_structure/002_three_drop_counters_behind_one_enum.md) | The three counters and the unused `drops_silently` filter |
| [`pattern/001`](../pattern/001_the_record_and_read_pair.md) | Why a refusal ended up under the drop verb |
| [`pitfall/001`](001_the_leak_in_flight_cannot_see.md) | The other reading a monitor displays, and its own defect |
| [`api/002`](../api/002_seven_readers_and_no_way_to_read_the_set.md) | The consistent read that would fix the breakdown |

### Sources

| Fact | Where |
|------|-------|
| "Items lost across every policy", and the doctest asserting 3 | Census above |
| The fold over all three policies | Census above |
| `drops_silently` and `reports_failure` | Census above |
| The breakdown's observed spread, two runs | Probe, quoted above |
| The suite pins the conflation as correct | Census above |
| The self-consistent breakdown, and what it does not promise | Census above |

### Tests

| Test | Covers |
|------|--------|
| `refusals_are_counted_even_though_nothing_is_lost` | Asserts the conflation, by name |
| `the_drop_total_is_the_sum_over_every_policy` | That the fold covers all three, at rest |
| `a_drop_lands_under_its_own_policy_only` | That the three counters are independent |
| `distinct_policy_counters_do_not_interfere_under_contention` | Contention, asserted after joining |
| `a_snapshot_agrees_with_itself_while_writers_run` | 50,000 breakdowns read beside four writers, each asserted to be its own total's parts |
