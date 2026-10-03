# Manual test plan for `ring_bench`

Readings and measurements the automated suite cannot make. Five stages, each
with a command, a prediction written **before** running it, and the observed
result.

The prediction-first order is the point. A stage written after seeing the output
records what happened; a stage written before it can be *wrong*, and a wrong
prediction is the only thing here that teaches anything.

## Why these five are manual

| Stage | Why a test cannot do it |
|---|---|
| B1 | Nothing in this crate prints. The suite asserts substrings of `report()`; whether the table *reads* correctly to a human is not a substring property |
| B2 | It requires editing `src/lib.rs` to remove the one rule the crate's verdict depends on. A test cannot degrade the crate under test |
| B3 | It measures how unstable the ordering is, which is the one quantity the suite is forbidden to assert. A test that measured it would be the flaky test the prohibition exists to prevent |
| B4 | The suite runs under one feature configuration at a time. Whether the accounting identity survives the other is a question about the build |
| B5 | It reintroduces the crate's most expensive historical defect to ask whether the suite would now catch it. A suite cannot ask that of itself |

---

## B1. What does the report look like?

The suite asserts `report()` only by substring. Three workloads exercise the
three shapes it can take: everything kept, everything lossy, and most
candidates refused.

**Command.** A throwaway `tests/zz_report_probe.rs` printing the three reports,
run with `--nocapture`, then deleted.

```sh
cargo test -p ring_bench --all-features --test zz_report_probe -- --nocapture
```

**Prediction.** Six rows at one producer on both the roomy and cramped
workloads. All lossless on roomy with a `fastest` line; all lossy on cramped
with no `fastest`. At four producers, **three** refusals and a three-row table.

**Result (2026-08-28): the shape was right and the count was wrong.**

```
===== ROOMY =====
1 producer(s) x 256 records, batch 32, capacity 4096, overflow DropNewest
candidate          offered  reported  received  dropped   silent     write ns
mutex_queue            256       256       256        0        0       315283
contract_ring          256       256       256        0        0        85201
tls_over_ring          256       256       256        0        0       110041
direct_spsc            256       256       256        0        0        79201
direct_mpsc            256       256       256        0        0       323883
off_the_shelf          256       256       256        0        0        34680
fastest lossless: off_the_shelf

===== CRAMPED =====
1 producer(s) x 256 records, batch 32, capacity 16, overflow DropNewest
candidate          offered  reported  received  dropped   silent     write ns
mutex_queue            256        16        16      240        0       130721
contract_ring          256       256        16      240      240        19480
tls_over_ring          256         0         0      256        0         6760
direct_spsc            256        16        16      240        0        14640
direct_mpsc            256        16        16      240        0       272162
off_the_shelf          256       256        16      240      240        34481
fastest lossless: none — every candidate dropped records

===== PARALLEL =====
4 producer(s) x 256 records, batch 32, capacity 4096, overflow DropNewest
candidate          offered  reported  received  dropped   silent     write ns
mutex_queue           1024      1024      1024        0        0       533764
direct_mpsc           1024      1024      1024        0        0      1107849
refused: contract_ring admits 1 producer(s), asked for 4
refused: tls_over_ring admits 1 producer(s), asked for 4
refused: direct_spsc admits 1 producer(s), asked for 4
refused: off_the_shelf admits 1 producer(s), asked for 4
fastest lossless: mutex_queue
```

**Three things came out of one reading.**

**(1) Four refuse at four producers, not three.** Only `mutex_queue` and
`direct_mpsc` survive, giving a two-row table against four refusal lines. The
suite never contradicted the wrong number because it asserts the *identity*
`outcomes + refusals == ALL`, which holds at any split.

**(2) The four producer ceilings of `1` have three different causes.** Two are
the door's (`contract_ring`, `off_the_shelf`), one is `ring_flush`'s
(`tls_over_ring`, where `ring_core::Producer::try_clone` would hand out more but
a `Flusher` owns one), and one is real (`direct_spsc`, where `try_clone` returns
`None`).

**(3) `tls_over_ring` keeps nothing on the cramped workload.** It reads
`reported 0, received 0, dropped 256`, while every other candidate keeps 16.
The first flush is rejected, its records stay staged, and by the crate's
deliberate no-retry rule every later `append` fails. This is not a defect. It
is the sharpest available demonstration that staging converts a capacity
shortfall into a total stall rather than a partial loss. On the cramped
workload the measured split is three handing their refusals back, two
absorbing, one stalling.

**Re-run (2026-10-03), after `ring_core` ADR 004 made `Fail` the default.** The
same three workloads, through `cargo run -p ring_bench --release --all-features
--example comparison`. Every header now reads `overflow Fail`. On the cramped
workload `contract_ring` and `off_the_shelf` read `reported 16, received 16,
silent 0`, like every other row, so no candidate absorbs a refusal any more.
`tls_over_ring` reads 16 and 16, as it has since the batch clamp. Roomy and
parallel are unchanged apart from the header. The output above is the run under
the old `DropNewest` default, kept as the history behind the change.

```
1 producer(s) x 256 records, batch 16, capacity 16, overflow Fail
candidate          offered  reported  received  dropped   silent     write ns
mutex_queue            256        16        16      240        0        56208
contract_ring          256        16        16      240        0          708
tls_over_ring          256        16        16      240        0          708
direct_spsc            256        16        16      240        0         2625
direct_mpsc            256        16        16      240        0        45375
off_the_shelf          256        16        16      240        0          458
fastest lossless: none — every candidate dropped records
```

---

## B2. Does the suite go red when the eligibility filter is removed?

The one rule the crate's verdict depends on is that a path which dropped records
cannot be fastest. Removing it makes the harness recommend whichever candidate
discarded the workload quickest, and the number still looks fine.

**Command.** Drop the filter, run, revert.

```rust
    self.outcomes
      .iter()
      // .filter( | outcome | outcome.is_lossless() )   // PROBE ONLY
      .min_by_key( | outcome | outcome.write_nanos )
```

```sh
cargo test -p ring_bench --all-features --test bench_test
cargo test -p ring_bench --all-features --doc
```

**Prediction.** Two failures: `a_path_that_dropped_records_is_not_eligible_to_be_fastest`
and the `Comparison::fastest` doctest.

**Result (2026-08-28): red, and by more than predicted, with five behavioural
failures plus the doctest.**

```
FAIL  a_dropnewest_ring_reports_successes_it_did_not_keep
FAIL  a_failing_policy_closes_the_gap_for_every_candidate
FAIL  a_comparison_of_the_same_workload_repeats_its_counts
FAIL  a_path_that_dropped_records_is_not_eligible_to_be_fastest
FAIL  the_report_names_every_candidate_and_every_refusal
Summary  18 tests run: 13 passed, 5 failed
FAIL  src/lib.rs - Comparison::fastest
```

**The rule is defended in depth, and mostly by accident.** Four of the five
failures are in tests written for other purposes; they catch it because each
happens to pin `fastest()` on a lossy workload along the way. The real detector
is an assertion pattern, not a test name.

---

## B3. How unstable is the ordering the suite refuses to assert?

No test asserts which candidate is fastest, on the grounds that it would be
flaky. That is a prediction about variance, and this stage measures it.

**Command.** Ten comparisons of the roomy workload in one process, printing
every duration and the winner.

**Prediction.** At least two distinct winners across ten rounds.

**Result (2026-08-28): two distinct winners, and the margins say something
stronger than the winner count does.**

```
round 0: winner=DirectSpsc   contract_ring=33881  direct_spsc=30960  off_the_shelf=31321
round 1: winner=OffTheShelf  contract_ring=33040  direct_spsc=30800  off_the_shelf=30241
round 2: winner=OffTheShelf  contract_ring=95800  direct_spsc=90240  off_the_shelf=86961
round 3: winner=OffTheShelf  contract_ring=94921  direct_spsc=89441  off_the_shelf=64761
round 4: winner=OffTheShelf  contract_ring=55760  direct_spsc=52080  off_the_shelf=42200
round 5: winner=OffTheShelf  contract_ring=45640  direct_spsc=31240  off_the_shelf=30481
round 6: winner=OffTheShelf  contract_ring=71041  direct_spsc=66760  off_the_shelf=64680
round 7: winner=OffTheShelf  contract_ring=35200  direct_spsc=32961  off_the_shelf=31320
round 8: winner=OffTheShelf  contract_ring=32960  direct_spsc=30961  off_the_shelf=30680
round 9: winner=OffTheShelf  contract_ring=33281  direct_spsc=43000  off_the_shelf=30240
DISTINCT WINNERS ACROSS 10 ROUNDS: 2  [DirectSpsc, OffTheShelf]
```

**Within-candidate variance dwarfs between-candidate difference.** `contract_ring`
ranges 32960–95800 ns across ten rounds of an identical workload, a factor of
2.9. The gap between the three leaders in any single round is 1–3%. An assertion
that `off_the_shelf` beats `direct_spsc` would be asserting a 1% difference
against 190% noise, which is why no test asserts an ordering.

**But a coarse ordering is stable, and it is the one this comparison asked for.**
Across all ten rounds the three leaders never overlap the other two:

| Group | Range across 10 rounds |
|---|---|
| `contract_ring`, `direct_spsc`, `off_the_shelf` | 30240 – 95800 ns |
| `mutex_queue` | 130601 – 291362 ns |
| `direct_mpsc` | 239282 – 653645 ns |

So at one producer the lock-free paths beat the mutex baseline by 3–5x with no
overlap in any round, and that verdict *is* safe to state. What is not safe is
ranking the three leaders against each other.

---

## B4. Does the accounting identity survive the feature being off?

`crossbeam` is opt-in, so `Candidate::ALL` has one more entry with
`--all-features` than by default. The suite expresses every count against
`Candidate::ALL.len()` rather than a literal.

**Command.**

```sh
cargo test -p ring_bench --no-default-features
cargo test -p ring_bench
cargo test -p ring_bench --all-features
```

**Prediction.** All three green, because no test writes a literal candidate
count.

**Result (2026-08-28): green in all three configurations.**
Prediction confirmed. It is the least interesting stage here, which is the
correct outcome for a stage checking that a deliberate abstraction holds.

---

## B5. Would the suite catch the crate's most expensive defect?

The `reported`-vs-`received` counter mapping was the crate's most expensive
lesson. The original defect fed workload totals through the slot-lifecycle
counters, so 240 refused records read as 240 leaked slots on a 16-slot ring.

**Command.** Reinstate the original mapping, run, revert.

```rust
  stats.record_claim( reported as u64 );      // PROBE ONLY — was `received`
  stats.record_publish( reported as u64 );
  stats.record_consume( reported as u64 );
  stats.record_drop( workload.config().overflow(), ( offered - reported ) as u64 );
```

**Prediction.** Caught by `a_dropnewest_ring_reports_successes_it_did_not_keep`,
which runs `ContractRing` on the cramped workload where `reported` is 256 and
`received` is 16.

**Result (2026-08-28): NOT CAUGHT. Every test passed with the defect
reinstated.**

```
test result: ok. 18 passed; 0 failed
test result: ok. 3 passed; 0 failed   (doctests)
```

**The crate had 100% line coverage and zero defence against its own headline
defect.** There were two reasons:

1. `the_counters_are_the_runs_own_totals`, the test named after the job, runs
   `Candidate::MutexQueue`, whose `reported` and `received` are both 16 on the
   cramped workload. The mapping it exists to pin is invisible to it, because
   the two candidate values are equal.
2. `a_dropnewest_ring_reports_successes_it_did_not_keep` uses the one candidate
   where they differ, but asserts on `Outcome`'s fields and never on
   `outcome.stats()`. The mutation changes only the `RingStats` mapping, so it
   passes straight through.

**Coverage measures which lines ran, not which values were checked.** Both lines
executed in both tests; neither test looked at the number they produced.

**Fix applied.** `a_dropnewest_ring_reports_successes_it_did_not_keep` now
asserts the four counters and `in_flight()` on `ContractRing`, the candidate
where the two counts diverge.

**Re-probe (2026-08-28): red, one failure, the intended one.**

```
FAIL  a_dropnewest_ring_reports_successes_it_did_not_keep
Summary  18 tests run: 17 passed, 1 failed
```

**Re-probe (2026-10-03), under the `Fail` default: red.** Under `Fail` the
cramped workload's `reported` equals `received` for every candidate, so the
mapping would be invisible there. The detector now runs on `cramped_dropping`,
which sets `DropNewest` itself. The same mutation, rewritten for the current
formatting, fails two tests, as it does on the commit before the default moved:

```
test a_dropnewest_ring_reports_successes_it_did_not_keep ... FAILED
test the_record_drop_input_is_guarded_before_the_subtraction_runs ... FAILED
test result: FAILED. 29 passed; 2 failed
```

---

## Run Record

**2026-08-28.** All five stages ran against `ring_bench` at 18 behavioural
tests, 3 doctests, 100% line coverage over 298 lines.

| Stage | Prediction | Outcome |
|---|---|---|
| B1 | 3 refusals at 4 producers, 6 rows elsewhere | **Wrong.** 4 refusals; also found a miscount in `pitfall/001` and an undocumented total stall in `tls_over_ring` |
| B2 | 2 failures | **Wrong, in the safe direction.** 5 behavioural + 1 doctest |
| B3 | ≥2 distinct winners | Confirmed. 2 winners, and the variance figures justify `decisions/002` quantitatively |
| B4 | Green in all feature configurations | Confirmed |
| B5 | Defect caught by the `DropNewest` test | **Wrong.** Not caught at all; suite hole closed and re-probed red |

**Three of five predictions were wrong. B5 changed the test suite; B1 changed
three documents.** The two stages only a human could run, reading the table and
asking the suite a question about itself, are the two that found everything.

After this round the suite is green at 18 + 3 in all three feature
configurations, B2 and B5 are both confirmed red under mutation, and the
documentation is corrected against observed output.

**2026-10-03.** B1 and B5 re-run after `ring_core` ADR 004 moved the default
to `Fail`. B1: the cramped `silent` column is 0 on every row. B5: still red,
with its detector pinned to `DropNewest`.
