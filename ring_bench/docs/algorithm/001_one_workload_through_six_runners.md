# Algorithm: One Workload Through Six Runners

### Scope

- **Purpose**: Record the shared three-phase shape every candidate is driven by, so that the only difference between two rows of a report is the write path itself.
- **Responsibility**: State the steps, what each phase may and may not contain, and the complexity.
- **In Scope**: Ceiling check, build, timed write, drain, accumulator fold, counter write; what each runner returns; why the drain is outside the clock.
- **Out of Scope**: Choosing among the results (→ [`algorithm/002`](002_the_eligibility_filter_runs_before_the_comparison.md)); the per-candidate construction details, which are in each runner's own source.

### Abstract

Six candidate write paths are driven through one six-step shape, of which only
step 3 is timed and only step 3 may differ. Everything before it — the ceiling
check and the construction — and everything after it — the drain, the
accumulator fold, and the counter writes — is held identical, so that the
difference between two rows of a report is the write path and nothing else.

### Algorithm

**Every candidate goes through the same six steps, and the differences between
runners are confined to step 3.**

| # | Step | Timed? | May differ per candidate |
|---|------|:---:|:---:|
| 1 | Check the producer ceiling; refuse before allocating | no | the ceiling value only |
| 2 | Build the write path from `workload.config()` | no | yes — this is the door |
| 3 | **Publish `offered` records, in batches of `workload.batch()`** | **yes** | yes — this is the measurement |
| 4 | Drain the consumer until it yields nothing | no | yes — the drain API differs |
| 5 | Fold every drained record into the accumulator table | no | no |
| 6 | Write the counters from totals | no | no |

```rust
let started = Instant::now();
/* step 3 only */
let write_nanos = started.elapsed().as_nanos();
```

**Step 1 precedes step 2 so that a refusal costs no allocation.** A candidate
the producer count excludes returns `RunError::ProducerCeiling` carrying both
numbers, and nothing is built.

**Step 2 is outside the clock because construction is not the write path.** A
ring that allocates 4096 slots and a `VecDeque` that reserves the same are being
compared on what they do *afterwards*; folding construction in would report a
one-off cost as if it were per-record throughput, and would do so unevenly —
`Factory.build` walks a config, `VecDeque::with_capacity` does not.

**Step 4 is outside the clock because this crate measures the write path.**
This crate compares write paths; a figure that also contains the read side measures two
things and reports one number. The drain is nonetheless mandatory, and it is
where every count treated as truth comes from
(→ [`pitfall/003`](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md)).

**Step 5 reads `received` records and writes `cells` values — the one place in
the pipeline whose cost is neither O(1) nor bounded by the candidate.** It runs
the same code for all six candidates (`destination_of` plus
`AccumulatorTable::apply`, →
[`data_structure/002`](../data_structure/002_three_counts_that_are_not_interchangeable.md)),
folding `drained` — never `reported`, which by construction may count records
step 5 never sees
(→ [`pitfall/003`](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md)).
Outside the clock for the same reason step 4 is: it is bookkeeping over an
already-completed write, not the write path itself.

**Step 6 is outside the clock because a counter inside it would change what it
counts** (→ [`pitfall/002`](../pitfall/002_a_counter_inside_the_timed_region_measures_itself.md)).

#### Return shape

**Every runner returns the same triple**, which is what keeps steps 5 and 6
uniform:

```rust
fn run_<candidate>( workload : &Workload ) -> ( usize, Vec< Record >, u128 )
//                                              ^reported ^drained    ^write_nanos
```

The two fallible runners — `run_contract_ring` and `run_tls_over_ring` — return
`Result< ( usize, Vec< Record >, u128 ), RunError >` instead, because their
construction step can be refused by a dependency. Nothing else varies: `run`
does the ceiling check, the accumulator fold, the counter write, and the
`Outcome` assembly for all six.

**The second element became the drained records themselves, not their count.**
`received` is now `drained.len()`, computed once in `run`. Every runner that
used to reduce its batch to a bare length inline (`.drain( .. ).count()`,
`consumer.drain().len()`) collects the values instead — step 5's accumulator
fold needs to know *which* records came back, not only how many.

**`reported` and `received` are still separate**, and that remains the single
most consequential shape decision in the crate. A runner that returned one
count would have to decide which, and every runner would decide the cheap way —
the API's own success count, which is available without draining.
→ [`data_structure/002`](../data_structure/002_three_counts_that_are_not_interchangeable.md).

#### Batching

**Every candidate honours `workload.batch()`, and each honours it in its own
vocabulary:**

| Candidate | What a batch means |
|---|---|
| `MutexQueue` | Records staged in a local `Vec`; the lock is taken once per batch |
| `ContractRing`, `OffTheShelf` | `try_push_batch` over a range of that length |
| `TlsOverRing` | The `TlsBuffer`'s capacity *and* the `FlushPolicy::OnBatch` trigger |
| `DirectSpsc`, `DirectMpsc` | Per-record `try_push`; the backends expose no batch operation |

**The mutex baseline amortising its lock over a batch is what makes it a fair
baseline.** A mutex taken per record loses to anything, and a comparison that
establishes it proves nothing that was in doubt. The measurement worth having is
against a competently-written mutex queue.

**The last two rows are an asymmetry worth stating rather than hiding.**
`ring_spsc` and `ring_mpsc` have no batch API, so their runners publish one
record at a time while the Contract candidates publish `batch` at a time through
the same underlying ring. Part of the `ContractRing`-versus-`DirectSpsc` gap is
therefore the batch API's benefit, not only the wrapper's cost — the wrapper is
being credited for something. Recorded here because the alternative (looping
`try_push_batch` with a one-element range) would measure a differently-shaped
lie.

### Complexity

| Quantity | Cost |
|---|---|
| Records published | `offered` = `producers × records_per_producer` |
| Allocations in the timed region | zero for every candidate except `MutexQueue`, whose per-producer staging `Vec` is reserved before the clock starts |
| Accumulator fold | `O( received )`, one `destination_of` + `apply` call per drained record — outside the clock, the pipeline's only post-drain step that scales with `received` rather than being O(1) |
| Counter writes | 4 per run, never per record |
| Drain passes | `O( received / batch )` for the batch-draining candidates, one call per pass |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_workload_description.md](../data_structure/001_the_workload_description.md) | The input every step reads |
| [../data_structure/002_three_counts_that_are_not_interchangeable.md](../data_structure/002_three_counts_that_are_not_interchangeable.md) | The output shape, and why two counts rather than one |

### Algorithms

| File | Relationship |
|------|--------------|
| [002_the_eligibility_filter_runs_before_the_comparison.md](002_the_eligibility_filter_runs_before_the_comparison.md) | What happens to the six results afterwards |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_the_counters_are_written_outside_the_clock.md](../invariant/002_the_counters_are_written_outside_the_clock.md) | Step 6's placement, stated as a property |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_a_description_to_a_verdict.md](../lifecycle/001_from_a_description_to_a_verdict.md) | These steps as phases, with the crate boundaries marked |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_a_counter_inside_the_timed_region_measures_itself.md](../pitfall/002_a_counter_inside_the_timed_region_measures_itself.md) | Why step 6 is where it is |
| [../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md) | Why step 4 is mandatory even though it is not timed |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_flush/src/lib.rs`](../../../ring_flush/src/lib.rs) | `Flusher::run`'s free-capacity pre-check, which is why the staged candidate's step 3 can publish nothing at all |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `a_roomy_run_keeps_everything_and_returns_it` exercises all six steps on every candidate; `a_partial_final_batch_is_published_rather_than_abandoned` covers the batching row's tail case, which coverage found and no assertion had; `every_candidate_agrees_on_the_delta_table_at_one_producer` and `delta_sums_correctly_across_many_producers_without_a_lock` exercise step 5 specifically, across every candidate and under contention |

### BN1 — The Baseline Is the One Candidate Whose Conservation Cannot Fail

`commit_batch` counts what it pushed, not what it was handed:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- what the mutex candidate reports --'
awk '/^fn commit_batch/, /^\}/' src/lib.rs | command grep -E 'taken|guard.len|drain' | sed 's/^/    /'
echo '  -- and what it is compared against --'
command grep 'pub const fn conserved\|self.reported == self.received' src/lib.rs
echo '  -- the fold that reads it --'
awk '/pub fn conserved\( &self \) -> bool/, /^  \}/' src/lib.rs | sed 's/^/    /'
echo '  -- which candidates the suite pins as diverging --'
command grep 'conserved()' tests/bench_test.rs | sed -E 's/^(.{0,96}).*/\1/'
```

Live output:

```
  -- what the mutex candidate reports --
      let mut taken = 0;
      for record in staged.drain( .. )
        if guard.len() < capacity
          taken += 1;
      taken
  -- and what it is compared against --
  pub const fn conserved( &self ) -> bool
    self.reported == self.received
  -- the fold that reads it --
      pub fn conserved( &self ) -> bool
      {
        self.outcomes.iter().all( Outcome::conserved )
      }
  -- which candidates the suite pins as diverging --
  assert!( direct.conserved() );
  assert!( mutex.conserved() );
    assert!( outcome.conserved() );
  assert!( staged.conserved() );
    // `conserved()` is the only assertion here that reads `reported`, and it
      outcome.conserved(),
  assert!( staged.conserved() );
  assert!( !through_the_factory.conserved() );
    assert!( outcome.conserved(), "{} absorbed a record it reported", candidate.name() );
  assert!( !comparison.conserved() );
  assert!( comparison.conserved(), "every candidate reported exactly what it kept" );
  assert!( roomy_run.conserved() );
  assert_eq!( first.conserved(), second.conserved() );
```

`commit_batch` drains the whole staged vector and increments `taken` only inside
the `guard.len() < capacity` branch, so records that arrive at a full queue are
destroyed without being counted. `reported` is the sum of those `taken` values;
`received` is the length of the drained queue. **They are the same number by
construction.**

So `Outcome::conserved()` is unconditionally `true` for `MutexQueue`, and
`Comparison::conserved()` — an `all()` over five outcomes — has one arm that
cannot contribute a `false`.

**Two of the assertions above are that arm.** Line 272's `assert!(
mutex.conserved() )` is a direct one, and line 483's loop asserts
`outcome.conserved()` across four candidates of which the mutex is one. Neither
can fail for that candidate under any workload, any capacity, any policy — they
are the same expression as `assert!( true )` with a candidate's name on it. They
read exactly like line 267's `assert!( direct.conserved() )`, which is a real
measurement of a real path, because nothing at the call site distinguishes a
count that was compared from a count that was copied.

**That is the wrong arm to have immune.** The crate's largest finding is
[`pitfall/003`](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md): a
write API that returns `Ok` for a record it discarded. `conserved` is the flag
that detects exactly that, and the candidate everything else is measured against
is the one candidate on which the flag is a tautology. A baseline is supposed to
be the boring comparison point; here it is also the only path whose accounting
is structurally beyond suspicion, which makes it look better on the one axis the
crate treats as most important.

Nothing is wrong with the implementation — a hand-written queue that counts its
own pushes is the correct way to write a baseline. The finding is that the
resulting `true` reads identically to a measured `true`, and
`Comparison::conserved()`'s doc describes the fold as telling you "at least one
path absorbed a record it reported taking", which is a claim about all five.

### BN2 — The Only Guarantee About Record Values Is Bypassed by Two Runners and Asserted by Nobody

`records_of` exists to make each producer's records identifiable; two runners
build the range inline instead:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- what records_of promises --'
awk '/The records one producer publishes/, /pub fn records_of/' src/lib.rs | sed -E 's/^ *\/\/\/ ?//'
echo '  -- who calls it, and who does not --'
command grep 'records_of( index )\|records_of( 0 )\|as Record \.\. end as Record' src/lib.rs | sed 's/^/    /'
echo '  -- what the suite asserts about record values --'
command grep 'records_of' tests/bench_test.rs | sed 's/^/    /'
echo '  -- what every drain site does with what it took --'
awk -v n1="$(( $( command grep -n -m1 -F /// src/lib.rs | cut -d: -f1 ) + 1 ))" 'NR > n1 && /drain\(|try_recv_batch|\.pop\(/ { printf "    %-5s %s\n", NR, $0 }' src/lib.rs \
  | sed -E 's/^(.{0,116}).*/\1/'
```

Live output:

```
  -- what records_of promises --
The records one producer publishes.

Disjoint per producer, so a drained record identifies its writer. Nothing
in this crate asserts on the values yet — they exist so that a future
ordering question can be asked of a recorded run rather than of a new one.
  #[ must_use ]
  pub fn records_of( &self, producer : usize ) -> Range< Record >
  -- who calls it, and who does not --
                for record in workload.records_of( index )
        let mut records = next as Record .. end as Record;
      for record in workload.records_of( 0 )
      for record in workload.records_of( 0 )
                for record in workload.records_of( index )
        let mut records = next as Record .. end as Record;
  -- what the suite asserts about record values --
      assert_eq!( workload.records_of( 0 ), 0 .. 256 );
      assert_eq!( workload.records_of( 3 ), 768 .. 1024 );
  -- what every drain site does with what it took --
    1015    for record in staged.drain( .. )
    1066      queue.into_inner().expect( "no producer panics while holding the lock" ).drain( .. ).collect();
    1095      let taken = consumer.try_recv_batch( &mut sink );
    1147      let taken = consumer.try_recv_batch( &mut sink );
    1178      let batch = consumer.drain();
    1228      let batch = consumer.drain();
    1265      let taken = consumer.try_recv_batch( &mut sink );
```

Four of the six runners call `records_of( index )` or `records_of( 0 )`.
`run_contract_ring` and `run_off_the_shelf` build `next as Record .. end as Record`
from `records_per_producer`, which is `records_of( 0 )` written out — correct
today because both are capped at one producer, and silently wrong the moment
either ceiling is lifted, since every producer would then emit the same values.

**Both ends of the guarantee are unenforced, and the drain sites are why.**
`records_of`'s own doc says the first half: "Nothing in this crate asserts on the
values yet — they exist so that a future ordering question can be asked of a
recorded run rather than of a new one." The second half is structural. Of the
seven sites above, six are consumer drains, and every one of them reduces its
batch to a number in the same expression that produces it — `.drain( ..
).count()` at 735, `consumer.drain().len()` at 844 and 894, and
`try_recv_batch( &mut sink )` at 762, 813 and 929, whose return *is* the count
and whose `sink` is reused for the next pass. No drained record survives the
line that received it, so there is nothing for a test to assert on even if one
wanted to.

**That last sentence has since drifted.** `run_direct_spsc` and
`run_direct_mpsc` no longer reduce their drain to a length inline — both now
collect into a `drained : Vec< Record >` (`drained.extend( batch.iter()
.filter_map( TypedSlot::get ).copied() )`, current `src/lib.rs:1178-1183` and
`:1228-1233`) and return it as part of the function's own tuple. Two of the
six sites therefore no longer match "no drained record survives the line that
received it" — a record from these two runners does survive. Whether anything
downstream actually asserts record identity on the surviving `Vec` is not
re-derived here.

The seventh, at 685, is the exception that names the shape: it binds `record`
and pushes it — and it is on the **write** side, inside `commit_batch`, the
mutex baseline's staging drain. The one drain in the crate that looks at a value
is the one BN1 is about, and it looks at it in order to count it.

The suite therefore asserts the *function* (`records_of( 3 ) == 768 .. 1024`)
and stops, because the function is the only place a record value still exists.

So the disjointness property is stated in a doc comment, implemented in one
function, reimplemented as a special case in two others, and checked by an
assertion on the function's arithmetic. The future question it was designed for
would be asked of recorded runs produced by six runners, two of which do not go
through the design.

The general shape: **a guarantee held for a future use has no present consumer
to keep it honest**, and inlining a special case of it costs nothing measurable
until the special case stops being one.
