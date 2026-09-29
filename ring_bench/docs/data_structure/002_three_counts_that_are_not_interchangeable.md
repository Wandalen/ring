# Data Structure: Three Counts That Are Not Interchangeable

### Scope

- **Purpose**: Define the result record a run produces, and establish that its three counts answer three different questions — so that a judgement computed from the wrong one is wrong in a way no test elsewhere can catch.
- **Responsibility**: State the fields, the derived quantities, and which count each derivation must read.
- **In Scope**: `offered`, `reported`, `received`; the four derived methods; why `conserved` is a report rather than an assertion; `table`, and which count it is folded from.
- **Out of Scope**: The trap that forced the shape (→ [`pitfall/003`](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md)); `RingStats`' own fields, which are `ring_stats`'; the `cells`/`semantics` fields `table`'s shape is driven by, which are `Workload`'s (→ [`data_structure/001`](001_the_workload_description.md)).

### Abstract

The record a single run produces. Three of its fields are counts, they answer
three different questions, and only the third — what the drain actually yielded —
is treated as truth. A judgement computed from either of the other two is wrong
in a way no test outside this crate can catch.

### Structure

```rust
pub struct Outcome
{
  candidate : Candidate,
  producers : usize,
  offered : usize,
  reported : usize,
  received : usize,
  write_nanos : u128,
  stats : RingStats,
  table : AccumulatorTable,
}
```

| Count | Question it answers | Source |
|---|---|---|
| `offered` | How many records did the workload ask for? | Arithmetic on the `Workload`, before anything ran |
| `reported` | How many did the write API say it took? | Summed return values inside the timed region |
| `received` | **How many came back out?** | The drain, after the clock stopped |

**Only `received` is treated as truth.** The other two are context: `offered` is
the question, `reported` is the path's own claim, `received` is what happened.

#### The three are not interchangeable

**Under `OverflowPolicy::Fail` all three coincide up to the capacity and the
distinction is invisible.** Under the default policy — `DropNewest`, which is
what a bare `RingConfig::new( n )` produces — `reported` can exceed `received`
by two orders of magnitude, because a full ring returns `Ok` for a record it
discarded.

Measured, on 256 records into a 16-slot ring:

| Candidate | offered | reported | received | Behaviour |
|---|---:|---:|---:|---|
| `contract_ring` | 256 | **256** | 16 | absorbs — every push returned `Ok` |
| `off_the_shelf` | 256 | **256** | 16 | absorbs — same door, same silence |
| `mutex_queue` | 256 | 16 | 16 | hands refusals back |
| `direct_spsc` | 256 | 16 | 16 | hands refusals back |
| `direct_mpsc` | 256 | 16 | 16 | hands refusals back |
| `tls_over_ring` | 256 | **0** | 0 | stalls — keeps nothing at all |

**The divergence lands on a subset**, which is what makes reading the wrong
count a *ranking* error rather than a uniform overstatement. Three candidates
hand their refusals back, two absorb them, and one stalls outright; a comparison
read off `reported` is ordered wrong, not merely imprecise.

**The `tls_over_ring` row is a third behaviour, not a more extreme second one.**
It keeps *nothing* while every unstaged candidate keeps 16. The first flush is
rejected because the ring is already full; `ring_flush` documents that a
rejection leaves the records staged and must be retried, and this crate
deliberately does not retry — a harness that retries measures its own retry
loop. So the buffer stays occupied and every subsequent `append` fails.

This is the property staging adds and the one a capacity plan has to account
for: **staging converts a capacity shortfall from partial loss into a total
stall.** A direct writer under-provisioned by 16x still delivers 6% of the
workload; a staged writer under-provisioned by the same factor delivers none.
Neither the ring nor the flusher is at fault — it is what `OnBatch` means when
the batch cannot land.

### Operations

Every field is private with a read-only accessor, and every judgement above a raw
count is a method — so the four derived quantities below are separately reachable
and separately testable, and a caller cannot recombine the counts into a verdict
this crate does not make.

| Method | Definition | Reads |
|---|---|---|
| `dropped()` | `offered - received` | the drain — from the workload's point of view a refusal and a silent discard are the same event |
| `silently_discarded()` | `reported - received` | the gap — the size of the trap, published rather than hidden |
| `is_lossless()` | `received == offered` | the drain — the eligibility test for `Comparison::fastest` |
| `conserved()` | `reported == received` | the gap — **a report, not an assertion** |

**`conserved` was demoted.** It was written as a correctness check and it is
legitimately `false` under the default policy, so asserting it would have made
the family's own documented idiom untestable. It now answers *how* a candidate
arrived at its `dropped` figure: `true` means the path handed its refusals back
to the caller, `false` means it absorbed them. A `false` is a fact about the
configuration, not a failure of the run.

**`silently_discarded` is the measurement that would be lost by simplifying.**
The tempting simplification is to keep only `received` and drop `reported` —
one count, no confusion. It also deletes the only number distinguishing a path
that applies back-pressure from one that absorbs, which is a genuine and
otherwise-unavailable property of a write path.

### Invariant

**`received ≤ reported ≤ offered`, for every candidate under every policy.**
Nothing stronger holds: the middle term can equal the last while the first is a
fraction of it, which is exactly the `contract_ring` row above.
→ [`invariant/001`](../invariant/001_received_never_exceeds_reported_never_exceeds_offered.md).

### The Counters Are Fed From `received`

`stats` carries `ring_stats`'s counters, written once from totals after the clock
stops. All three lifecycle counters take `received`:

```rust
stats.record_claim( received as u64 );
stats.record_publish( received as u64 );
stats.record_consume( received as u64 );
stats.record_drop( policy, ( offered - received ) as u64 );
```

**The first version fed them workload numbers and it was wrong.**
`RingStats::in_flight()` is `claimed - published` — a *slot* concept — so
`claimed = offered, published = reported` made 240 refused records read as 240
leaked slots on a ring with 16 of them. A record the ring never took never
occupied a slot, so it was never claimed either.
→ [`pitfall/002`](../pitfall/002_a_counter_inside_the_timed_region_measures_itself.md).

### The Table Is Folded From `received`, Never `reported`

`table` carries the accumulator-semantics axis (→
[`data_structure/001`](001_the_workload_description.md)'s `cells` and
`semantics` fields). It is built once per run, in `run`, by folding every
element of `drained` — the same `Vec< Record >` `received` is a length of —
through `destination_of` and `AccumulatorTable::apply`:

```rust
let mut table = AccumulatorTable::new( workload.cells() );
for record in &drained
{
  let ( cell, delta ) = destination_of( workload, *record );
  table.apply( workload.semantics(), cell, delta );
}
```

**Never `reported`.** The divergence table two sections up is exactly why:
`contract_ring` and `off_the_shelf` report 256 for a run that received 16, so a
fold over `reported` would apply 240 deltas the ring never actually held —
poisoning the one number this crate treats as ground truth with the same
optimism `pitfall/003` already distrusts everything else for. Building `table`
from `drained` instead means the accumulator inherits `received`'s guarantee
for free: whatever it disagrees with, it cannot disagree with the drain.

`AccumulatorTable::cells()` returns `&[ i64 ]`, one entry per `Workload::cells`.
Under `AccumulatorSemantics::Set`, `apply` overwrites a cell; under `Delta`, it
adds — so the same fold produces last-write-wins or a commutative sum depending
only on which `semantics` the workload asked for, never on candidate identity.

### `write_nanos` Is Read and Never Asserted

The one non-deterministic field. `Outcome::write_nanos` covers the publication
phase only — not construction, not the drain — and no test in this crate
constrains it. It is nonetheless *read* by the suite, because a field nobody
reads can silently stop being populated.
→ [`decisions/002`](../decisions/002_no_test_asserts_an_ordering.md).

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_workload_through_six_runners.md](../algorithm/001_one_workload_through_six_runners.md) | Where each count is produced, and why `reported` and `received` are separate return values |
| [../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md) | The judgement that reading the wrong count inverts |

### Data Structures

| File | Relationship |
|------|--------------|
| [001_the_workload_description.md](001_the_workload_description.md) | Where `offered` comes from |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_received_never_exceeds_reported_never_exceeds_offered.md](../invariant/001_received_never_exceeds_reported_never_exceeds_offered.md) | The ordering stated as a property, with the candidate-by-candidate reasons |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_measurement_is_a_value.md](../pattern/001_the_measurement_is_a_value.md) | Why every one of these is a field with an accessor rather than a printed column |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md) | The measurement that forced three counts where one was planned |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_types/src/policy.rs`](../../../ring_types/src/policy.rs) | `DropNewest` as the default, and `drops_silently()` — the predicate that names the property |
| [`ring_stats/src/lib.rs`](../../../ring_stats/src/lib.rs) | `in_flight()`'s definition, which the first counter mapping contradicted |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `a_dropnewest_ring_reports_successes_it_did_not_keep` asserts the divergence table above; `a_failing_policy_closes_the_gap_for_every_candidate` asserts it closes under `Fail`; `the_counters_are_the_runs_own_totals` asserts the `received`-fed mapping and the `in_flight` zero; `the_overflow_gap_never_reaches_the_accumulator_table` asserts `table` on the same divergence fixture and pins it to `received`, not `reported` |

### BN11 — Two of the Three Counters Are Fed the Same Expression, Which Zeroes the Only Quantity Derived From Them

The section above records *that* the three take `received`. What follows from it
is that the one derived reading `ring_stats` offers cannot be anything but zero:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the four counter writes, and what each is given --'
command grep 'stats.record_' src/lib.rs | sed 's/^/    /'
echo '  -- every accessor ring_stats offers, derived ones last --'
command grep -E 'pub fn (claimed|published|consumed|dropped|dropped_total|wait_nanos|in_flight)' ../ring_stats/src/lib.rs | sed 's/^/    /'
echo '  -- the only one that is a computation --'
awk '/pub fn in_flight/, /^  \}/' ../ring_stats/src/lib.rs | sed 's/^/    /'
echo '  -- and what the suite checks --'
awk '/fn the_counters_are_the_runs_own_totals/, /^\}/' tests/bench_test.rs \
  | command grep -E 'assert_eq!\( stats' | sed 's/^/    /'
echo '  -- and what now names the zero as structural, in both places --'
command grep -m1 -A2 -F '// Fix(BN11): that zero is *structural*, not measured.' src/lib.rs | sed 's/^/    /'
command grep -m1 -A2 -F '// Structural, not measured (BN11).' tests/bench_test.rs | sed 's/^/    /'
```

Live output:

```
  -- the four counter writes, and what each is given --
      stats.record_claim( received as u64 );
      stats.record_publish( received as u64 );
      stats.record_consume( received as u64 );
      stats.record_drop( workload.config().overflow(), ( offered - received ) as u64 );
  -- every accessor ring_stats offers, derived ones last --
      pub fn claimed( &self ) -> u64
      pub fn published( &self ) -> u64
      pub fn consumed( &self ) -> u64
      pub fn dropped( &self, policy : OverflowPolicy ) -> u64
      pub fn dropped_total( &self ) -> u64
      pub fn wait_nanos( &self ) -> u64
      pub fn in_flight( &self ) -> u64
  -- the only one that is a computation --
      pub fn in_flight( &self ) -> u64
      {
        self.claimed().saturating_sub( self.published() )
      }
  -- and what the suite checks --
      assert_eq!( stats.claimed(), outcome.received() as u64 );
      assert_eq!( stats.published(), outcome.received() as u64 );
      assert_eq!( stats.consumed(), outcome.received() as u64 );
      assert_eq!( stats.dropped( policy ), outcome.dropped() as u64 );
      assert_eq!( stats.dropped_total(), outcome.dropped() as u64 );
      assert_eq!( stats.in_flight(), 0 );
      assert_eq!( stats.wait_nanos(), 0 );
  -- and what now names the zero as structural, in both places --
      // Fix(BN11): that zero is *structural*, not measured. `in_flight` is
      // `claimed - published` and both are this same expression, so no run of this
      // harness can make it nonzero — the two assertions on it in the suite pin the
      // Structural, not measured (BN11). `in_flight` is `claimed - published` and
      // `run` hands both the same expression, so this reads 0 for every mapping that
      // keeps them equal — including the wrong one that maps `reported` through both,
```

`in_flight()` is `claimed - published`, and both are handed `received as u64` on
adjacent lines. **The subtraction is `x - x` for every candidate, every
workload, every capacity and every policy.** `assert_eq!( stats.in_flight(), 0 )`
therefore holds by construction of the two adjacent `record_` calls, not by
anything the run did.

**This is not the same as saying the assertion is worthless**, and the
distinction is the interesting part. It guards precisely one regression — the
historical one this document records, where `claim` was fed `offered` and
`publish` was fed `reported`. Change one of the two lines and the assertion
fires. Change both together, or change neither, and it cannot. So the test
protects the *edit* that was once made and not the *property* it is written as.

`consumed` is worse off: `in_flight` does not read it, no other accessor derives
anything from it, and the suite's only check on it —
`assert_eq!( stats.consumed(), outcome.received() )` — restates its own writer's
argument. It is
a counter with one writer, one reader, and the reader asserts the writer's
argument.

`ring_stats`'s counters describe a slot lifecycle: claimed, then published, then
consumed. This crate has no access to that lifecycle — it measures a write path
from outside and learns one number, the drain count — so it writes that one
number into all three stages. **The record is honest and the vocabulary is
borrowed**, which leaves three fields that look like three measurements and are
one measurement stated three times.

The general shape: **a counter set whose stages are all fed the same expression
cannot report a stage**, and the derived quantities built on top of it become
assertions about the source code rather than about the run.

That "change both together and it cannot fire" clause was tested rather than
argued. Both `record_claim` and `record_publish` were switched to `reported` — a
mapping that is wrong for `DropNewest`, where a discarded record never reaches a
slot — and the suite was run: `in_flight() == 0` stayed green in both places it
is asserted, while `a_dropnewest_ring_reports_successes_it_did_not_keep` failed
on `claimed()` reading 256 against a 16-slot ring. The mapping is guarded, and
not by the assertion that looks like it guards it. The source was restored
byte-for-byte before anything else was changed.

What that leaves is a labelling problem rather than a coverage one, so that is
what was fixed. `run`'s comment and both `in_flight` assertions now say the zero
is structural and name the assertion that actually catches a mapping edit; the
second assertion's failure message also stopped claiming the wrong thing — it
promised a 240-leak reading that, per the experiment above, no mapping in this
harness's history would produce.

**Disposition:** applied — the mapping is correct and unchanged, and the two
`in_flight` assertions are unchanged in value; what changed is that neither is
now readable as a measurement. Now prints: `Fix(BN11): that zero is *structural*, not measured.`

### BN12 — Every Drop Is Filed Under an Overflow Policy, Including the Candidate That Has None

`record_drop` takes a policy and buckets by it. The mutex baseline is handed one
it never consults:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- how ring_stats files a drop --'
awk '/pub fn record_drop/, /^  \}/' ../ring_stats/src/lib.rs | sed 's/^/    /'
echo '  -- what this crate passes, for every candidate alike --'
command grep 'stats.record_drop' src/lib.rs | sed 's/^/    /'
echo '  -- and whether the mutex path has ever heard of a policy --'
printf '    mentions of overflow/policy in run_mutex_queue : %s\n' \
  "$( awk '/^fn run_mutex_queue/, /^\}/' src/lib.rs | command grep -cE 'overflow|OverflowPolicy|policy' )"
printf '    mentions of overflow/policy in commit_batch    : %s\n' \
  "$( awk '/^fn commit_batch/, /^\}/' src/lib.rs | command grep -cE 'overflow|OverflowPolicy|policy' )"
echo '  -- what commit_batch actually does when the queue is full --'
awk '/^fn commit_batch/, /^\}/' src/lib.rs | command grep -E 'guard.len|taken|drain' | sed 's/^/    /'
echo '  -- and which fixture the counter test uses --'
awk '/fn the_counters_are_the_runs_own_totals/, /^\}/' tests/bench_test.rs \
  | command grep -E 'let workload|let policy|Candidate::' | sed 's/^/    /'
```

Live output:

```
  -- how ring_stats files a drop --
      pub fn record_drop( &self, policy : OverflowPolicy, n : u64 )
      {
        let counter = match policy
        {
          OverflowPolicy::DropNewest => &self.dropped_newest,
          OverflowPolicy::DropOldest => &self.dropped_oldest,
          OverflowPolicy::Fail => &self.failed,
        };
        counter.fetch_add( n, Ordering::Relaxed );
      }
  -- what this crate passes, for every candidate alike --
      stats.record_drop( workload.config().overflow(), ( offered - received ) as u64 );
  -- and whether the mutex path has ever heard of a policy --
    mentions of overflow/policy in run_mutex_queue : 0
    mentions of overflow/policy in commit_batch    : 0
  -- what commit_batch actually does when the queue is full --
      let mut taken = 0;
      for record in staged.drain( .. )
        if guard.len() < capacity
          taken += 1;
      taken
  -- and which fixture the counter test uses --
      let workload = cramped();
      let policy = workload.config().overflow();
      let outcome = run( Candidate::MutexQueue, &workload ).unwrap();
```

`OverflowPolicy` names three ring behaviours: discard the incoming record,
discard the oldest one, or refuse the push. `run_mutex_queue` implements none of
them. `commit_batch` drains its staging vector and increments `taken` only when
`guard.len() < capacity`, so a record arriving at a full `VecDeque` is destroyed
where it stands — which is *behaviourally* `DropNewest` and is not selected by
any policy, because the code never reads one.

`run` nevertheless files those losses under `workload.config().overflow()`. On
the default that lands them in `dropped_newest`, which happens to describe what
occurred. Under `OverflowPolicy::DropOldest` the same losses are filed as
oldest-discarded, which is the opposite of what occurred. Under
`OverflowPolicy::Fail` they are filed as `failed` — a bucket meaning "the push
was refused and the caller was told" — and the mutex queue told nobody.

**So one physical event is filed into three different counters depending on a
field the code path executing it never reads.** The counter is not wrong about
the count; it is wrong about the reason, and the reason is the only thing the
three buckets distinguish.

`a_failing_policy_closes_the_gap_for_every_candidate` runs every candidate under
`Fail` — including this one — and asserts `conserved()`, which for the mutex is
already unconditional
(→ [`algorithm/001`](../algorithm/001_one_workload_through_six_runners.md)'s
BN1). Nothing asserts which bucket a mutex drop lands in, and
`the_counters_are_the_runs_own_totals` reads `stats.dropped( policy )` with the
same `policy` the write used, so it agrees with itself whatever that policy is.

The general shape: **a categorised counter written by a caller that knows the
category and a callee that does not can only be as accurate as their agreement**
— and here there is no agreement to be accurate about, because one of the two has
no opinion.
