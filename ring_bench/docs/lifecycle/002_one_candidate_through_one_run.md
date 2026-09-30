# Lifecycle: One Candidate Through One Run

### Scope

- **Purpose**: Record the states one candidate occupies during `run`, the guard on each transition, and why two of the terminal states are both successes.
- **Responsibility**: State the states, transitions, guards, and terminals.
- **In Scope**: `run( candidate, workload )` from entry to `Result`.
- **Out of Scope**: The cross-crate phase ordering (→ [`lifecycle/001`](../lifecycle/001_from_a_description_to_a_verdict.md)); the ring's own internal states.

### States

| State | Entered when | Terminal |
|---|---|:---:|
| `Admitting` | `run` is called | no |
| `Refused` | the producer ceiling is exceeded | ✅ `Err` |
| `Building` | the ceiling check passed | no |
| `Unbuildable` | a dependency refused the configuration | ✅ `Err` |
| `Publishing` | construction succeeded; the clock is running | no |
| `Draining` | the clock has stopped | no |
| `Accounted` | the counters are written | ✅ `Ok` |

### Transitions

```text
Admitting ──[ ceiling exceeded ]──────────────► Refused        (Err::ProducerCeiling)
Admitting ──[ ceiling absent or satisfied ]───► Building

Building  ──[ dependency refused ]────────────► Unbuildable    (Err::Build|Ring|Flush)
Building  ──[ constructed ]───────────────────► Publishing     ⏱ clock starts

Publishing ─[ every record offered ]──────────► Draining       ⏱ clock stops
Draining  ──[ consumer empty ]────────────────► Accounted      (Ok::Outcome)
```

#### The guards

**G1 — the ceiling, on `Admitting`.** Read from
`Candidate::producer_ceiling() : Option< usize >`. `None` means no ceiling and
the guard cannot fire; `Some( n )` fires when the workload asks for more than
`n`. Checked before anything is allocated, so a refusal costs nothing — which
is why `Comparison::run` can afford to attempt all six candidates
unconditionally.

**The important property of G1 is where the ceilings come from.** Three of the
four `Some( 1 )` values describe the door rather than the structure: the ring
underneath is multi-producer capable and the Contract cannot express that.
→ [`pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md).

**G2 — the dependency's refusal, on `Building`.** Not this crate's rule; it is
whatever the constructed crate says. `OverflowPolicy::DropOldest` fires it,
because both in-house backends refuse to evict an unread record. The variant
names the route: `RunError::Build` through the factory, `RunError::Ring` when
the staged candidate builds its ring directly.

**Two variants for one refusal is a finding, not redundancy.** It is the
observable trace of the staged candidate having to reach below the export
Contract to get a `ring_core::Producer` that `Flusher::new` accepts.
→ [`integration/001`](../integration/001_declared_edges_and_the_three_that_were_missing.md).

### Behavioral Invariants

**Once construction succeeds, the run cannot fail.** Every transition out of
`Publishing` and `Draining` is unguarded — records are dropped, not refused, and
a drop is a number rather than an error.
→ [`pattern/002`](../pattern/002_a_refusal_is_a_row.md).

#### `Accounted` is reached by losing runs too

**A candidate that kept nothing still ends in `Accounted`.** On a 16-slot ring
offered 256 records, the staged candidate reaches `Ok` with `received` far below
`offered`. Turning that into an `Err` would be the more obvious design and it
would delete the measurement: *how much a path loses under pressure* is one of
the two things this crate exists to report.

The judgement lives one level up, in the eligibility filter, which excludes
lossy outcomes from being fastest without excluding them from the table.
→ [`algorithm/002`](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md).

#### The counters are written on the `Draining → Accounted` edge

All four `RingStats` writes happen on that single transition, from totals, and
every one of them takes `received` — never `reported`:

```rust
stats.record_claim( received as u64 );
stats.record_publish( received as u64 );
stats.record_consume( received as u64 );
stats.record_drop( workload.config().overflow(), ( offered - received ) as u64 );
```

**Placing them on this edge rather than inside `Publishing` is the whole
measurement discipline in one line of code.**
→ [`non_functional_requirement/002`](../non_functional_requirement/002_the_harness_is_not_in_the_measurement.md).

**And using `received` rather than `reported` is what keeps `in_flight` at
zero.** A record the ring never took never claimed a slot, so it was never
claimed — the first version of this mapping fed workload totals into slot
counters and made 240 refused records read as 240 leaked slots on a ring with
16 of them.
→ [`pitfall/003`](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md).

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_workload_through_six_runners.md](../algorithm/001_one_workload_through_six_runners.md) | This machine run six times |
| [../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md) | What happens to `Accounted` outcomes afterwards |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_three_counts_that_are_not_interchangeable.md](../data_structure/002_three_counts_that_are_not_interchangeable.md) | `reported` vs `received` on the accounting edge |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_declared_edges_and_the_three_that_were_missing.md](../integration/001_declared_edges_and_the_three_that_were_missing.md) | Why G2 has two variants |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_a_description_to_a_verdict.md](../lifecycle/001_from_a_description_to_a_verdict.md) | The same run as phases, with crate ownership marked |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_harness_is_not_in_the_measurement.md](../non_functional_requirement/002_the_harness_is_not_in_the_measurement.md) | Why the counters sit on the edge they do |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_door_caps_what_the_structure_does_not.md](../pitfall/001_the_door_caps_what_the_structure_does_not.md) | Where G1's ceilings come from |
| [../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md) | Why the accounting edge uses `received` |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_run_error.md](../type/002_run_error.md) | The two terminal `Err` states, and which guard produces each |

### Sources

| File | Relationship |
|------|--------------|
| [`../readme.md`](../readme.md) | The crate's own stated purpose — the comparison is the deliverable, which is why losing runs end in `Ok` |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `the_contract_door_caps_a_multi_producer_structure_at_one_producer` fires G1; `a_policy_refusal_names_the_crate_that_refused` fires G2 on both routes; `a_cramped_run_drops_and_the_drop_is_counted_from_the_drain` reaches `Accounted` with heavy loss |

### BN31 — Six of the Seven State Names Appear Nowhere in the Crate

The States table and the transition diagram describe a machine with seven states
and five edges. No such machine is implemented:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- each documented state name, counted in source and tests --'
for s in Admitting Refused Building Unbuildable Publishing Draining Accounted; do
  printf '    %-12s src %s   tests %s\n' "$s" \
    "$( command grep -c "$s" src/lib.rs )" "$( cat tests/*.rs | command grep -c "$s" )"
done
echo '  -- the one nonzero hit, in full --'
command grep 'Refused' src/lib.rs | sed -E 's/^(.{0,100}).*/\1/' | sed 's/^/    /'
echo '  -- and where this crate does put a "State Machines" heading --'
command grep -r '^### State Machines' docs/ | sed 's/^/    /'
```

Live output:

```
  -- each documented state name, counted in source and tests --
    Admitting    src 0   tests 0
    Refused      src 1   tests 0
    Building     src 0   tests 0
    Unbuildable  src 0   tests 0
    Publishing   src 0   tests 0
    Draining     src 0   tests 0
    Accounted    src 0   tests 0
  -- the one nonzero hit, in full --
        /// Refused and silently discarded records both count here — from the
  -- and where this crate does put a "State Machines" heading --
    docs/type/001_candidate.md:### State Machines
    docs/lifecycle/001_from_a_description_to_a_verdict.md:### State Machines
    docs/data_structure/001_the_workload_description.md:### State Machines
```

`run` is a sequence of statements. The states are names a reader gives to
positions between them, and the only one that appears in the source is a word in
an unrelated doc comment about `Outcome::dropped`. Nothing constructs a state,
nothing asserts one, and no test can observe a transition — the entire machine
is a reading of control flow, and it is a correct one.

**That is defensible and it has a specific cost: the model cannot drift-check
against anything.** The `Publishing ─► Draining ⏱ clock stops` edge is drawn once
here; the code has six clock stops in six functions
(→ [`invariant/002`](../invariant/002_the_counters_are_written_outside_the_clock.md)'s
BN24). The `Draining ─► Accounted` edge is drawn as the single place the counters
are written, and it is — but the same collapse hides that the edge before it
differs per runner. A one-edge diagram over six implementations is accurate about
the shape and silent about the variation, and there is no mechanism that would
ever say so.

**The heading placement makes the same point structurally.** This crate's three
`### State Machines` headings are in `lifecycle/001`, `type/001` and
`data_structure/001`. The document that *is* the state machine has none — because
`state_machine/` is not one of the thirteen definitions and its content folds into
`lifecycle/`, so the heading survives as a section in other instances while the
instance it names has no reason to carry it. Across all 33 crates, 33 have
`lifecycle/` and none has `state_machine/`.

The general shape: **a state machine that exists only as documentation is a
model, and a model is checked by a reader or by nothing.** Writing it down is
still worth doing; believing the diagram tracks the code is the error, and there
is no signal that would distinguish the two.

### BN32 — The Diagram Names Three Refusal Variants and the Paragraph Below It Names Two

Four lines apart, this document gives the `Building → Unbuildable` edge two
different variant counts:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the edge label in the diagram --'
awk '/^### BN/ { exit } /Unbuildable/ { printf "    %3d  %s\n", NR, substr( $0, 1, 96 ) }' \
  docs/lifecycle/002_one_candidate_through_one_run.md
echo '  -- the sentence under G2, and the one under it --'
awk '/^### BN/ { exit } /names the route/, /not redundancy/' \
  docs/lifecycle/002_one_candidate_through_one_run.md | sed 's/^/    /'
echo '  -- the variants that exist, and where each is produced --'
awk '/pub enum RunError/, /^\}/' src/lib.rs | command grep -E '^  [A-Z]' | sed 's/^/    /'
command grep 'map_err( RunError::' src/lib.rs | sed -E 's/^(.{0,96}).*/\1/' | sed 's/^/    /'
```

Live output:

```
  -- the edge label in the diagram --
     17  | `Unbuildable` | a dependency refused the configuration | ✅ `Err` |
     28  Building  ──[ dependency refused ]────────────► Unbuildable    (Err::Build|Ring|Flush)
  -- the sentence under G2, and the one under it --
    names the route: `RunError::Build` through the factory, `RunError::Ring` when
    the staged candidate builds its ring directly.
    
    **Two variants for one refusal is a finding, not redundancy.** It is the
  -- the variants that exist, and where each is produced --
      ProducerCeiling
      Build
      Ring
      Flush
```

The diagram's edge is labelled `Err::Build|Ring|Flush` — three. The guard
paragraph immediately below says "the variant names the route:
`RunError::Build` through the factory, `RunError::Ring` when the staged
candidate builds its ring directly" — two, and the next paragraph makes the
number load-bearing: "**Two variants for one refusal is a finding, not
redundancy.**"

`RunError::Flush` is real, is produced on this edge at line 782, and is
unreachable through the public API — `Workload::with_batch` rejects zero, so
`FlushPolicy::OnBatch( workload.batch() )` can never be the `ConfigError`
`Flusher::new` returns. That is
[`api/001`](../api/001_the_run_surface.md)'s BN6, and it is the reason a careful
enumeration lands on two while a mechanical one lands on three.

**Both counts are defensible and neither says which it is counting.** "Variants
this edge can produce" is three; "variants this edge is observed to produce" is
two. The document uses the first in the diagram, the second in the prose, and
draws a conclusion from the second — so a reader reconciling the two has to
already know the reachability fact that neither states.

The general shape, and the reason this is filed rather than silently fixed:
**a count is a claim with a hidden predicate**, and where the predicate is
"reachable", the number changes without anything in the code changing. The
diagram is the enumeration a compiler would give; the paragraph is the
enumeration a run would give; the gap between them is exactly one unreachable
variant, and it is the same variant a test in this crate is named after.
