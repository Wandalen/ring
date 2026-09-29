# Lifecycle: From a Description to a Verdict

### Scope

- **Purpose**: Record the seven phases a comparison passes through, which crate owns each, where each can fail, and the single phase the clock covers.
- **Responsibility**: State the phases, their ordering constraints, and the transitions.
- **In Scope**: Description → admission → construction → publication → drain → accounting → verdict.
- **Out of Scope**: The per-candidate state machine (→ [`lifecycle/002`](../lifecycle/002_one_candidate_through_one_run.md)); the internal lifecycle of a ring, which is `ring_core`'s.

### Lifecycle Phases

| # | Phase | Owner | Can refuse | Timed |
|---|-------|-------|:---:|:---:|
| 1 | **Description** — a `Workload` is built from a `RingConfig` | this crate + `ring_config` | ✅ three degenerate dimensions | no |
| 2 | **Admission** — the candidate's producer ceiling is checked | this crate | ✅ `ProducerCeiling` | no |
| 3 | **Construction** — the write path is built from the config | `ring_factory` / `ring_core` / the backends | ✅ for 3 of 6 candidates: `Build`, `Ring` (→ BN29) | no |
| 4 | **Publication** — `offered` records are pushed, in batches | the candidate | — records are dropped, not refused | **yes** |
| 5 | **Drain** — the consumer is emptied | the candidate | — | no |
| 6 | **Accounting** — the counters are written from totals | this crate + `ring_stats` | — | no |
| 7 | **Verdict** — eligibility filter, then minimum time | this crate | — returns `None` rather than refusing | no |

**Phases 1–3 can refuse; phases 4–7 cannot.** Once construction succeeds a run
always produces an `Outcome`, even if that `Outcome` records that nothing
landed. That is deliberate: a candidate which accepted zero records is a
measurement (the staged candidate on a cramped ring is exactly this), and
turning it into an error would delete the finding.

### Phase Transitions

**2 before 3** — a refusal must cost no allocation. A candidate excluded by the
producer count never builds a ring.

**4 before 5, and 5 outside the clock.** The write path is what this comparison
measures; a figure containing the read side measures two things and reports one
number. The drain is nonetheless mandatory, because it is where every count
treated as truth comes from.

**6 after the clock stops, always.** A counter inside phase 4 changes what phase
4 costs, unevenly across candidates.
→ [`invariant/002`](../invariant/002_the_counters_are_written_outside_the_clock.md).

**7 after all six candidates have completed 1–6.** The verdict is comparative;
there is nothing to filter until every candidate has an outcome or a refusal.

### Dependencies

**Four crates own phases, and the split is not where the Contract says it
should be.** Phase 3 for the staged candidate goes through `ring_core` directly
rather than `ring_factory`, because `ring_flush::Flusher::new` takes a
`ring_core::Producer` and nothing on the export Contract produces one. So one
candidate's construction phase sits below the door every other Contract-bound
consumer would use.
→ [`integration/001`](../integration/001_declared_edges_and_the_three_that_were_missing.md).

**The observable consequence is in phase 3's refusal**: `ContractRing` refuses
`DropOldest` as `RunError::Build` and `TlsOverRing` refuses the same policy as
`RunError::Ring`. Two variants for one refusal, and the variant names the route.

### Cleanup Requirements

**Nothing survives a run.** The ring, the producer, the consumer, and the
staging buffer are all local to the runner and dropped before it returns; only
three integers cross the boundary
(→ [`algorithm/001`](../algorithm/001_one_workload_through_six_runners.md)'s
return triple). The `Outcome` owns its `RingStats` and borrows nothing.

**The `Workload` is `Copy` and outlives every run it drives.** `Comparison::run`
takes it by value and stores it, so a report can restate the conditions it was
produced under — which is phase 1's output surviving to phase 7, and the reason
the report's header line can name the overflow policy that explains its own
`silent` column.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_workload_through_six_runners.md](../algorithm/001_one_workload_through_six_runners.md) | Phases 2–6 as steps, with the clock boundary |
| [../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md) | Phase 7 |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_workload_description.md](../data_structure/001_the_workload_description.md) | Phase 1's output, and its three refusals |
| [../data_structure/002_three_counts_that_are_not_interchangeable.md](../data_structure/002_three_counts_that_are_not_interchangeable.md) | Phases 4 and 5's outputs, which are different numbers |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_declared_edges_and_the_three_that_were_missing.md](../integration/001_declared_edges_and_the_three_that_were_missing.md) | Why phase 3 crosses a different boundary for one candidate |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_the_counters_are_written_outside_the_clock.md](../invariant/002_the_counters_are_written_outside_the_clock.md) | Phase 6's placement |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/002_one_candidate_through_one_run.md](../lifecycle/002_one_candidate_through_one_run.md) | The same sequence as states, with the terminal ones distinguished |

### Sources

| File | Relationship |
|------|--------------|
| [`../readme.md`](../readme.md) | The seven-phase measurement this lifecycle documents |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `a_workload_refuses_every_degenerate_dimension` covers phase 1's refusals; `the_contract_door_caps_a_multi_producer_structure_at_one_producer` covers phase 2's; `a_policy_refusal_names_the_crate_that_refused` covers phase 3's two routes |

### BN29 — Phase 3 "Can Refuse" Is a Property of Three Candidates, Not of the Phase

The Lifecycle Phases table gives Construction a single ✅ with two error variants
beside it, which reads as a property every candidate passes through. Half the
runners cannot refuse at all — their signatures say so:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- runner signatures --'
command grep -E '^fn run_[a-z_]+.*->' src/lib.rs | sed -E 's/^(.{0,108}).*/\1/' | sed 's/^/    /'
echo '  -- what each fallible one builds, and what the infallible ones build --'
command grep -E 'Factory\.build|ring_core::Ring::new|Flusher::new|Ring::with_config|Mutex::new' src/lib.rs \
  | sed -E 's/^(.{0,104}).*/\1/' | sed 's/^/    /'
echo '  -- RunError variants --'
awk '/pub enum RunError/, /^\}/' src/lib.rs | command grep -E '^  [A-Z]' | sed 's/^/    /'
```

Live output:

```
  -- runner signatures --
    fn run_mutex_queue( workload : &Workload ) -> ( usize, Vec< Record >, u128 )
    fn run_contract_ring( workload : &Workload ) -> Result< ( usize, Vec< Record >, u128 ), RunError >
    fn run_tls_over_ring( workload : &Workload ) -> Result< ( usize, Vec< Record >, u128 ), RunError >
    fn run_direct_spsc( workload : &Workload ) -> ( usize, Vec< Record >, u128 )
    fn run_direct_mpsc( workload : &Workload ) -> ( usize, Vec< Record >, u128 )
    fn run_off_the_shelf( workload : &Workload ) -> Result< ( usize, Vec< Record >, u128 ), RunError >
  -- what each fallible one builds, and what the infallible ones build --
      let queue = Mutex::new( VecDeque::< Record >::with_capacity( capacity ) );
        ring_core::Ring::new( &workload.config() )
      let mut flusher = Flusher::new( buffer, producer, FlushPolicy::OnBatch( workload.batch() ) )
        ring_spsc::Ring::with_config( &workload.config() );
        ring_mpsc::Ring::with_config( &workload.config() );
  -- RunError variants --
      ProducerCeiling
      Build
      Ring
      Flush
```

`run_mutex_queue`, `run_direct_spsc` and `run_direct_mpsc` return
`( usize, Vec< Record >, u128 )` — no `Result`, no refusal, no phase-3 failure mode. The
other three return `Result< …, RunError >` because they go through a door that
validates: `Factory.build`, `ring_core::Ring::new`, `Flusher::new`.

**The difference is not incidental to this crate — it is one of its findings.**
`ring_spsc::Ring::with_config` and `ring_mpsc::Ring::with_config` read capacity
and ignore `overflow`, so the identical `RingConfig` that `Factory.build`
rejects builds a working ring one level down. That is
[`pitfall/003`](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md)'s
territory and `ring_factory`'s own Pending 8. Phase 3's ✅ flattens it into a
checkmark: the lifecycle model says the phase can refuse, and what is actually
true is that *the door* can refuse and the structure underneath it cannot.

A reader taking the table at face value would expect a `DropOldest` config to be
rejected by every candidate. Three accept it and run.

**The table is not wrong about anything it says.** ✅ is correct — some
construction can refuse. It is a resolution problem: a phase model with one row
per phase has nowhere to put a property that varies per candidate, so the
variation goes to the row's granularity and disappears. That is why
[`lifecycle/002`](002_one_candidate_through_one_run.md) exists, and the
cross-reference in this document's Scope points *out* of the phase table without
noting that the phase table's own ✅ is one of the things that needs it.

```sh
cd "$(git rev-parse --show-toplevel)"
# -m1 and no -n: this file is its own subject, so an unbounded match
# also finds this command line and every copy of its own output below,
# and -n re-prefixes a fresh line number onto each earlier pass's output
command grep -m1 'for 3 of 6 candidates' ring_bench/docs/lifecycle/001_from_a_description_to_a_verdict.md
```

Live output:

```
| 3 | **Construction** — the write path is built from the config | `ring_factory` / `ring_core` / the backends | ✅ for 3 of 6 candidates: `Build`, `Ring` (→ BN29) | no |
```

**Disposition:** applied — the phase table's Construction row now states the
count the checkmark was hiding, so a reader sees "3 of 6" rather than a
uniform ✅ before ever reaching this finding's own section.
Now prints: `for 3 of 6 candidates`

### BN30 — Every Candidate Is Measured Once, In a Fixed Order, With No Warm-Up

Phase 7 is described as comparative — "there is nothing to filter until every
candidate has an outcome or a refusal". What produces those outcomes is a plain
sequential loop:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the loop that produces every number the verdict compares --'
awk '/pub fn run\( workload : Workload \)/, /^  \}/' src/lib.rs | sed 's/^/    /'
echo '  -- warm-up, repetition, or central-tendency anywhere in the crate --'
for w in 'warm' 'warmup' 'median' 'percentile' 'iterations' 'repetition'; do
  printf '    %-11s : %s\n' "$w" "$( cat src/lib.rs tests/*.rs | command grep -ciw "$w" )"
done
echo '  -- and the order the loop follows, in both cfg arms --'
awk '/pub const ALL/, /\];/' src/lib.rs | command grep -E 'Self::|ALL' | sed 's/^/    /'
```

Live output:

```
  -- the loop that produces every number the verdict compares --
      pub fn run( workload : Workload ) -> Self
      {
        let mut outcomes = Vec::new();
        let mut refusals = Vec::new();
    
        for candidate in Candidate::ALL
        {
          match run( *candidate, &workload )
          {
            Ok( outcome ) => outcomes.push( outcome ),
            Err( refusal ) => refusals.push( refusal ),
          }
        }
    
        Self { workload, outcomes, refusals }
      }
  -- warm-up, repetition, or central-tendency anywhere in the crate --
    warm        : 0
    warmup      : 0
    median      : 0
    percentile  : 0
    iterations  : 0
    repetition  : 0
  -- and the order the loop follows, in both cfg arms --
      pub const ALL : &'static [ Self ] =
        Self::MutexQueue,
        Self::ContractRing,
        Self::TlsOverRing,
        Self::DirectSpsc,
        Self::DirectMpsc,
        Self::OffTheShelf,
      pub const ALL : &'static [ Self ] =
        Self::MutexQueue,
        Self::ContractRing,
        Self::TlsOverRing,
        Self::DirectSpsc,
        Self::DirectMpsc,
```

One measurement per candidate, taken in `Candidate::ALL`'s declaration order, in
one process, with nothing run before the first one. There is no warm-up pass, no
repetition, and no median — `write_nanos` is a single reading of a single
execution.

**So position in the sequence is perfectly confounded with candidate identity.**
`MutexQueue` is always measured first and therefore always pays the coldest
caches, the first allocator growth, and whatever frequency the CPU happens to be
at when the process starts; `OffTheShelf` is always measured last. Nothing in
the phase model separates "this candidate is slower" from "this candidate ran
first", because the two are the same fact in every run.

This bears directly on the one claim that reads across all six numbers.
[`non_functional_requirement/001`](../non_functional_requirement/001_the_comparison_is_reproducible_and_same_conditions.md)
scopes itself to "identical conditions across candidates" — same workload, same
capacity, same policy, all true — and the condition that differs is not in its
list because it is not a property of the workload. It is a property of the
phase ordering this document owns.

**The right reading is that the confound is small and the omission is not.**
That same NFR already declares the ordering deliberately unenforced and
[`decisions/002`](../decisions/002_no_test_asserts_an_ordering.md) records the
run-to-run spread that justifies it, so nothing here is being reported as more
reliable than it is. What is missing is the sentence naming sequence position as
a systematic difference rather than a random one — a spread that shrinks with
repetition and a bias that does not are different problems, and only the first
is written down.

The general shape: **"identical conditions" is a claim about the axes somebody
enumerated.** Execution order is not a parameter of the workload, so it does not
appear on a list of workload parameters, and a document organized around that
list cannot notice it.
