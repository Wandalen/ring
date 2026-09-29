# NFR: The Harness Is Not in the Measurement

### Scope

- **Purpose**: Record exactly what the clock covers in each runner, what is deliberately kept outside it, and the two places where harness cost is provably still inside.
- **Responsibility**: State the boundary, the measurement method, and the disclosed residue.
- **In Scope**: The `Instant::now()` / `elapsed()` boundary in all six runners; what sits on each side of it.
- **Out of Scope**: Whether the resulting durations are comparable *as a ranking* (→ [`decisions/002`](../decisions/002_no_test_asserts_an_ordering.md)).

### The Requirement

**A benchmark's number must be of the thing measured, not of the measuring.**
Every quantity this crate adds — counters, accounting, refusal checks, report
strings — has a cost, and any of it inside the timed region is added to the
candidate's score.

**Threshold**: the timed region contains publication and nothing else. Zero
harness bookkeeping that scales with the record count.

### The Boundary, Per Runner

Every runner has the identical shape, and it is checkable by eye:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'Instant::now\|started.elapsed\|let received' ring_bench/src/lib.rs
```

Live output:

```
  let received = drained.len();
  let started = Instant::now();
  let write_nanos = started.elapsed().as_nanos();
  let started = Instant::now();
  let write_nanos = started.elapsed().as_nanos();
  let started = Instant::now();
  let write_nanos = started.elapsed().as_nanos();
  let started = Instant::now();
  let write_nanos = started.elapsed().as_nanos();
  let started = Instant::now();
  let write_nanos = started.elapsed().as_nanos();
  let started = Instant::now();
  let write_nanos = started.elapsed().as_nanos();
```

| Side | What is there |
|---|---|
| **Before `Instant::now()`** | Ring construction, factory `build`, `Flusher::new`, buffer allocation, the producer-ceiling check |
| **Inside** | The publication loop only — `try_push_batch`, `commit_batch`, `append`/`drive` |
| **After `elapsed()`** | The entire drain, `received`, every `RingStats` write, the eligibility filter, the report |

**The drain being outside is the load-bearing part.** `received` — the number
this crate treats as truth — is produced by emptying the consumer, which is read
work. This crate compares *write* paths; a duration containing the
read side measures two things and reports one number.

→ [`lifecycle/001`](../lifecycle/001_from_a_description_to_a_verdict.md) for the
phase ordering this implements.

### Why the Counters Are Written From Totals

**Not one counter is incremented per record.** Each runner returns a triple and
the `RingStats` are written once, afterwards, from those totals.

An atomic increment per record would be the textbook version of this mistake:
it instruments the very loop under measurement, and it does so *unevenly* — the
mutex candidate is already paying for a lock, so a contended atomic costs it
proportionally less than it costs a single-producer ring doing nothing else.
The harness would be compressing the difference it exists to expose.

The cost of writing from totals is that no intra-run distribution exists — no
percentiles, no time series, no "where did the drops cluster". Accepted: the
question this comparison answers is which path to pick, and that is answered by totals.

→ [`invariant/002`](../invariant/002_the_counters_are_written_outside_the_clock.md)
for this stated as an invariant with its own test.

### Status: MET, With Two Disclosed Residues

**Neither is a defect to fix; both are facts a reader of the numbers needs.**

**Residue 1 — the mutex candidate times its own thread scope.** `run_mutex_queue`
wraps `std::thread::scope` inside the timed region, so spawn and join are in its
duration. This is deliberate: at four producers the parallel machinery *is* part
of what that candidate costs, and hoisting the spawn outside the clock would
measure a mutex queue nobody could actually use. But it means the mutex
candidate's one-producer number is not comparable to a single-producer ring's
on equal terms — it contains a thread create/join the ring never pays.

**Consequence for a reader**: the interesting mutex column is the four-producer
one. Its one-producer number is a floor, not a like-for-like.

**Residue 2 — the staged candidate times one branch per record.** `run_tls_over_ring`
evaluates `flusher.append(...).is_err()` and matches on `FlushOutcome` inside
the loop. Both are harness-side control flow that the other candidates do not
execute. The alternative — retrying a rejected flush — was rejected explicitly,
and the source says why at the site: a harness that retries measures its own
retry loop rather than the write path.

So the staged candidate carries a small per-record constant the others don't.
Disclosed rather than removed, because removing it means either not handling
the rejection (wrong counts) or retrying (worse contamination).

### What Is Not Claimed

**That the six durations are therefore comparable as a ranking.** They are
same-conditions (→ [`001`](001_the_comparison_is_reproducible_and_same_conditions.md))
and harness-light, which is necessary and not sufficient. No test in this crate
asserts an ordering, for reasons recorded in full at
[`decisions/002`](../decisions/002_no_test_asserts_an_ordering.md).

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_the_counters_are_written_outside_the_clock.md](../invariant/002_the_counters_are_written_outside_the_clock.md) | The same boundary as an asserted invariant |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_a_description_to_a_verdict.md](../lifecycle/001_from_a_description_to_a_verdict.md) | Which of the seven phases is the timed one |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_the_comparison_is_reproducible_and_same_conditions.md](001_the_comparison_is_reproducible_and_same_conditions.md) | Same conditions; this instance is the *unmeasured* half |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_a_counter_inside_the_timed_region_measures_itself.md](../pitfall/002_a_counter_inside_the_timed_region_measures_itself.md) | The failure mode this requirement forecloses, and why `ring_stats`'s "cheap enough to leave on" claim does not transfer here |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_no_test_asserts_an_ordering.md](../decisions/002_no_test_asserts_an_ordering.md) | Why a clean boundary still does not license an ordering assertion |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_stats`](../../../ring_stats/readme.md) | The counters this crate consumes, and the production-path "cheap enough to leave on" claim that does not transfer to a comparison harness |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `the_counters_are_the_runs_own_totals` asserts the counters equal run totals rather than per-record accumulations; `a_comparison_of_the_same_workload_repeats_its_counts` reads `write_nanos` and asserts nothing about it |

### BN35 — There Is a Third Residue of the Same Kind, in the Candidate the Timing Record Calls Slowest

Residue 1 is `run_mutex_queue` timing its own `std::thread::scope`, disclosed
with the right consequence for a reader. A second runner does the identical
thing and appears in this section zero times:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- thread::scope sites, against each runner clock --'
awk '/^fn run_[a-z_]+\(/ { n = $0; sub( /^fn /, "", n ); sub( /\(.*/, "", n ) }
     /Instant::now|thread::scope|started\.elapsed|fetch_add/ { printf "    %4d  %-18s %s\n", NR, n, $0 }' src/lib.rs
echo '  -- how often the residue section names each threaded runner --'
for c in run_mutex_queue run_direct_mpsc; do
  printf '    %-18s %s\n' "$c" \
    "$( awk '/^### Status: MET/, /^### What Is Not Claimed/' \
        docs/non_functional_requirement/002_the_harness_is_not_in_the_measurement.md \
        | command grep -c "$c" )"
done
```

Live output:

```
  -- thread::scope sites, against each runner clock --
    1033  run_mutex_queue      let started = Instant::now();
    1034  run_mutex_queue      std::thread::scope
    1057  run_mutex_queue                reported.fetch_add( taken, Ordering::Relaxed );
    1063  run_mutex_queue      let write_nanos = started.elapsed().as_nanos();
    1078  run_contract_ring    let started = Instant::now();
    1088  run_contract_ring    let write_nanos = started.elapsed().as_nanos();
    1118  run_tls_over_ring    let started = Instant::now();
    1140  run_tls_over_ring    let write_nanos = started.elapsed().as_nanos();
    1164  run_direct_spsc      let started = Instant::now();
    1173  run_direct_spsc      let write_nanos = started.elapsed().as_nanos();
    1197  run_direct_mpsc      let started = Instant::now();
    1198  run_direct_mpsc      std::thread::scope
    1217  run_direct_mpsc                reported.fetch_add( taken, Ordering::Relaxed );
    1223  run_direct_mpsc      let write_nanos = started.elapsed().as_nanos();
    1248  run_off_the_shelf    let started = Instant::now();
    1258  run_off_the_shelf    let write_nanos = started.elapsed().as_nanos();
  -- how often the residue section names each threaded runner --
    run_mutex_queue    1
    run_direct_mpsc    0
```

`run_direct_mpsc` starts its clock, enters `std::thread::scope`, spawns one
thread per producer, and stops the clock after the scope joins — the same shape
as `run_mutex_queue`, line for line. Residue 1's argument applies to it
unchanged: at four producers the parallel machinery is part of what that
candidate costs, and hoisting the spawn out would measure something nobody could
use.

**Residue 1's *consequence* applies unchanged too, and that is what is missing.**
The disclosure exists so a reader knows the mutex candidate's one-producer
number is a floor rather than a like-for-like. `direct_mpsc`'s one-producer
number is a floor for exactly the same reason, and it is the number
[`decisions/002`](../decisions/002_no_test_asserts_an_ordering.md) reports as
the slowest row in the family's only recorded timing table — where it is read as
evidence about a lock-free path rather than about a thread spawn.

Both threaded runners also close each spawned thread with one
`AtomicUsize::fetch_add` inside the region — lines 1057 and 1217. That is one per
producer, not one per record, so it sits outside this NFR's stated threshold;
it is still one more thing in the region that the Boundary table's "the
publication loop only" does not describe, and it is symmetric between the two,
which the thread scope's *consequence* is not.

The general shape: **a disclosed residue is a finding that stopped at its first
instance.** The mutex candidate was inspected because it is the baseline and its
threading is the point; the second threaded runner has the same structure and
was never revisited, because the disclosure closed the question instead of
opening a search.

### BN36 — Status "MET" and Residue 2 Are Both Correct, and They Contradict Each Other

The threshold is stated in two sentences, and the second is the operative one:
"Zero harness bookkeeping that scales with the record count." Residue 2 is
disclosed in the same document as bookkeeping that scales with the record count:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench/docs/non_functional_requirement
echo '  -- the threshold --'
awk '/^### BN/ { exit } /^\*\*Threshold\*\*: the timed region/, /^$/' 002_the_harness_is_not_in_the_measurement.md | sed 's/^/    /'
echo '  -- the status line --'
awk '/^### BN/ { exit } /^### Status/ { print "    " $0 }' 002_the_harness_is_not_in_the_measurement.md
echo '  -- and residue 2, in its own words --'
awk '/^### BN/ { exit } /^\*\*Residue 2/, /rejected explicitly/' 002_the_harness_is_not_in_the_measurement.md | sed 's/^/    /'
echo '  -- the per-record branch it names --'
cd ../..
awk '/^fn run_tls_over_ring/, /^\}/' src/lib.rs | command grep -E 'append|FlushOutcome|is_err' | sed 's/^/    /'
```

Live output:

```
  -- the threshold --
    **Threshold**: the timed region contains publication and nothing else. Zero
    harness bookkeeping that scales with the record count.
    
  -- the status line --
    ### Status: MET, With Two Disclosed Residues
  -- and residue 2, in its own words --
    **Residue 2 — the staged candidate times one branch per record.** `run_tls_over_ring`
    evaluates `flusher.append(...).is_err()` and matches on `FlushOutcome` inside
    the loop. Both are harness-side control flow that the other candidates do not
    execute. The alternative — retrying a rejected flush — was rejected explicitly,
  -- the per-record branch it names --
        // An `append` refusal means a previous flush was `Rejected` and its records
        if flusher.append( record ).is_err()
        if let FlushOutcome::Flushed { count } = flusher.drive()
      if let FlushOutcome::Flushed { count } = flusher.drain_final()
```

The threshold admits no residue of this kind — "zero", and the qualifier that
selects for it is "scales with the record count". Residue 2 is a branch
evaluated once per record. By the document's own criterion the requirement is
not met; the Status line says MET, and the residue is filed as a disclosure
rather than as the reason the status is qualified.

**Both halves are individually right, which is why this survived.** The residue
really is small, really is unavoidable given the alternatives the source rejects
at the site, and really should be disclosed rather than removed — that reasoning
is sound and is the most careful thing in this document. The threshold really is
the right threshold: per-record harness cost is the failure mode a benchmark
must exclude. What is wrong is only the join between them, which is a single
word.

**The honest status is MET-with-a-bounded-exception**, and writing it that way
costs nothing and changes what a reader does with it: "MET" invites treating the
six durations as harness-free, and the staged candidate's is not, by a constant
this document can name and does not bound. Residue 1's disclosure gives a reader
an action ("the interesting mutex column is the four-producer one"); Residue 2's
gives a reader a fact with no action attached, because the status above it
already said there was nothing to act on.

The general shape: **a threshold written as an absolute and a status written as a
verdict will drift apart the moment a real exception is found**, because the
exception has to be recorded somewhere, and the only place that does not require
revisiting the verdict is underneath it.
