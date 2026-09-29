# Algorithm: The Eligibility Filter Runs Before the Comparison

### Scope

- **Purpose**: Record that `Comparison::fastest` filters on losslessness before minimising on time, that the ordering of those two operations is the crate's one load-bearing rule, and that returning `None` is a result rather than a failure.
- **Responsibility**: State the steps, the ordering constraint, and what each alternative ordering produces.
- **In Scope**: The filter-then-minimise sequence; the `None` case; why the filter reads `received` and not `reported`.
- **Out of Scope**: How an `Outcome` is produced (→ [`algorithm/001`](001_one_workload_through_six_runners.md)); the report's rendering of the result (→ [`api/002`](../api/002_the_report_surface.md)).

### Abstract

`fastest` is two iterator adapters, and their order is the crate's one
load-bearing rule: filter out every candidate that lost records, *then* take the
smallest time among what remains. Reversing them yields a plausible table whose
recommendation is inverted, because the quickest way to finish a write phase is
to refuse every record.

### Algorithm

```rust
pub fn fastest( &self ) -> Option< &Outcome >
{
  self.outcomes
    .iter()
    .filter( | outcome | outcome.is_lossless() )   // 1
    .min_by_key( | outcome | outcome.write_nanos ) // 2
}
```

| # | Step | Why it is where it is |
|---|------|-----------------------|
| 1 | Discard every candidate that did not return the whole workload | **The quickest way to finish a write phase is to refuse every record.** A discarding path has a genuine, reproducible, best-in-table time |
| 2 | Among the survivors, take the smallest elapsed time | Only meaningful once every remaining row moved the same number of records |
| — | Empty survivor set ⇒ `None` | On a workload larger than the capacity this is the honest answer: no candidate is eligible, and the least-bad one is not a winner |

#### The ordering is the rule

**Reversing the two steps, or fusing them, produces a plausible table with an
inverted recommendation.** All three alternatives were considered:

| Alternative | What it produces |
|---|---|
| Minimise on time, then check losslessness of the winner | The winner is a discarding path, and the check reports it as such. **A caveat attached to a recommendation is still a recommendation** |
| Minimise on a composite score — time penalised by drop rate | Requires a weighting nobody can justify. Two arbitrary constants decide which path the family adopts |
| Minimise on time and let the reader consult the `dropped` column | This is the failure the crate was built with. The column was right and the ranking was read anyway |
| ✅ **Filter, then minimise** | A path that dropped records cannot appear as the answer at all |

**The rule is not "penalise dropping".** It is that losslessness is a
*precondition for being compared*, not a factor in the comparison. Two paths
that moved different numbers of records are not slower and faster than each
other; they did different amounts of work, and a ratio between their times is
not a measurement of anything.

#### The filter reads the drain

`is_lossless()` is `received == offered`, where `received` is what the consumer
produced after the clock stopped — **never `reported`, the count the write API
returned.** Under `OverflowPolicy::DropNewest` a full ring returns `Ok` for a
record it discarded, so a filter reading `reported` admits precisely the
candidates it exists to exclude, and admits them with a best-in-table time.

That is not hypothetical: it is what the first working version of this crate
did, and `contract_ring` was reported lossless at 256 records in a 16-slot ring.
→ [`pitfall/003`](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md).

#### `None` is a result

**`fastest()` returning `None` is the most informative output this crate
produces**, and it is the one a caller is most likely to treat as a failure. It
says: *at this capacity, against this offered load, none of these paths is a
candidate at all.* The correct response is to change the workload or the
capacity, not to pick the best of six.

The report renders it explicitly rather than omitting the line:

```text
fastest lossless: none — every candidate dropped records
```

An omitted line reads as a formatting gap. A stated absence reads as a finding,
and it is one.

### Complexity

Two passes over at most six outcomes. Nothing about this needs to be fast; it
runs once per comparison, after the measurement it summarises.

### Algorithms

| File | Relationship |
|------|--------------|
| [001_one_workload_through_six_runners.md](001_one_workload_through_six_runners.md) | Where the `Outcome`s this filters over come from |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_report_surface.md](../api/002_the_report_surface.md) | How the `None` case is rendered |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_three_counts_that_are_not_interchangeable.md](../data_structure/002_three_counts_that_are_not_interchangeable.md) | `is_lossless`'s definition, and why it reads the third count |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_no_test_asserts_an_ordering.md](../decisions/002_no_test_asserts_an_ordering.md) | Why the suite asserts the eligibility set and never the winner |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_received_never_exceeds_reported_never_exceeds_offered.md](../invariant/001_received_never_exceeds_reported_never_exceeds_offered.md) | The ordering that makes `is_lossless` well-defined for every candidate |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md) | The measurement that forced the filter to read the drain |

### Sources

| File | Relationship |
|------|--------------|
| [`../readme.md`](../readme.md) | "the comparison is the deliverable" — which is why the ordering of two lines of iterator code is an architectural matter |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `a_path_that_dropped_records_is_not_eligible_to_be_fastest` asserts `Some` and lossless on the roomy fixture, `None` on the cramped one; `the_report_names_every_candidate_and_every_refusal` asserts the `None` case is stated rather than omitted |

### BN3 — Eligibility Requires a Ring That Never Fills, So Only the Uncontended Case Can Be Ranked

`is_lossless()` is `received == offered`, and that is the whole filter:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the eligibility predicate --'
command grep 'self.received == self.offered\|pub const fn is_lossless' src/lib.rs
echo '  -- every capacity/offered pair the suite constructs --'
awk '/RingConfig::new\( [0-9]+ \)/{ cap = $0; sub( /.*RingConfig::new\( /, "", cap ); sub( / \).*/, "", cap )
       printf "    line %-5s capacity %s\n", NR, cap }' tests/bench_test.rs
echo '  -- and what fastest() is asserted to be, in each --'
command grep 'fastest()' tests/bench_test.rs | sed -E 's/^(.{0,100}).*/\1/'
```

Live output:

```
  -- the eligibility predicate --
  pub const fn is_lossless( &self ) -> bool
    self.received == self.offered
  -- every capacity/offered pair the suite constructs --
    line 89    capacity 4096
    line 99    capacity 16
    line 109   capacity 4096
    line 128   capacity 64
    line 314   capacity 4096
    line 506   capacity 16
    line 700   capacity 64
    line 743   capacity 64
    line 774   capacity 64
    line 847   capacity 64
    line 1295  capacity 4096
    line 1340  capacity 4096
  -- and what fastest() is asserted to be, in each --
  assert!( comparison.fastest().is_none(), "no candidate kept the workload" );
  assert!( comparison.fastest().is_none(), "16 slots still cannot hold 256 records" );
fn a_path_that_dropped_records_is_not_eligible_to_be_fastest()
  let fastest = roomy_run.fastest().expect( "every candidate was lossless" );
  assert!( cramped_run.fastest().is_none(), "no candidate kept the workload" );
  assert!( first.fastest().is_none() );
  assert!( second.fastest().is_none() );
    "the five candidates both cfg arms declare, in the order fastest() breaks ties by",
```

A candidate is eligible only if it kept every record offered. So a workload
whose offered load exceeds capacity has no eligible candidate at all — and
`a_failing_policy_closes_the_gap_for_every_candidate` proves this is not a
policy artefact. Under `OverflowPolicy::Fail`, where every path reports its
refusals honestly and `conserved()` is `true` for all five, `fastest()` is still
`None`. The comment in the test says it plainly: "16 slots still cannot hold 256
records."

**A ranking therefore exists only when the ring never fills**, which is to say
only when there is no back-pressure — and back-pressure is where a lock-free
ring and a mutex actually differ. The comparison is structurally confined to the
regime in which its own subject matter is least interesting, and nothing in the
crate says so.

This is the right rule. A candidate that dropped records is genuinely not
comparable to one that did not, and the alternative — ranking on time alone —
is the failure `decisions/002` was written to prevent. But the rule has a
consequence the filter's own documentation does not carry: the set of workloads
that produce an answer and the set that exercise contention do not overlap.

### BN4 — The Filter Has Never Separated Anything

Every workload in the crate is all-eligible or all-ineligible:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the three fixtures, and their headroom --'
awk '
  /^fn (roomy|cramped|parallel)\(/ { n = $2; sub( /\(.*/, "", n ); cap = 0; prod = 1; rec = 0; next }
  n != "" && /RingConfig::new\(/            { cap  = $0; sub( /.*RingConfig::new\( /, "", cap );           sub( / \).*/, "", cap ) }
  n != "" && /with_producers\(/             { prod = $0; sub( /.*with_producers\( /, "", prod );           sub( / \).*/, "", prod ) }
  n != "" && /with_records_per_producer\(/  { rec  = $0; sub( /.*with_records_per_producer\( /, "", rec ); sub( / \).*/, "", rec ) }
  n != "" && /^\}/ { printf "    %-9s %5s slots for %s x %-4s = %-5s offered   headroom %5.2fx\n", n, cap, prod, rec, prod * rec, cap / ( prod * rec ); n = "" }
' tests/bench_test.rs
echo '  -- what the suite asserts about the eligible set --'
command grep 'is_lossless()' tests/bench_test.rs | sed -E 's/^(.{0,104}).*/\1/'
echo '  -- and the executable examples in the docs --'
command grep -E '^\s*(//!|///) (assert!\( Comparison|assert!\( comparison)' src/lib.rs | sed -E 's/^(.{0,104}).*/\1/'
```

Live output:

```
  -- the three fixtures, and their headroom --
    roomy      4096 slots for 1 x 256  = 256   offered   headroom 16.00x
    cramped      16 slots for 1 x 256  = 256   offered   headroom  0.06x
    parallel   4096 slots for 4 x 256  = 1024  offered   headroom  4.00x
  -- what the suite asserts about the eligible set --
  assert!( direct.is_lossless(), "4096 slots hold 1024 records" );
  assert!( mutex.is_lossless() );
    assert!( outcome.is_lossless() );
  assert!( staged.is_lossless() );
    assert!( outcome.is_lossless(), "{} lost the partial tail", candidate.name() );
    assert!( !outcome.is_lossless(), "{} kept 256 records in 16 slots", candidate.name() );
    !through_the_factory.is_lossless(),
  assert!( fastest.is_lossless() );
  assert!( cramped_run.outcomes().iter().all( | o | !o.is_lossless() ) );
    assert_eq!( a.is_lossless(), b.is_lossless(), "{:?}", a.candidate() );
    assert!( outcome.is_lossless(), "{} built and filled a ring anyway", candidate.name() );
    assert!( outcome.is_lossless(), "{} lost records before the table could be checked", candidate.name(
    assert!( outcome.is_lossless() );
    assert!( outcome.is_lossless(), "{} dropped records under a 4096-slot ring", candidate.name() );
      assert!( outcome.is_lossless() );
  -- and the executable examples in the docs --
//! assert!( comparison.fastest().is_some(), "1024 slots hold 256 records" );
  /// assert!( Comparison::run( cramped ).fastest().is_none() );
```

`roomy` offers 256 into 4096 — sixteen times the headroom. `parallel` offers
1024 into 4096 — four times. `cramped` offers 256 into 16, and every candidate
loses. The two doctests that call `fastest()` repeat the same split: 256 into
1024 asserts `is_some()`, 1024 into 16 asserts `is_none()`.

**There is no case anywhere — test, fixture or doctest — in which some
candidates are eligible and others are not.** The suite asserts `all(
is_lossless )` on one side and `all( !is_lossless )` on the other. So the filter
that this instance calls the crate's one load-bearing rule has never once
removed a row from a set it did not remove entirely.

That matters because the filter's value is precisely in the mixed case. A
comparison where the mutex kept everything and one ring did not is the run where
reading the `write ns` column without the filter produces a wrong recommendation
— and it is the run nothing constructs. Building it needs only a capacity
between the two extremes, which no fixture has.

The general shape, worth carrying: **a rule tested only at its extremes is
tested where its answer is unanimous**, and unanimity is the one condition under
which the rule makes no difference.
