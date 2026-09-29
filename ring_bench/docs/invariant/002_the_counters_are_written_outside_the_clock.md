# Invariant: The Counters Are Written Outside the Clock

### Scope

- **Purpose**: State that every `RingStats` write happens after `Instant::elapsed`, from totals, exactly four times per run — and that this is a property to check rather than a habit to maintain.
- **Responsibility**: State the property, the mechanism, and how a violation would be detected.
- **In Scope**: The four counter writes; their placement relative to the timed region; the `in_flight` and `wait_nanos` zeroes that make the placement checkable.
- **Out of Scope**: Why per-record counters would corrupt the comparison (→ [`pitfall/002`](../pitfall/002_a_counter_inside_the_timed_region_measures_itself.md)); `RingStats`' own atomics, which are `ring_stats`'.

### Invariant Statement

**Exactly four counter writes per run, all after the clock stops:**

```rust
let write_nanos = started.elapsed().as_nanos();   // ← the boundary
/* … drain … */
stats.record_claim( received as u64 );
stats.record_publish( received as u64 );
stats.record_consume( received as u64 );
stats.record_drop( workload.config().overflow(), ( offered - received ) as u64 );
```

**The snippet's two halves are two functions, not one region.** The boundary
line is one of six near-identical clock stops, one per runner; the four
`record_*` calls are all in `run`, after a runner has returned and been
destructured. `/* … drain … */` elides that function boundary, not just lines
of drain logic (→ BN24).

**Four per run, never per record.** On the default workload that is four writes
against 1024 records; on a large one it is still four.

#### The mapping is part of the invariant

**All three lifecycle counters take `received`, and only the drop counter takes
a difference.** This is not arbitrary: `RingStats::in_flight()` is
`claimed - published`, a *ring-slot* concept. A record the ring never took never
occupied a slot, so it was never claimed either.

| Counter | Value | Reason |
|---|---|---|
| `claim` | `received` | one slot claimed per record that landed |
| `publish` | `received` | each of those slots was published |
| `consume` | `received` | and drained |
| `drop( policy )` | `offered - received` | everything else, once, under the run's policy |

### Enforcement Mechanism

**The structural guarantee is that the runners cannot violate it.** Every runner
returns `( reported, received, write_nanos )` and holds no `RingStats` at all —
the type is not in scope inside any of the six. `run` constructs the counters
after the runner returns, so a per-record write would require passing the stats
object into a runner, which is a visible signature change rather than a one-line
addition inside a loop.

**Two zeroes make the invariant observable from outside**, which is what turns
it from a convention into a test:

| Assertion | What a failure would mean |
|---|---|
| `stats.in_flight() == 0` | A slot was claimed and never published — either a real leak in a candidate, or the workload-vocabulary mapping reintroduced |
| `stats.wait_nanos() == 0` | Something recorded a wait. No candidate blocks, so there is no wait to time; a nonzero value means a counter was written somewhere unaccounted for |

**The `wait_nanos` zero is asserted precisely because it is an omission.** An
unwritten counter and a counter that stopped being written look identical from
the outside; asserting the zero makes the omission deliberate and recorded
rather than forgotten.

**What is not checked:** that no counter write sits inside the timed region.
Nothing mechanical enforces the placement — the guarantee is the signature shape
above, and a reviewer reading `run`. Recorded as a known gap rather than
implied to be covered; a `#[ deny ]`-style enforcement would need the runners to
be unable to name `RingStats`, which they already cannot, so the remaining
exposure is `run` itself moving a line.

### Violation Consequences

**The timed region is the measurement.** Anything inside it that is not the write
path under test is attributed to the write path under test. A counter is the most
natural thing to put there and the most damaging, because its cost is
*candidate-dependent* — uncontended on the single-producer paths, contended on
`MutexQueue` and `DirectMpsc` — so it does not wash out as a constant offset and
the ranking changes.
→ [`pitfall/002`](../pitfall/002_a_counter_inside_the_timed_region_measures_itself.md).

**The category is broader than counters, and `std::thread::scope` is the other
member.** It sits inside the timed region of exactly `run_mutex_queue` and
`run_direct_mpsc` — the same two candidate-dependent-cost candidates named
above — for the identical reason: thread creation and the scope-exit join are
real work attributed to the write path under test (→ BN23).
[`non_functional_requirement/002`](../non_functional_requirement/002_the_harness_is_not_in_the_measurement.md)'s
"Residue 1" already discloses the `run_mutex_queue` half; `run_direct_mpsc`
has the same structure and was not disclosed until this finding.

**Violating the mapping half is quieter and was actually done.** The first
version mapped workload numbers instead — `claimed = offered`,
`published = reported` — and 240 refused records on a 16-slot ring read as 240
leaked slots. The counters were internally consistent and describing something
that had not happened, which is why the `in_flight() == 0` assertion above is the
one that caught it.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_workload_through_six_runners.md](../algorithm/001_one_workload_through_six_runners.md) | Step 5, and the return triple that keeps the runners unable to violate this |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_three_counts_that_are_not_interchangeable.md](../data_structure/002_three_counts_that_are_not_interchangeable.md) | The counts the mapping is drawn from |

### Invariants

| File | Relationship |
|------|--------------|
| [001_received_never_exceeds_reported_never_exceeds_offered.md](001_received_never_exceeds_reported_never_exceeds_offered.md) | The other property, which keeps `offered - received` from underflowing |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_a_description_to_a_verdict.md](../lifecycle/001_from_a_description_to_a_verdict.md) | The phase boundary this invariant is stated across |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_harness_is_not_in_the_measurement.md](../non_functional_requirement/002_the_harness_is_not_in_the_measurement.md) | The requirement this invariant is one half of |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_stats/src/lib.rs`](../../../ring_stats/src/lib.rs) | `in_flight()`, `wait_nanos()`, and `record_drop`'s policy argument |
| [`ring_stats`](../../../ring_stats/readme.md) | "cheap enough to leave on" is true of a production path, and the inference this invariant refuses |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `the_counters_are_the_runs_own_totals` asserts the full mapping and both zeroes. It is the test that failed on the first, wrong mapping |

### BN23 — Two Runners Spawn Their Threads Inside the Clock, and They Are the Two the Ranking Turns On

Violation Consequences states the rule this instance exists to protect:
"anything inside [the timed region] that is not the write path under test is
attributed to the write path under test", and warns that such a cost is
dangerous specifically when it is *candidate-dependent*. `std::thread::scope` is
inside the timed region of exactly two runners:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- setup, clock start, and what falls between start and stop --'
awk '/Instant::now|started\.elapsed|thread::scope|Vec::with_capacity|Mutex::new|Ring::with_config|Factory\.build|TlsBuffer::|Flusher::new/ \
     { printf "    %4d  %s\n", NR, $0 }' src/lib.rs
echo '  -- and what this document says about a cost like that --'
awk '/^### BN/ { exit } /its cost is/, /the ranking changes/' \
  docs/invariant/002_the_counters_are_written_outside_the_clock.md | sed 's/^/    /'
```

Live output:

```
  -- setup, clock start, and what falls between start and stop --
    1030    let queue = Mutex::new( VecDeque::< Record >::with_capacity( capacity ) );
    1033    let started = Instant::now();
    1034    std::thread::scope
    1046              let mut staged = Vec::with_capacity( workload.batch() );
    1063    let write_nanos = started.elapsed().as_nanos();
    1078    let started = Instant::now();
    1088    let write_nanos = started.elapsed().as_nanos();
    1114    let buffer = TlsBuffer::< Record >::with_capacity( workload.batch() );
    1115    let mut flusher = Flusher::new( buffer, producer, FlushPolicy::OnBatch( workload.batch() ) )
    1118    let started = Instant::now();
    1140    let write_nanos = started.elapsed().as_nanos();
    1161      ring_spsc::Ring::with_config( &workload.config() );
    1164    let started = Instant::now();
    1173    let write_nanos = started.elapsed().as_nanos();
    1192      ring_mpsc::Ring::with_config( &workload.config() );
    1197    let started = Instant::now();
    1198    std::thread::scope
    1223    let write_nanos = started.elapsed().as_nanos();
    1248    let started = Instant::now();
    1258    let write_nanos = started.elapsed().as_nanos();
  -- and what this document says about a cost like that --
    natural thing to put there and the most damaging, because its cost is
    *candidate-dependent* — uncontended on the single-producer paths, contended on
    `MutexQueue` and `DirectMpsc` — so it does not wash out as a constant offset and
    the ranking changes.
```

Every runner builds its structure before starting the clock — `Mutex::new` at
1030, `Factory.build` at 1072, `Ring::with_config` at 1161 and 1192, the `TlsBuffer`
and `Flusher` at 1114–1115 — so construction is correctly excluded. Thread
creation is not. `run_mutex_queue` starts its clock at 1033 and enters
`std::thread::scope` at 1034; `run_direct_mpsc` starts at 1197 and enters at 1198.
The scope's exit joins every thread, and the join is before `elapsed` too.

**Those two runners are `MutexQueue` and `DirectMpsc`** — the baseline every
other candidate is measured against, and the candidate
[`decisions/002`](../decisions/002_no_test_asserts_an_ordering.md) records as
the slowest row in the family's only recorded timing table. The four
single-producer runners spawn nothing and pay none of it. So the excluded cost
is not merely inside the region; it is inside the region for one half of the
comparison and absent from the other, which is the exact shape this document
names as the one that changes the ranking rather than offsetting it.

`run_mutex_queue` adds a second: `Vec::with_capacity( workload.batch() )` at
716, inside the spawned closure and therefore inside the clock, once per
producer. The staging vector is arguably part of that candidate's write path —
that is a judgement call. The thread is not.

**Half of this is already disclosed, and the disclosure is what makes the other
half a finding.**
[`non_functional_requirement/002`](../non_functional_requirement/002_the_harness_is_not_in_the_measurement.md)
names "Residue 1 — the mutex candidate times its own thread scope", argues the
inclusion is deliberate, and draws the right conclusion for a reader: the mutex
candidate's one-producer number is a floor rather than a like-for-like. So the
`run_mutex_queue` case is known.

`run_direct_mpsc` is not. That document's residue section names it zero times,
and its clock covers the same `std::thread::scope` for the same reason. The
argument transfers verbatim — at four producers the parallel machinery *is* part
of what the candidate costs — but the consequence transfers too, and the
consequence is the one that matters here: `direct_mpsc`'s one-producer number
also contains a thread create/join, and `direct_mpsc` is the candidate
[`decisions/002`](../decisions/002_no_test_asserts_an_ordering.md) reports as the
slowest lock-free path in the family's only recorded timing table.

**This document's mechanism section is about `RingStats` and is correct.** The
structural guarantee it describes — runners cannot name the counters, so a
per-record write would need a signature change — holds exactly as written. What
is missing is symmetry: the rule is stated over a category ("anything inside it
that is not the write path"), one member was found and disclosed elsewhere, and
the second instance of that same member was not.

The general shape: **finding one instance of a category feels like handling the
category.** The mutex candidate was examined because it is the baseline; the
second threaded runner has the identical structure and was not examined, because
nothing turned the first finding back into a search.

```sh
cd "$(git rev-parse --show-toplevel)"
# -m1 and no -n: this file is its own subject, so an unbounded match
# also finds this command line and every copy of its own output below,
# and -n re-prefixes a fresh line number onto each earlier pass's output
command grep -m1 'was not disclosed until this finding' ring_bench/docs/invariant/002_the_counters_are_written_outside_the_clock.md
```

Live output:

```
has the same structure and was not disclosed until this finding.
```

**Disposition:** applied — Violation Consequences now names
`std::thread::scope` as a second member of the "anything inside the timed
region" category alongside counters, and states plainly that `run_direct_mpsc`
carries the same undisclosed residue as `run_mutex_queue`'s already-disclosed
one. The category this document warns about now covers both known instances,
not one.
Now prints: `was not disclosed until this finding`

### BN24 — The Boundary Shown as One Line Is Six Lines, in Six Functions, in a Different Function From the Writes

The Invariant Statement presents the property as a single readable region:
`let write_nanos = started.elapsed()` marked "← the boundary", a
`/* … drain … */` elision, then the four `stats.record_*` calls. That is not a
region of code — it is two functions, and the first of them is six:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- every clock stop, with its enclosing function --'
awk '/^(pub )?fn [a-z_]+\(/ { n = $0; sub( /^(pub )?fn /, "", n ); sub( /[(<].*/, "", n ) }
     /started\.elapsed/ { printf "    %4d  %-20s %s\n", NR, n, $0 }' src/lib.rs
echo '  -- every counter write, with its enclosing function --'
awk '/^(pub )?fn [a-z_]+\(/ { n = $0; sub( /^(pub )?fn /, "", n ); sub( /[(<].*/, "", n ) }
     /stats\.record_/ { printf "    %4d  %-20s %s\n", NR, n, $0 }' src/lib.rs
```

Live output:

```
  -- every clock stop, with its enclosing function --
    1063  run_mutex_queue        let write_nanos = started.elapsed().as_nanos();
    1088  run_contract_ring      let write_nanos = started.elapsed().as_nanos();
    1140  run_tls_over_ring      let write_nanos = started.elapsed().as_nanos();
    1173  run_direct_spsc        let write_nanos = started.elapsed().as_nanos();
    1223  run_direct_mpsc        let write_nanos = started.elapsed().as_nanos();
    1258  run_off_the_shelf      let write_nanos = started.elapsed().as_nanos();
  -- every counter write, with its enclosing function --
     944  run                    stats.record_claim( received as u64 );
     945  run                    stats.record_publish( received as u64 );
     946  run                    stats.record_consume( received as u64 );
     961  run                    stats.record_drop( workload.config().overflow(), ( offered - received ) as u64 );
```

There are six clock stops, one per runner, and four counter writes, all in `run`.
Between the boundary line the snippet shows and the four lines under it lie a
runner's return, a tuple destructure, and a drain that has already happened —
`received` is a value `run` was handed, not something it computes after the
elision.

**The elision is doing more work than an elision should.** `/* … drain … */`
reads as "some lines omitted for brevity"; what it omits is a function boundary,
which is the load-bearing fact of the entire invariant. The structural guarantee
this document then argues for — that runners hold no `RingStats` and so cannot
write per-record — is *precisely* the claim that the two halves of the snippet
are in different functions. The snippet undermines its own mechanism section by
drawing them adjacent.

**And the six-versus-one asymmetry is where a violation would actually hide.**
"All after the clock stops" is checkable in `run` by reading eight lines. It is
not checkable for the boundary itself, because there are six boundaries, in six
functions, each of which could move a line independently — and this document's
own "What is not checked" paragraph names the exposure as "`run` itself moving a
line", which is the one of the two places where a reader would notice.

The general shape: **a code snippet that composes two call sites into one region
is a claim about adjacency**, and adjacency is exactly what an invariant about
ordering across a function boundary cannot borrow.

```sh
cd "$(git rev-parse --show-toplevel)"
# -m1 and no -n: this file is its own subject, so an unbounded match
# also finds this command line and every copy of its own output below,
# and -n re-prefixes a fresh line number onto each earlier pass's output
command grep -m1 "two halves are two functions" ring_bench/docs/invariant/002_the_counters_are_written_outside_the_clock.md
```

Live output:

```
**The snippet's two halves are two functions, not one region.** The boundary
```

**Disposition:** applied — a sentence now sits directly under the Invariant
Statement's code block naming the function boundary the `/* … drain … */`
elision hides, so the six-boundaries-in-six-functions fact is stated where the
misleading snippet is, not only in this finding's own section.
Now prints: `two halves are two functions`
