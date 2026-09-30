# Workaround: The Clock Is the Platform's, and the Tie-Break Is the List

### Scope

- **Purpose**: Record W1 — the crate's only ordering operation reads a clock whose resolution is a property of the operating system, not of anything in this workspace — what the crate does about it, and the one cost the compensation leaves behind.
- **Responsibility**: State the constraint, the compensation, its cost, and the condition under which the entry can be deleted.
- **In Scope**: `std::time::Instant`; `Outcome::write_nanos`; `Comparison::fastest`; what the suite asserts about time and what it does not.
- **Out of Scope**: Why the counters sit outside the timed region (→ [`invariant/002`](../invariant/002_the_counters_are_written_outside_the_clock.md)); why a measurement is a value (→ [`pattern/001`](../pattern/001_the_measurement_is_a_value.md)); W2 (→ [`002`](002_a_feature_that_cannot_be_negated_at_the_use_site.md)).

### The Constraint

`std::time::Instant` is monotonic and its *resolution* is whatever the platform
supplies. Rust documents no lower bound; on this workspace's host the granularity
is good, on another it may be coarse enough that two write phases differing by a
real margin read as identical. Nothing in this workspace can change that, and no
dependency edge, feature flag or configuration value affects it.

This crate reads that clock six times and derives one ordering from it —
`Comparison::fastest`, a `min_by_key` over `write_nanos`. That single ordering
is the crate's entire output as a *recommendation*; everything else it produces
is a count.

| | |
|---|---|
| **Constraint** | `Instant`'s resolution is the platform's, and the crate's one ordering is a comparison of two readings of it |
| **Compensation** | The crate never asserts on elapsed time. `write_nanos` is reported and is excluded from every assertion in the suite; eligibility for "fastest" is decided by record accounting, which is deterministic |
| **Cost** | A tie is resolved silently by `Candidate::ALL`'s declaration order, and nothing states this |
| **Deleted when** | Never, structurally — unless the crate stops ranking, at which point W1 stops applying rather than being solved |

**The compensation is unusually complete, and that is worth stating before the
cost.** The module doc says it in the crate's own words — "an assertion about
wall-clock time is a flaky test on a shared machine, and a flaky test in a
benchmark harness discredits the measurement it exists to protect" — and
`a_path_that_dropped_records_is_not_eligible_to_be_fastest` carries a doc
comment that names precisely what it is *not* asserting. This is a workaround
that was thought through, not one that accumulated.

### Sources

| File | Relationship |
|------|-----------------|
| [`src/lib.rs`](../../src/lib.rs) | The six clock reads, the one `min_by_key`, and the module doc that rules on both |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/002_the_counters_are_written_outside_the_clock.md`](../invariant/002_the_counters_are_written_outside_the_clock.md) | The other half of keeping the measurement honest — what the timed region excludes |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [`../non_functional_requirement/002_the_harness_is_not_in_the_measurement.md`](../non_functional_requirement/002_the_harness_is_not_in_the_measurement.md) | The requirement this workaround is the platform-side half of |

### Workarounds

| File | Relationship |
|------|--------------|
| [`002_a_feature_that_cannot_be_negated_at_the_use_site.md`](002_a_feature_that_cannot_be_negated_at_the_use_site.md) | W2, which owns the second copy of the list this entry's cost depends on |

### Tests

| Test | Relationship |
|------|--------------|
| `a_path_that_dropped_records_is_not_eligible_to_be_fastest` | The compensation, asserted — the eligible *set*, never the winner |
| `a_comparison_of_the_same_workload_repeats_its_counts` | What is deterministic across runs, which is everything except the clock |
| `the_report_names_every_candidate_and_every_refusal` | Asserts the `fastest lossless:` line exists, not what follows it |

### BN49 — Nineteen Mentions of the Elapsed Time, and Not One Assertion on It

The compensation is honoured exactly:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
printf '  write_nanos in src        : %s\n' "$( command grep -c 'write_nanos' src/lib.rs )"
printf '  write_nanos in tests      : %s\n' "$( command grep -c 'write_nanos' tests/bench_test.rs )"
printf '  assertions naming it      : %s\n' "$( command grep -chE 'assert.*write_nanos' src/lib.rs tests/bench_test.rs | paste -sd+ - | bc )"
printf '  Instant::now calls        : %s\n' "$( command grep -c 'Instant::now' src/lib.rs )"
echo '  -- every use of it in the suite --'
command grep 'write_nanos' tests/bench_test.rs | sed -E 's/^(.{0,96}).*/\1/'
echo '  -- and the one ordering it feeds --'
command grep 'min_by_key\|max_by_key' src/lib.rs
```

Live output:

```
  write_nanos in src        : 20
  write_nanos in tests      : 3
  assertions naming it      : 0
  Instant::now calls        : 6
  -- every use of it in the suite --
//! - `write_nanos()` can return a constant. Its own doc comment says never to
        let _elapsed = outcome.write_nanos();
        let _ = outcome.write_nanos();
  -- and the one ordering it feeds --
    /// [`Candidate::ALL`], because `min_by_key` returns the first minimum and the
    /// Pitfall: `min_by_key` has a documented tie behaviour and inherits its
            .min_by_key(|outcome| outcome.write_nanos)
```

Three mentions in the suite: one in a module comment stating the rule, and two
bindings to `_elapsed` and `_` — the value is fetched, discarded, and never
compared. That is the whole of the test suite's relationship with time.

**The compensation is not free, and the price is paid one level up.** Ranking
is what a benchmark harness is *for*, and this one has ruled that its ranking
cannot be tested. So `Comparison::fastest` is a public method, returned by
`report()` on a line beginning `fastest lossless:`, and nothing anywhere
verifies it beyond checking that a lossless candidate is eligible and a lossy
one is not. The suite tests the filter and leaves the `min_by_key` untested by
construction.

That is the right trade — a flaky ranking assertion would be worse — but it
means the crate's headline output has exactly the test coverage of a value the
crate has declared untestable, and a reader who takes the `fastest lossless:`
line at face value is trusting an unasserted comparison of two platform clock
readings.

### BN50 — A Tie Hands the Verdict to Whichever Candidate Is Declared First, Which Is the Baseline

`min_by_key` returns the *first* minimum, and the iteration order is
`Candidate::ALL`:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the ordering, and what it iterates --'
command grep -B4 'min_by_key' src/lib.rs | sed -E 's/^(.{0,96}).*/\1/'
echo '  -- the order the outcomes were pushed in --'
awk '/pub const ALL/, /\];/' src/lib.rs | command grep -E 'cfg|Self::' | sed 's/^/    /'
echo '  -- and whether any doc or test mentions a tie --'
printf '    mentions of "tie" in src   : %s\n' "$( command grep -ciE '\btie(s|-break)?\b' src/lib.rs )"
printf '    mentions of "tie" in tests : %s\n' "$( command grep -ciE '\btie(s|-break)?\b' tests/bench_test.rs )"
```

Live output:

```
  -- the ordering, and what it iterates --
  ///
  /// # Ties
  ///
  /// Fix(BN50): a tie goes to whichever candidate appears **first** in
  /// [`Candidate::ALL`], because `min_by_key` returns the first minimum and the
--
  /// must compare `write_nanos()` across the eligible outcomes itself.
  ///
  /// Root cause: `Candidate::ALL`'s declaration order silently doubles as this
  /// method's tie-break rule.
  /// Pitfall: `min_by_key` has a documented tie behaviour and inherits its
--
  {
    self.outcomes
      .iter()
      .filter( | outcome | outcome.is_lossless() )
      .min_by_key( | outcome | outcome.write_nanos )
  -- the order the outcomes were pushed in --
        Self::MutexQueue,
        Self::ContractRing,
        Self::TlsOverRing,
        Self::DirectSpsc,
        Self::DirectMpsc,
        Self::OffTheShelf,
        Self::MutexQueue,
        Self::ContractRing,
        Self::TlsOverRing,
        Self::DirectSpsc,
        Self::DirectMpsc,
  -- and whether any doc or test mentions a tie --
    mentions of "tie" in src   : 8
    mentions of "tie" in tests : 9
```

`Comparison::run` iterates `Candidate::ALL` in order and pushes each success
onto `outcomes`, so `outcomes` is in declaration order. `fastest` filters that
vector and takes `min_by_key`, which the standard library documents as returning
the first element when several are equally minimum. So when two lossless
candidates record the same `write_nanos`, the winner is whichever appears
earlier in `Candidate::ALL`.

`MutexQueue` is declared first.

**This is the failure mode W1 makes reachable, and it is the worst-shaped one
available.** A clock too coarse to separate two fast paths does not produce a
noisy result or an obviously wrong one — it produces a clean, plausible
`fastest lossless: mutex_queue`, which is precisely the answer a reader would
believe, and which is the baseline the whole comparison exists to be measured
against. Coarser clock, more ties, more often the baseline "wins".

**The crate had reasoned about ties, once, and about a different tie.** When
this was written the census returned six mentions, and every one of them was one
of two other senses — the counts above read higher now only because the fix
below put the third sense into words. `WorkloadError::ZeroProducers`
exists because a zero-record workload "would tie at zero" — a degenerate tie,
closed by refusing the workload outright, and
`a_workload_refuses_every_degenerate_dimension` is that refusal asserted. The
other four were `the_flush_relay_is_unreachable_while_the_batch_ties_the_buffer`,
where "ties" means one scalar bound to two places.

So the tie that decides the verdict is the third sense, and it was the one with
no name anywhere. The degenerate case was found precisely because it made *every*
candidate tie, which is conspicuous; a tie between two of five is not, and the
work of closing the loud case is what leaves the quiet one looking handled.
`fastest`'s doc comment described only the eligibility filter, and the two
hand-maintained copies of `Candidate::ALL` (→ [`002`](002_a_feature_that_cannot_be_negated_at_the_use_site.md))
were therefore two hand-maintained copies of a rule that had never been written
down.

The general shape: **a compensation that removes a value from every assertion
also removes it from every review**, and a rule that survives only in the
declaration order of a list is a rule nobody will find when they reorder the
list.

**Disposition:** applied — the rule is now written down in the three places a
reader can arrive from. `Comparison::fastest` gained a `# Ties` section stating
that a tie goes to whichever candidate `Candidate::ALL` declares first, naming
`MutexQueue` — the baseline — as the one that therefore wins by default, and
telling a caller who needs a different rule to compare `write_nanos()` across the
eligible outcomes itself. Both `cfg` arms of `Candidate::ALL` now say the order
is load-bearing rather than cosmetic. The rule is also *checked*, not only
stated: `the_candidate_list_matches_a_copy_written_outside_the_declaration` pins
the five common names and their order against a copy written in the suite, so
reordering either arm fails a test instead of silently re-electing the tie
winner; the test was proven able to fail by swapping two entries and watching it
go red. What this does not buy: nothing observes that a tie actually happened at
runtime. `fastest` still returns one winner with no signal that the margin was
zero, so a coarse-clock run where two candidates land on the same nanosecond
still reads as a clean verdict for the first-declared one — surfacing that means
changing the return type, which is a larger change than this finding names. Now
prints: `mentions of "tie" in src   : 8`
