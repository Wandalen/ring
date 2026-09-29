# NFR: The Comparison Is Reproducible and Same-Conditions

### Scope

- **Purpose**: Record this crate's own comparison criterion as a checkable property, with the measurement method for each half.
- **Responsibility**: State the requirement, the threshold, how it is measured, and where it is currently unmet.
- **In Scope**: Identical conditions across candidates; reproducibility of everything except the timing.
- **Out of Scope**: How fast any candidate is (→ [`decisions/002`](../decisions/002_no_test_asserts_an_ordering.md)); what is excluded from the clock (→ [`002`](002_the_harness_is_not_in_the_measurement.md)).

### The Requirement

The comparison criterion is stated directly:

> One workload run against every candidate write path — a mutex-guarded queue, an
> off-the-shelf concurrent queue, the in-house ring, and thread-local staging over
> that ring — under the same producer counts, batch sizes, and payloads.

Two properties, and they are measured differently.

### R1 — Same Conditions

**Property**: every candidate in a comparison is driven by one `Workload` value,
not by six similar ones.

**Threshold**: exact — the same value, not equal values.

**Measurement**: structural rather than statistical. `Comparison::run` takes one
`Workload` by value and passes a `Copy` of it to each runner; there is no
per-candidate configuration path in the API at all. A candidate cannot be given
a different batch size, because nothing accepts one.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'fn run' ring_bench/src/lib.rs
```

Live output:

```
pub fn run( candidate : Candidate, workload : &Workload ) -> Result< Outcome, RunError >
fn run_mutex_queue( workload : &Workload ) -> ( usize, Vec< Record >, u128 )
fn run_contract_ring( workload : &Workload ) -> Result< ( usize, Vec< Record >, u128 ), RunError >
fn run_tls_over_ring( workload : &Workload ) -> Result< ( usize, Vec< Record >, u128 ), RunError >
fn run_direct_spsc( workload : &Workload ) -> ( usize, Vec< Record >, u128 )
fn run_direct_mpsc( workload : &Workload ) -> ( usize, Vec< Record >, u128 )
fn run_off_the_shelf( workload : &Workload ) -> Result< ( usize, Vec< Record >, u128 ), RunError >
  pub fn run( workload : Workload ) -> Self
```

The signatures are `run( candidate, workload )` and `Comparison::run( workload )`
— one workload argument, threaded down.

**Status: MET.** The suite asserts the consequence rather than the mechanism:
`the_report_names_every_candidate_and_every_refusal` shows one header line
describing conditions that apply to every row below it.

### R2 — Reproducible Except the Clock

**Property**: two runs of the same workload produce the same counts, the same
refusals, and the same eligibility verdicts. Only `write_nanos` may differ.

**Threshold**: exact for every count; unconstrained for the duration.

**Measurement**: run twice, compare everything but the timing.

```bash
cd "$(git rev-parse --show-toplevel)"
cargo test -p ring_bench --all-features -- --exact \
  a_comparison_of_the_same_workload_repeats_its_counts
```

**Status: MET for the counts, and deliberately unenforced for the ordering.**
Which candidate is fastest is *not* reproducible in the sense R2 requires, and
no test asserts it — a decision recorded with its full reasoning in
[`decisions/002`](../decisions/002_no_test_asserts_an_ordering.md). Pretending
otherwise would produce a suite that fails on a loaded machine and passes on an
idle one, which is a worse property than not asserting.

### R3 — Every Candidate Reached

**Property**: the comparison covers the four named write paths.

**Threshold**: four named paths, all reached.

**Measurement**: count the rows in a comparison's `outcomes()` at one producer.

**Status: NOT MET above one producer, and the failure is the finding.** Six
candidates cover the four paths — two of them reach the in-house ring by
different routes, which is what makes the door's cost visible — but at four
producers **four of the six refuse**, leaving `mutex_queue` and `direct_mpsc`.
The in-house ring is reachable through the export Contract only at one producer.

So the comparison is complete at one producer and, at four,
degenerates to a baseline-versus-one-backend run. Measured directly:

| Producers | Candidates run (`--features crossbeam`) | Candidates run (default build) | Candidates refused (crossbeam / default) |
|---:|---:|---:|---:|
| 1 | 6 | 5 | 0 / 0 |
| 4 | 2 | 2 | 4 / 3 |

Both builds are routinely run — `verb/test` invokes `cargo nextest run
--all-features`, so the six-candidate row is the graded one, but a bare
`cargo test -p ring_bench` produces the five-candidate row (→ BN33). The
refusal count differs by one at four producers because `OffTheShelf` is one of
the four candidates capped at a single producer.

→ [`pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md)
for why, and
[`decisions/001`](../decisions/001_five_candidates_for_four_named_paths.md) for
the candidate/path mapping.

**This is reported, not hidden**: `Comparison::refusals` carries a line per
unreachable candidate rather than a shorter table
(→ [`pattern/002`](../pattern/002_a_refusal_is_a_row.md)).

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [002_the_harness_is_not_in_the_measurement.md](002_the_harness_is_not_in_the_measurement.md) | The other half — R1 says the same conditions, 002 says the same *unmeasured* conditions |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_five_candidates_for_four_named_paths.md](../decisions/001_five_candidates_for_four_named_paths.md) | Which candidate covers which of R3's named paths |
| [../decisions/002_no_test_asserts_an_ordering.md](../decisions/002_no_test_asserts_an_ordering.md) | Why R2 stops at the counts |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_door_caps_what_the_structure_does_not.md](../pitfall/001_the_door_caps_what_the_structure_does_not.md) | Why R3 is unmet above one producer |

### Sources

| File | Relationship |
|------|--------------|
| [`../readme.md`](../readme.md) | The crate's own stated purpose — one workload, every candidate, identical conditions |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `a_comparison_of_the_same_workload_repeats_its_counts` measures R2; `a_comparison_lists_refusals_rather_than_shortening_the_table` measures R3's shortfall |

### BN33 — R3's Measured Table Is a `--features crossbeam` Measurement, and Does Not Say So

The two-row table under R3 reports 6 candidates run at one producer and 2 run /
4 refused at four. Both rows count `OffTheShelf`, which the default build does
not compile:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the ceiling each candidate declares --'
awk '/fn producer_ceiling/, /^  \}/' src/lib.rs | command grep -E 'Self::.*=>' | sed 's/^/    /'
echo '  -- Candidate::ALL, per build --'
printf '    with    --features crossbeam : %s candidates\n' \
  "$( awk '/pub const ALL/, /\];/' src/lib.rs | command grep -c 'Self::' | awk '{ print $1 - 5 }' )"
printf '    without --features crossbeam : 5 candidates\n'
echo '  -- so the table row for four producers, both ways --'
printf '    crossbeam on  : 2 run, 4 refused\n'
printf '    crossbeam off : 2 run, 3 refused\n'
echo '  -- and the feature is not default --'
awk '/^\[features\]/, /^\[dependencies\]/' Cargo.toml | command grep -vE '^#|^$' | sed 's/^/    /'
```

Live output:

```
  -- the ceiling each candidate declares --
          Self::MutexQueue | Self::DirectMpsc => None,
          Self::ContractRing | Self::TlsOverRing | Self::DirectSpsc => Some( 1 ),
          Self::OffTheShelf => Some( 1 ),
  -- Candidate::ALL, per build --
    with    --features crossbeam : 6 candidates
    without --features crossbeam : 5 candidates
  -- so the table row for four producers, both ways --
    crossbeam on  : 2 run, 4 refused
    crossbeam off : 2 run, 3 refused
  -- and the feature is not default --
    [features]
    crossbeam = [ "ring_factory/crossbeam" ]
    [dependencies]
```

`MutexQueue` and `DirectMpsc` declare `None` and run at any producer count; the
other four declare `Some( 1 )` and refuse at four. That arithmetic is
build-independent. The *totals* are not: six candidates exist only with the
feature on, so the table's `1 | 6 | 0` row is `1 | 5 | 0` in the build a bare
`cargo test -p ring_bench` produces, and `4 | 2 | 4` is `4 | 2 | 3`.

**R3's threshold is "four named paths, all reached", and one of the four is the
off-the-shelf queue** — the path that exists only under the optional feature. So
the requirement's own subject is conditional on a flag the requirement does not
mention, and the status line ("NOT MET above one producer") is derived from a
table measured in the other build.

**The finding is not that the numbers are wrong.** They are right for the build
that produced them, and `verb/test` — the repository's own verification command
— runs `cargo nextest run --all-features`, so that build is the graded one. The
finding is that two builds exist, both are used routinely, they disagree about
how many candidates the comparison has, and no document in this crate states
which build a given count came from.
→ [`decisions/002`](../decisions/002_no_test_asserts_an_ordering.md)'s BN14,
where the same ambiguity reaches a stated conclusion.

The general shape: **a count over a feature-gated set is two counts**, and a
table with one number per row has nowhere to say which one it is.

```sh
cd "$(git rev-parse --show-toplevel)"
# -m1 and no -n: this file is its own subject, so an unbounded match
# also finds this command line and every copy of its own output below,
# and -n re-prefixes a fresh line number onto each earlier pass's output
command grep -m1 'Both builds are routinely run' ring_bench/docs/non_functional_requirement/001_the_comparison_is_reproducible_and_same_conditions.md
```

Live output:

```
Both builds are routinely run — `verb/test` invokes `cargo nextest run
```

**Disposition:** applied — R3's measured table now carries a column per build
instead of one column that was silently the `--features crossbeam` count, and
a sentence states which build each column is and which one `verb/test` grades.
The two counts this finding names are both now on the page.
Now prints: `Both builds are routinely run`

### BN34 — R1 Is Met Exactly as Written, and the Value It Shares Carries Two Batch Sizes

R1's threshold is "exact — the same value, not equal values", and the mechanism
holds: one `Workload` is `Copy`ed to every runner and no per-candidate
configuration path exists. What that shared value contains is two different batch
numbers:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the cramped fixture --'
awk '/^fn cramped\(/, /^\}/' tests/bench_test.rs | sed 's/^/    /'
echo '  -- what with_batch writes, and where --'
awk '/pub fn with_batch/, /^  \}/' src/lib.rs | sed 's/^/    /'
echo '  -- and what RingConfig does to the same argument --'
awk '/fn with_batch/ { on = 1 } on { print "    " $0 } on && /^  \}/ { exit }' ../ring_config/src/lib.rs
echo '  -- so on cramped, the two numbers a candidate can read --'
printf '    workload.batch()          -> 32   (unclamped)\n'
printf '    workload.config().batch() -> 16   (clamped to capacity)\n'
echo '  -- and which runner reads which --'
command grep -nE 'workload\.batch\(\)|workload\.config\(\)' src/lib.rs | sed -E 's/^(.{0,96}).*/\1/' | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  -- the cramped fixture --
    fn cramped() -> Workload
    {
      Workload::new( RingConfig::new( 16 ).unwrap() )
        .with_records_per_producer( 256 )
        .unwrap()
        .with_batch( 32 )
        .unwrap()
    }
  -- what with_batch writes, and where --
      pub fn with_batch( mut self, batch : usize ) -> Result< Self, WorkloadError >
      {
        if batch == 0
        {
          return Err( WorkloadError::ZeroBatch );
        }
    
        self.config = self.config.with_batch( batch );
        Ok( self )
      }
  -- and what RingConfig does to the same argument --
      pub const fn with_batch( mut self, batch : usize ) -> Self
      {
        let capped = if batch > self.capacity.get() { self.capacity.get() } else { batch };
        self.batch = if capped == 0 { 1 } else { capped };
        self
      }
  -- so on cramped, the two numbers a candidate can read --
    workload.batch()          -> 32   (unclamped)
    workload.config().batch() -> 16   (clamped to capacity)
  -- and which runner reads which --
    /// assert_eq!( workload.config().producers(), 4 );
      stats.record_drop( workload.config().overflow(), ( offered - received ) as u64 );
                let mut staged = Vec::with_capacity( workload.batch() );
                  if staged.len() == workload.batch()
        .build::< Record >( workload.config() )
        let end = usize::min( next + workload.batch(), workload.records_per_producer() );
        ring_core::Ring::new( &workload.config() )
      let buffer = TlsBuffer::< Record >::with_capacity( workload.batch() );
      let mut flusher = Flusher::new( buffer, producer, FlushPolicy::OnBatch( workload.batch() 
        ring_spsc::Ring::with_config( &workload.config() );
        ring_mpsc::Ring::with_config( &workload.config() );
        .build_crossbeam::< Record >( workload.config() )
        let end = usize::min( next + workload.batch(), workload.records_per_producer() );
          self.workload.batch(),
          self.workload.config().overflow(),
```

`Workload::with_batch` writes the caller's number to `self.batch` unchecked and
the same number to `self.config`, where `RingConfig::with_batch` caps it at
capacity. On the cramped fixture — capacity 16, batch 32 — the one shared value
therefore holds 32 in one field and 16 in the other.

**Which of the two a candidate feels depends on which the candidate reads.**
`run_contract_ring` slices its publishing loop with `workload.batch()` and builds
its ring from `workload.config()`, so it pushes in runs of 32 into a ring
configured for 16. `run_tls_over_ring` sizes its `TlsBuffer` and its
`FlushPolicy::OnBatch` from `workload.batch()`. `run_mutex_queue` sizes its staging vector and its
commit trigger from `workload.batch()` and never sees a ring at all.
`run_direct_spsc` and `run_direct_mpsc` push one record at a time and read
neither field's batch.

So R1 delivers exactly what it promises — one value, no divergence, nothing
per-candidate — and "the same conditions" still is not what the candidates
experience, because the value is internally inconsistent and the candidates
disagree about which half of it to consult.

**The requirement is not wrong; its threshold is measuring the wrong thing.**
"The same value" is a property of the plumbing. "The same conditions" is a
property of what arrives, and the two coincide only when the value is coherent.
Nothing in R1's structural measurement could detect the difference, because the
measurement's whole design is to look at the argument rather than at the effect.
→ [`data_structure/001`](../data_structure/001_the_workload_description.md)'s BN9
for the field pair itself.

The general shape: **a structural check on a shared parameter proves sharing, not
equivalence** — and where the parameter has more than one field describing the
same quantity, sharing is exactly as strong as the parameter's own internal
agreement, which nothing checks.
