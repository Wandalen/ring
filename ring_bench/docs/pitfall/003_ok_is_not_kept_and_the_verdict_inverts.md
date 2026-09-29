# Pitfall: `Ok` Is Not Kept, and the Verdict Inverts

### Scope

- **Purpose**: Record that under the family's default overflow policy a write API returns success for records it discarded, that this crate's first working version counted those successes, and that the resulting report ranked the candidate which threw the workload away first.
- **Responsibility**: Name the trap, the failures it produces, the mitigation applied, and why the mitigation publishes the gap rather than closing it.
- **In Scope**: `OverflowPolicy::default()`; the difference between `reported` and `received`; why losslessness and ranking are both computed from the drain.
- **Out of Scope**: Whether `DropNewest` is the right default, which is `ring_types`' decision (→ [`ring_types`](../../../ring_types/readme.md)); the delivery-side statement of the same trap, which is [`ring_shutdown/docs/pitfall/002`](../../../ring_shutdown/docs/pitfall/002_ok_does_not_mean_kept_under_drop_newest.md).

### Trap

**The family's documented idiom for describing a ring is a bare constructor:**

```rust
let cfg = RingConfig::new( 16 ).unwrap();
```

**And `OverflowPolicy::default()` is `DropNewest`.** Verify:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -B2 'DropNewest' ring_types/src/policy.rs | head -20
```

Live output:

```
/// assert_eq!( OverflowPolicy::ALL.len(), 3 );
/// assert!( OverflowPolicy::Fail.reports_failure() );
/// assert!( !OverflowPolicy::DropNewest.reports_failure() );
--
  /// Discard the item being published; the ring's contents are untouched.
  #[ default ]
  DropNewest,
--
  /// the `contains` loop beside it pins this array's own shape. Until that loop
  /// existed the doctest below was the crate's only check on the roster's
  /// contents, and it covers one entry of three — dropping `DropNewest` instead
--
  /// assert!( OverflowPolicy::ALL.contains( &OverflowPolicy::DropOldest ) );
  /// ```
  pub const ALL : [ Self; 3 ] = [ Self::DropNewest, Self::DropOldest, Self::Fail ];
--
    {
      Self::Fail => true,
      Self::DropNewest | Self::DropOldest => false,
--
```

The `#[ default ]` attribute sits on the first variant, so every `RingConfig`
that does not name a policy gets the discarding one. On such a ring a `try_push`
into a full buffer **returns `Ok`**. The record is gone; the caller is told it
was taken. That is the policy behaving exactly as specified — `DropNewest`
means the newest record is dropped, and a caller who chose it asked for
precisely this.

**A harness that counts successes therefore counts discards.** This one did.
Its first working version ran the cramped fixture — 256 records into 16 slots —
and reported:

| candidate | offered | accepted | dropped | verdict |
|---|---|---|---|---|
| `contract_ring` | 256 | **256** | **0** | lossless |

**It was also fast, and correctly so.** Discarding is the cheapest thing a queue
can do: no memory traffic, no contention, no back-pressure loop. So the
candidate that kept 6% of the workload had both the best loss figure and the
best time, and `fastest()` would have returned it.

**The failure is not a wrong number in a column.** Every number above is a
faithful count of something real. The failure is that the harness's *output* —
a recommendation about which write path to adopt — pointed at the path that
threw the work away, and pointed at it with evidence.

**`ring_types` already had a name for this, and the first version did not ask.**
The policy enumeration carries two predicates — `reports_failure()` and
`drops_silently()` — and the second is exactly the property that decides whether
an `Ok` is evidence:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'fn drops_silently' -A4 ring_types/src/policy.rs
```

Live output:

```
  pub const fn drops_silently( self ) -> bool
  {
    match self
    {
      Self::DropNewest | Self::DropOldest => true,
```

The vocabulary was one crate away, written before this one existed, and a
harness that had consulted it would have had to decide what to do about a
silently-dropping policy instead of quietly assuming there was none.
**A predicate that answers a question nobody thinks to ask does not prevent the
error it was written for.**

**This is a strictly worse failure than the delivery-side version of the same
trap.** [`ring_shutdown/docs/pitfall/002`](../../../ring_shutdown/docs/pitfall/002_ok_does_not_mean_kept_under_drop_newest.md)
names it as a hazard to a producer that believes its records landed: bad, and
visible downstream when something fails to reconcile. Here nothing fails to
reconcile, because the only artefact is a table. A lost record eventually
announces itself; a wrong recommendation is adopted.

### Failure

| # | Failure | How it presents |
|---|---------|-----------------|
| C1 | A discarding candidate is ranked fastest | The report recommends it. **Nothing in the run looks wrong** |
| C2 | A discarding candidate is ranked lossless | Its `dropped` column reads 0 on a workload that lost 94% of its records |
| C3 | A mixed run under the default policy | Some candidates hand refusals back and some absorb them, so a ranking read off the API's own count is **ordered wrong**, not merely imprecise |
| C4 | The same run under `OverflowPolicy::Fail` | Every candidate agrees, the gap is zero, and the trap is invisible. **A suite whose fixtures all set `Fail` would never see it** |
| C5 | A consumer reads `Outcome::reported` | They get what the API said, which is documented as an upper bound. **This is the failure that does not happen** — the number is published with its meaning attached |

**C3 is the one that makes this a ranking bug rather than a precision bug.** If
every candidate absorbed its drops the error would be a uniform overstatement
and the ordering would survive. Under the default policy the mutex queue checks
its own length and refuses, the direct backends return `Err`, the staged
candidate refuses before it writes, and only the `ring_core`-mediated paths
absorb — so the overstatement lands on a subset, and the subset it lands on is
the one being evaluated.

**C4 is why the suite runs both.** `a_dropnewest_ring_reports_successes_it_did_not_keep`
asserts the gap is open under the default; `a_failing_policy_closes_the_gap_for_every_candidate`
asserts it closes under `Fail`, on the same capacity and the same record count
with one field changed. Either test alone would be consistent with the harness
having no notion of the distinction at all.

### Mitigation

**What does not work:**

| Attempt | Why it fails |
|---------|--------------|
| Set `OverflowPolicy::Fail` in every fixture | C4. Hides the trap rather than measuring it, and the default is what a consumer following the family's documentation will actually get |
| Treat a `DropNewest` run as invalid | The policy is a supported configuration; refusing to measure it means the comparison cannot speak to the configuration most consumers will use |
| Check the ring's length after each push | Instruments the timed region (→ [`pitfall/002`](002_a_counter_inside_the_timed_region_measures_itself.md)), and is not available uniformly across candidates |
| Report `reported` and note the caveat in prose | A note is not graded, and `fastest()` would still read the wrong field |
| Drop `reported` entirely and keep only the drain count | Loses the measurement that distinguishes a path with back-pressure from one without — which is C3's diagnostic |

**What works:**

1. **The drain is the only count treated as truth.** `Outcome::received` is what
   came back out of the consumer after the clock stopped. `is_lossless`,
   `dropped`, and `Comparison::fastest`'s eligibility filter are all computed
   from it, so no judgement this crate makes can be reached through the API's
   own claim.
2. **The gap is published, not closed.** `Outcome::reported` keeps what the write
   API said and `Outcome::silently_discarded` is the difference. Under `Fail` it
   is zero; under `DropNewest` it is the size of the trap. That number is the
   only thing distinguishing a path that applies back-pressure from one that
   absorbs — deleting it would remove a real measurement to avoid a
   misreading.
3. **`Outcome::conserved` was demoted from an assertion to a report.** It was
   written as a correctness check — reported must equal received — and under the
   default policy it is legitimately `false`. It now answers *how* a candidate
   arrived at its `dropped` figure: `true` means the path handed refusals back,
   `false` means it absorbed them. A `false` is a fact about the configuration,
   not a failure of the run.
4. **`Comparison::fastest` returns `None` when nothing was lossless.** The honest
   answer on a cramped workload is that no candidate is eligible, rather than the
   least-bad one.

**The demotion in mitigation 3 is the part worth keeping.** The first fix
attempt was to assert conservation and treat a violation as a bug — which would
have made every default-policy run a test failure, and the family's own idiom
untestable. The distinction that resolves it is between a *property the run must
have* and a *property of the configuration the run is reporting on*, and the
first version of this crate had no vocabulary for the second.

### The Mitigation Was Undefended for Longer Than the Coverage Suggested

**Manual stage B5 reinstated the original defect and every test passed.** 18
behavioural tests, 3 doctests, 100% line coverage over 298 lines — all green
with the `reported`-vs-`received` mapping put back exactly as it had been.

The mutation:

```rust
  stats.record_claim( reported as u64 );   // was `received`
  stats.record_publish( reported as u64 );
  stats.record_consume( reported as u64 );
  stats.record_drop( workload.config().overflow(), ( offered - reported ) as u64 );
```

Two independent reasons it slipped through, and the pair is the lesson:

| Test | Why it missed |
|---|---|
| `the_counters_are_the_runs_own_totals` — *the test named after the job* | Runs `MutexQueue`, where `reported == received == 16` on the cramped fixture. The two values it exists to distinguish are equal, so the mapping is invisible to it |
| `a_dropnewest_ring_reports_successes_it_did_not_keep` | Uses `ContractRing`, the one candidate where they differ — but asserts on `Outcome`'s fields, never on `outcome.stats()`. The mutation touches only the counter mapping |

**Coverage measures which lines ran, not which values were checked.** Both
mutated lines executed under both tests. Neither test looked at what they
produced. A crate can hold 100% line coverage and zero defence against the
defect its own documentation calls its most expensive.

**The fixture choice is the deeper trap.** Picking `MutexQueue` for the counter
test was reasonable — it is the simplest candidate — and it is precisely the
wrong choice, because a test distinguishing two quantities must run on an input
where they differ. That property is not visible in the test's name, its
assertions, or its coverage.

**Closed:** `a_dropnewest_ring_reports_successes_it_did_not_keep` now asserts
all four counters and `in_flight()` on `ContractRing`. Re-probed under the same
mutation: red, one failure, the intended one.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_three_counts_that_are_not_interchangeable.md](../data_structure/002_three_counts_that_are_not_interchangeable.md) | `offered`, `reported`, `received` — the record this trap forced into existence |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_received_never_exceeds_reported_never_exceeds_offered.md](../invariant/001_received_never_exceeds_reported_never_exceeds_offered.md) | The only ordering that holds across every candidate and policy |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md) | Mitigation 4 — why filtering precedes ranking rather than annotating it |

### Pitfalls

| File | Relationship |
|------|--------------|
| [001_the_door_caps_what_the_structure_does_not.md](001_the_door_caps_what_the_structure_does_not.md) | The other trap that produces a plausible table about the wrong thing |
| [002_a_counter_inside_the_timed_region_measures_itself.md](002_a_counter_inside_the_timed_region_measures_itself.md) | Why the length check in the "does not work" table is unavailable |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_measurement_is_a_value.md](../pattern/001_the_measurement_is_a_value.md) | Why the gap is a field on `Outcome` rather than a line of printed text |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_types/src/policy.rs`](../../../ring_types/src/policy.rs) | `#[ default ]` on `DropNewest`, the fact the whole trap rests on |
| [`ring_shutdown/docs/pitfall/002`](../../../ring_shutdown/docs/pitfall/002_ok_does_not_mean_kept_under_drop_newest.md) | The delivery-side statement of the same trap, written before this crate existed |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `a_dropnewest_ring_reports_successes_it_did_not_keep` asserts the open gap and that four other candidates close it on the same workload; `a_failing_policy_closes_the_gap_for_every_candidate` is its control; `a_path_that_dropped_records_is_not_eligible_to_be_fastest` asserts mitigation 4 |

### BN44 — "For Every Candidate" Is a Claim About One Candidate in the Build the Default `cargo test` Compiles

The Failure section explains why the suite runs a pair:

> **C4 is why the suite runs both.** `a_dropnewest_ring_reports_successes_it_did_not_keep`
> asserts the gap is open under the default; `a_failing_policy_closes_the_gap_for_every_candidate`
> asserts it closes under `Fail` […] Either test alone would be consistent with
> the harness having no notion of the distinction at all.

That is right about the pair and wrong about its reach, and the reason is three
paragraphs above it in this same document: *"only the `ring_core`-mediated paths
absorb."*

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
printf -- '--- candidates the DropNewest test already proves conserve, under the default policy ---\n'
awk '/fn a_dropnewest_ring_reports_successes_it_did_not_keep/, /^\}/' tests/bench_test.rs \
  | awk '/for candidate in/, /^    \]/' | command grep -oE 'Candidate::[A-Za-z]+' | sed 's/^/  /'
printf -- '--- what the Fail test asserts, and over what ---\n'
awk '/fn a_failing_policy_closes_the_gap_for_every_candidate/, /^\}/' tests/bench_test.rs \
  | command grep -nE 'for outcome|assert' | sed -E 's/^([0-9]+:)\s*/  \1 /' | sed -E 's/^(.{0,90}).*/\1/' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
printf -- '--- the arithmetic ---\n'
printf '  entries in ALL, crossbeam / not        : %s\n' \
  "$( awk '/pub const ALL/{ inb = 1; c = 0 } inb && /Self::/{ c++ } inb && /\];/{ printf "%s ", c; inb = 0 }' src/lib.rs )"
printf '  already shown conserved under DropNewest : %s\n' \
  "$( awk '/fn a_dropnewest_ring_reports_successes_it_did_not_keep/,/^\}/' tests/bench_test.rs \
      | awk '/for candidate in/, /^    \]/' | command grep -cE 'Candidate::[A-Za-z]+' )"
```

Live output:

```
--- candidates the DropNewest test already proves conserve, under the default policy ---
  Candidate::MutexQueue
  Candidate::DirectSpsc
  Candidate::DirectMpsc
  Candidate::TlsOverRing
--- what the Fail test asserts, and over what ---
   assert!( comparison.conserved(), "every candidate reported exactly what it kept" );
   assert_eq!( comparison.silently_discarded(), 0 );
   assert!( comparison.fastest().is_none(), "16 slots still cannot hold 256 records" );
   for outcome in comparison.outcomes()
   assert_eq!
--- the arithmetic ---
  entries in ALL, crossbeam / not        : 6 5 
  already shown conserved under DropNewest : 4
```

The sibling test names four candidates and asserts `conserved()` and
`silently_discarded() == 0` for each of them **under `DropNewest`** — the policy
under which the gap is supposed to be open. Those four hand their refusals back
regardless of the policy: the mutex queue checks its own length, the two direct
backends never see the policy at all, and the staged candidate refuses before it
writes. Switching to `Fail` cannot change any of their answers.

So `a_failing_policy_closes_the_gap_for_every_candidate` re-asserts four facts
already established under the opposite policy, plus one that is genuinely new —
`contract_ring` — and, only with `--features crossbeam` compiled in,
`off_the_shelf`. The bare `cargo test -p ring_bench` a contributor runs builds
the five-candidate list, so in that build the universally-quantified loop has
exactly **one** load-bearing iteration.

**This does not make the test wrong; it makes the reason given for it wrong.**
"Either test alone would be consistent with the harness having no notion of the
distinction at all" is true, and the pair does close that. What the pair does
not do is what the name promises — the `Fail` half is a one-candidate assertion
in a five-candidate loop, and a change that broke the policy plumbing for
`contract_ring` alone would fail it while a change that broke it for four others
could not, because those four never route a policy decision through the code
under test.

The document's own sentence has the shape of the fix in it. *"Only the
`ring_core`-mediated paths absorb"* is the set the loop should have named
explicitly, the way the sibling test names its four — and naming it would have
made the count, and its dependence on the cargo feature, visible in the test
rather than derivable from a paragraph in a pitfall document.

