# Pitfall: A Counter Inside the Timed Region Measures Itself

### Scope

- **Purpose**: Record that `ring_stats`'s counters, used the way they are designed to be used, would instrument the exact loop this crate is timing — and that this crate therefore writes every counter once, from totals, after the clock stops.
- **Responsibility**: Name the trap, the failures it produces, the mitigation applied, and what the mitigation costs.
- **In Scope**: Where `RingStats` is written relative to `Instant::elapsed`; what a per-record counter would cost; what the totals-only mapping gives up.
- **Out of Scope**: Whether `RingStats` should use relaxed atomics, which is `ring_stats`' decision; the counters' own correctness, which that crate's suite covers.

### Trap

**`ring_stats` exists to make a result readable:**

> Per-ring counters for what actually happened: items claimed, items published,
> items dropped and under which policy, and nanoseconds spent waiting. **They
> are cheap enough to leave on** […]

**And its natural use is per event** — increment on each claim, each publish,
each drop. That is what a counter is for, and every other consumer in the family
uses it that way. Here it would be a defect, because the events being counted
are the events being timed:

**"Cheap enough to leave on" is a claim about a production write path, and this
crate is not one.** In production the counter's cost is amortised against work
that dwarfs it and, more importantly, is paid *identically* by whatever the
alternative would have been. In a comparison, the same cost is paid differently
by each candidate and is attributed to the candidate rather than to the counter.
The feature's sentence is true and the inference from it — leave them on here
too — is not.

```rust
let started = Instant::now();
for record in workload.records_of( 0 )
{
  if producer.try_push( record ).is_ok()
  {
    stats.record_publish( 1 );   // ← inside the region under measurement
  }
}
let write_nanos = started.elapsed().as_nanos();
```

**An atomic increment per record is not free, and it is not uniformly
expensive across candidates.** It is one relaxed `fetch_add` on an uncontended
cache line for the single-producer paths, and a contended one for
`MutexQueue` and `DirectMpsc` where four threads share it. So the instrumentation
costs *more* on exactly the candidates whose advantage is multi-producer
throughput — the measurement penalises the thing it exists to detect.

**The trap is not that instrumentation has a cost.** Everyone knows that. It is
that the cost is **candidate-dependent and correlates with the property under
comparison**, so it does not wash out as a constant offset. A benchmark with a
uniform overhead still ranks correctly; this one would not.

**And nothing would look wrong.** Every count would be right. The report would
have the same columns, the same totals, and a ranking assembled partly from how
much contention the harness's own counter suffered.

### Failure

| # | Failure | How it presents |
|---|---------|-----------------|
| C1 | A multi-producer candidate is slowed by the shared counter | Its row is slower than the path is. **The correction is invisible because the number is plausible** |
| C2 | The counter's contention scales with producer count | The gap widens exactly as the workload is scaled up to make the difference visible |
| C3 | A reviewer asks for per-record counters "for detail" | The request is reasonable, the cost is not visible in the diff, and the ranking changes |
| C4 | Counters are recorded outside the clock, as here | No intra-run distribution is available — no "drops per second", no burst profile. **This is the cost of the mitigation, not a failure of it** |
| C5 | A future candidate is added that increments internally | The harness's own discipline does not reach inside a candidate's implementation |

**C1 and C2 are the ones that matter**, and C2 is the sharper of the two: the
error grows with precisely the parameter an investigator would increase to make
a difference stand out, so the harness gets *more* wrong the harder it is
pushed.

**C4 is listed as the accepted price.** Writing counters from totals means a run
reports what happened and never when. For this comparison that is sufficient — the
deliverable is a comparison across candidates, not a time series within one — and
the alternative is C1. If an intra-run profile is ever needed, it needs a
separate run with instrumentation acknowledged in the report, not a quieter
version of this one.

**C5 is real and out of reach.** `ring_core` may increment its own counters; if
it does, that cost is part of the candidate and belongs in the measurement. The
distinction the mitigation preserves is between *the path's* instrumentation and
*the harness's*.

### Mitigation

**What does not work:**

| Attempt | Why it fails |
|---------|--------------|
| Use relaxed ordering and call it free | Relaxed is cheap, not free, and contention on a shared line is a cache-coherence cost that ordering does not remove |
| Give each producer its own counter and sum afterwards, *incrementing it once per record* | Removes contention, but still keeps a `fetch_add` per record on every path — the cost this mitigation exists to remove. A single harness-owned atomic incremented once *per thread* at close, not per record, is a different shape and is what the two threaded runners actually do (`reported` in `run_direct_mpsc`/`run_mutex_queue`, → BN43) |
| Subtract an estimated overhead | Requires knowing the per-candidate cost, which is the thing being measured |
| Time only part of the loop | Splitting the timed region around the counter is more bookkeeping inside the loop, not less |

**What works:**

1. **Write every counter once, from totals, after `elapsed()`.** The timed
   region contains the publication protocol and nothing else. The counters are
   populated afterwards from three integers the runner already returned, so the
   cost is four calls per *run* rather than four per record.
2. **The totals are the drained counts, not the reported ones.** A record that
   came back out claimed exactly one slot and published it; a record discarded
   under `DropNewest` claimed none. That mapping is what keeps `in_flight()` at
   zero, so a nonzero reading means what `ring_stats` says it means — a slot
   taken and abandoned — rather than "the workload offered more than the ring
   could hold", which is not a leak and is already reported as `dropped`.
   → [`pitfall/003`](003_ok_is_not_kept_and_the_verdict_inverts.md).
3. **The suite asserts the zero.** `the_counters_are_the_runs_own_totals` checks
   `in_flight() == 0` and `wait_nanos() == 0` explicitly, so both omissions are
   deliberate and recorded rather than forgotten.

**The second mitigation was arrived at by getting it wrong.** The first mapping
fed workload numbers to the lifecycle counters — `claimed = offered`,
`published = accepted` — and 240 refused records on a 16-slot ring read as 240
leaked slots. `in_flight()` is `claimed - published`, a *ring-slot* concept; the
harness was speaking a workload vocabulary into it. The test failure was the
only thing that caught it, and it caught it because the assertion was written
against `ring_stats`' own definition rather than against what the harness
intended.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_workload_through_six_runners.md](../algorithm/001_one_workload_through_six_runners.md) | Where the clock starts and stops relative to everything else |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_the_counters_are_written_outside_the_clock.md](../invariant/002_the_counters_are_written_outside_the_clock.md) | The same rule stated as a property to hold rather than a trap to avoid |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_a_description_to_a_verdict.md](../lifecycle/001_from_a_description_to_a_verdict.md) | The six phases, one of which is timed |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_harness_is_not_in_the_measurement.md](../non_functional_requirement/002_the_harness_is_not_in_the_measurement.md) | The unstated requirement this crate has to hold anyway |

### Pitfalls

| File | Relationship |
|------|--------------|
| [003_ok_is_not_kept_and_the_verdict_inverts.md](003_ok_is_not_kept_and_the_verdict_inverts.md) | Mitigation 2's other half — which count the totals are taken from |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_stats/src/lib.rs`](../../../ring_stats/src/lib.rs) | `RingStats`, and `in_flight()`'s definition as `claimed - published` |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `the_counters_are_the_runs_own_totals` asserts the totals mapping, the `in_flight` zero, and the `wait_nanos` zero. It is also the test that failed on the first, wrong mapping described above |

### BN42 — One of the Two Deliberate Omissions Is an Assertion Nothing in the Crate Can Fail

Mitigation 3 presents a matched pair:

> **The suite asserts the zero.** `the_counters_are_the_runs_own_totals` checks
> `in_flight() == 0` and `wait_nanos() == 0` explicitly, so both omissions are
> deliberate and recorded rather than forgotten.

```sh
cd "$(git rev-parse --show-toplevel)"
printf -- '--- who could ever make wait_nanos nonzero ---\n'
printf '  record_wait calls in ring_bench/src   : %s\n' "$( command grep -c 'record_wait' ring_bench/src/lib.rs )"
printf '  record_wait calls in its suite        : %s\n' "$( command grep -c 'record_wait' ring_bench/tests/bench_test.rs )"
printf '  ring_*/src callers, less its definition : %s\n' \
  "$( command grep -rl 'record_wait' ring_*/src/ | command grep -v ring_stats | wc -l )"
printf -- '--- the two assertions, and what the test runs ---\n'
awk '/fn the_counters_are_the_runs_own_totals/, /^\}/' ring_bench/tests/bench_test.rs \
  | command grep -E 'let outcome = run|in_flight|wait_nanos' | sed 's/^/  /'
printf '  candidate variants exercised : %s\n' \
  "$( awk '/fn the_counters_are_the_runs_own_totals/,/^\}/' ring_bench/tests/bench_test.rs \
      | command grep -oE 'Candidate::[A-Za-z]+' | sort -u | tr '\n' ' ' )"
printf '  entries in Candidate::ALL    : %s\n' \
  "$( awk '/pub const ALL/{ inb = 1; c = 0 } inb && /Self::/{ c++ } inb && /\];/{ printf "%s ", c; inb = 0 }' \
      ring_bench/src/lib.rs )"
```

Live output:

```
--- who could ever make wait_nanos nonzero ---
  record_wait calls in ring_bench/src   : 0
  record_wait calls in its suite        : 0
  ring_*/src callers, less its definition : 0
--- the two assertions, and what the test runs ---
      let outcome = run(Candidate::MutexQueue, &workload).unwrap();
      assert_eq!(stats.in_flight(), 0);
      assert_eq!(stats.wait_nanos(), 0);
  candidate variants exercised : Candidate::MutexQueue 
  entries in Candidate::ALL    : 6 5 
```

The two assertions are not the same kind of thing.

`in_flight() == 0` is load-bearing, and the document says so two paragraphs
later: it is the assertion that failed on the first, wrong mapping and forced the
totals to be taken from the drained counts. `wait_nanos() == 0` cannot fail.
`record_wait` is the only way that counter becomes nonzero, and it is called
nowhere in this crate, nowhere in its suite, and by no crate under
`ring_*` other than `ring_stats`, which defines it. The value is
structurally zero for every candidate under every workload, so the assertion
holds for reasons that have nothing to do with what the comment above it says it
is guarding — *"no candidate blocks, so there is no wait to time."* That
sentence would still be the stated reason if a candidate did block, because the
assertion would still pass.

The live guard is also narrower than the sentence implies. The test runs one
candidate — `MutexQueue`, on the cramped fixture — so `in_flight() == 0` is
checked for one of the five or six paths, and the totals mapping it protects is
the same code for all of them only because `run` writes the counters once,
outside the `match`.

**The general shape:** an assertion written to keep an omission *deliberate*
records the intent perfectly and tests nothing, because the omission it names is
enforced somewhere the assertion cannot see — here, by the absence of a call
site in another crate. Pairing it in one sentence with an assertion that did
catch a real defect makes both look equally load-bearing.


### BN43 — The Design the Rejected-Attempts Table Rules Out Is the One That Shipped

The Mitigation section opens with four attempts that do not work. The second:

> Give each producer its own counter and sum afterwards | Removes contention,
> keeps a `fetch_add` per record on every path, and **adds per-thread state the
> candidates do not otherwise have — the harness's own bookkeeping becomes part
> of what is compared**

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
printf -- '--- the harness-owned atomics ---\n'
command grep -n 'AtomicUsize::new' src/lib.rs | sed 's/^/  /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
printf -- '--- run_direct_mpsc, in order, relative to its own first line ---\n'
awk '/^fn run_direct_mpsc/, /^\}/' src/lib.rs \
  | command grep -nE 'Instant::now|thread::scope|scope\.spawn|for record|fetch_add|elapsed|let taken' \
  | sed -E 's/^([0-9]+:)\s*/  \1 /' | sed -E 's/^(.{0,92}).*/\1/' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
printf -- '--- and run_mutex_queue ---\n'
awk '/^fn run_mutex_queue/, /^\}/' src/lib.rs \
  | command grep -nE 'Instant::now|thread::scope|scope\.spawn|for record|fetch_add|elapsed|let taken' \
  | sed -E 's/^([0-9]+:)\s*/  \1 /' | sed -E 's/^(.{0,92}).*/\1/' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
--- the harness-owned atomics ---
    let reported = AtomicUsize::new( 0 );
    let reported = AtomicUsize::new( 0 );
--- run_direct_mpsc, in order, relative to its own first line ---
   let started = Instant::now();
   std::thread::scope
   scope.spawn
   for record in workload.records_of( index )
   reported.fetch_add( taken, Ordering::Relaxed );
   let write_nanos = started.elapsed().as_nanos();
--- and run_mutex_queue ---
   let started = Instant::now();
   std::thread::scope
   scope.spawn
   for record in workload.records_of( index )
   reported.fetch_add( taken, Ordering::Relaxed );
   let write_nanos = started.elapsed().as_nanos();
```

`reported` is a shared `AtomicUsize`, one per run, written by every producer
thread from inside the timed region. That is a counter the candidates do not
otherwise have, holding harness bookkeeping, inside what is being compared — the
three clauses of the rejection, met.

**The quantitative half of the objection does not reach it, and the document has
no way to say so.** The rejected design keeps *"a `fetch_add` per record"*;
`reported` accumulates into a plain local and does one relaxed `fetch_add` per
thread at close, so a four-producer run pays four increments rather than 1024.
That is the difference between a real hazard and a rounding error, and it is
exactly the distinction the rest of this document is built on — C1 and C2 matter
because the cost *scales with the producer count and the record count*. Four
increments do not scale with the record count.

So the shipped code is defensible and the table is not. Written as an absolute
prohibition on per-thread harness state, it forbids `reported`; the reasoning
underneath it forbids only per-record state, which is what the crate avoided.
The two threaded candidates — the ones the trap section names as the vulnerable
pair — are the only two that carry the construct their own pitfall document
rules out.

**The general shape, and the reason this is worth a row rather than a rewrite:**
a rejected-attempts table states conclusions, and a conclusion outlives the
magnitude argument that produced it. Someone adding a seventh candidate reads
"adds per-thread state" as the rule, sees `reported` in the two existing threaded
runners, and has to decide which one is the mistake.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
# -m1 and no -n: this file is its own subject, so an unbounded match
# also finds this command line and every copy of its own output below,
# and -n re-prefixes a fresh line number onto each earlier pass's output
command grep -m1 'incrementing it once per record' docs/pitfall/002_a_counter_inside_the_timed_region_measures_itself.md
```

Live output:

```
| Give each producer its own counter and sum afterwards, *incrementing it once per record* | Removes contention, but still keeps a `fetch_add` per record on every path — the cost this mitigation exists to remove. A single harness-owned atomic incremented once *per thread* at close, not per record, is a different shape and is what the two threaded runners actually do (`reported` in `run_direct_mpsc`/`run_mutex_queue`, → BN43) |
```

**Disposition:** applied — the rejected-attempts row now names the per-record
rate as the actual objection and discloses the shipped `reported` atomic as
the different, once-per-thread shape the crate actually uses, cross-referencing
this finding. What is not fixed: the row still reads as one attempt rather
than two distinct shapes at different granularities, which would need a table
restructure beyond what a documentation-disposition pass makes on its own.
Now prints: `incrementing it once per record`

