# Non-Functional Requirement: Cheap Enough to Leave On

### Scope

**Purpose:** Take the crate's one stated non-functional requirement — that the
counters are cheap enough to leave on — and measure it, per call and per system.

**Responsibility:** What `record_claim` costs against an empty loop, how that cost
behaves as producers are added, and what the crate itself measures.

**In Scope:** This crate's own stated cheapness requirement; the module comment's
cheapness and ordering paragraphs in `ring_stats/src/lib.rs`;
`ring_stats/tests/stats_test.rs`.

**Out of Scope:** The layout that makes contention worse than it needs to be is
[`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md). The
ordering cost the crate reasons about instead is
[`decisions/001`](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md). A
counter distorting the thing it measures is
[`non_functional_requirement/002`](002_counters_that_must_not_distort_what_they_measure.md).

---

## The Requirement, and What Checks It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the requirement, as feature 185 states it --'
command grep -m1 -F 'Per-ring counters for what actually happened: items claimed, items published, items dropped and under which policy, and nanoseconds spent waiting. They are cheap enough to leave on and are the only thing that distinguishes a ring under mild pressure from one that is quietly discarding traffic.' docs/feature/185_ring_stats.md
echo '  -- every cost statement in the crate source --'
command grep 'cheap\|cost\|free\|overhead\|fence\|hot path' ring_stats/src/lib.rs
echo '  -- and what measures any of it --'
printf '    benches/ present: %s   Instant/Duration in the suite: %s\n' \
  "$( test -d ring_stats/benches && echo yes || echo no )" \
  "$( command grep -c 'Instant\|Duration' ring_stats/tests/stats_test.rs || true )"
```

Live output:

```
  -- the requirement, as feature 185 states it --
Per-ring counters for what actually happened: items claimed, items published, items dropped and under which policy, and nanoseconds spent waiting. They are cheap enough to leave on and are the only thing that distinguishes a ring under mild pressure from one that is quietly discarding traffic.
  -- every cost statement in the crate source --
//! `docs/feature/185_ring_stats.md` asks for counters cheap enough to leave on
//! synchronisation point, so ordering them would buy nothing. The fence-cost
//! contention is the real, measured cost in this struct, at 2.6x-3.3x, not
//! calls `would_resolve`, the counter-free half, and does not declare
  -- and what measures any of it --
    benches/ present: no   Instant/Duration in the suite: 0
```

---

### ST33 — The Per-Call Claim Is True, and Nothing in the Crate Establishes It

`record_claim` costs six to ten nanoseconds more than the same loop touching no
counter at all, and that figure barely moves as producers are added:

```
  -- ns per operation, median of 9 paired runs, 400000 ops per thread --

    threads   nothing   record_claim   one_each   claim cost   claims/sec
          1      2.0ns          8.3ns       9.9ns       +6.3ns     120.0M
          3      0.8ns         11.8ns      11.1ns      +11.0ns      84.8M
          6      0.5ns         10.2ns      10.3ns       +9.7ns      98.3M
         12      0.3ns         10.6ns      10.7ns      +10.3ns      94.6M

    'nothing'      the same loop with no counter touched at all
    'record_claim' every thread bumping the one counter
    'one_each'     threads split across claimed and consumed
    'claims/sec'   aggregate across all threads, from the record_claim column
```

A second run, to establish the numbers reproduce:

```
  -- ns per operation, median of 9 paired runs, 400000 ops per thread --

    threads   nothing   record_claim   one_each   claim cost   claims/sec
          1      2.1ns          9.9ns       9.1ns       +7.8ns     101.1M
          3      0.8ns         10.5ns      10.4ns       +9.6ns      95.5M
          6      0.5ns         10.0ns       9.8ns       +9.5ns     100.5M
         12      0.3ns         10.3ns      10.4ns      +10.1ns      96.7M
```

**Finding.** Ten nanoseconds is cheap by any reasonable reading of "cheap enough to
leave on", so the requirement is met — and the crate has no way of knowing that. There
is no `benches/` directory, no `Instant` anywhere in the suite, and no assertion
anywhere that bounds a cost. The module comment's opening paragraph restates the
feature's claim as the crate's own premise and nothing downstream of it is a check.

This is the crate's one stated non-functional requirement, and it is the one property
the suite does not touch — seventeen tests when this was written, twenty-one since, and
the four added in between are all correctness tests too. That the answer turns out to be
favourable is a fact about the hardware, not a fact anyone here established.

---

### ST34 — The Cost Is Per Call; the Ceiling Is Per System, and Neither Lever Moves It

Read the `claims/sec` column down rather than the `claim cost` column across. One
thread records about 100M claims per second. Twelve threads record about 95M. Adding
eleven cores to the machine buys nothing at all, because a single `fetch_add` on a
single cache line is a serialisation point that no amount of parallelism widens.

That ceiling has two obvious workarounds and this layout defeats both.

*Use a different counter.* The `one_each` column splits the threads across `claimed`
and `consumed` — two distinct fields, two distinct atomics — and measures the same as
all twelve threads hammering one. The counters are not independent in practice because
all seven live inside a 56-byte struct on one cache line
([`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md) § ST9), so
the coherence traffic is identical whichever field is named. This confirms from the
opposite direction what the padded comparison found: the packed layout costs 2.6×–3.3×
under contention, and here the same effect appears as two counters behaving as one.

*Add producers.* Already covered: it does not scale.

**Finding.** "Cheap enough to leave on" is a claim about one call, and it is true. The
property that would actually bind a system is the aggregate ceiling, and neither the
feature nor the crate states it, measures it, or provides an affordance for raising
it — no per-producer sharding, no padded variant, no way to opt one counter out.

Whether 95M records per second binds anything is unknown, and unknowable from this
workspace: no crate here records a ring throughput to compare it against. What is
knowable is that nothing currently pays this cost at all. `ring_bench` writes every
counter once per run from totals after the clock stops, deliberately, because a
per-record counter would have distorted its comparison
([`non_functional_requirement/002`](002_counters_that_must_not_distort_what_they_measure.md));
and the only per-item recorder in the workspace sits inside `ring_overflow::resolve`,
which no `src/` imports
([`integration/001`](../integration/001_the_write_path_and_two_callers_that_are_not_there.md)
§ ST17). The measurement above says what the requirement would cost the first time
something actually leaves the counters on.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_stats
command grep -F 'is a separate property this crate does not' src/lib.rs
```

Live output:

```
//! spreading load across counters — is a separate property this crate does not
```

**Disposition:** applied — the module comment now scopes "cheap enough to leave
on" to the per-call figure it actually measures, and names the aggregate
per-system ceiling — bound by one `fetch_add` on one cache line, unmoved by
adding cores or spreading load across counters — as a separate, unstated,
unbounded property, matching what this finding measures.
Now prints: `is a separate property this crate does not`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/002`](002_counters_that_must_not_distort_what_they_measure.md) | The requirement the feature does not state, which a consumer had to hold anyway |
| [`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md) | The layout behind the ceiling, and the padded comparison |
| [`decisions/001`](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md) | The cost the crate reasons about, measured at roughly nothing |
| [`lifecycle/002`](../lifecycle/002_implemented_tested_and_still_planned.md) | The consumer whose adoption is conditional on this cost |

### Sources

| Fact | Where |
|------|-------|
| "cheap enough to leave on" | Census above |
| The crate restating it as its premise | The module comment's opening paragraph |
| No `benches/`, no `Instant` in the suite | Census above |
| Per-call cost and aggregate ceiling, two runs | Probe, quoted above |
| Two counters on one line measure as one | `one_each` column, same probe |
| `ring_bench` writes counters outside the clock | `ring_bench/docs/pitfall/002_a_counter_inside_the_timed_region_measures_itself.md` |

### Tests

| Test | Covers |
|------|--------|
| `counts_are_exact_under_contention` | Correctness under twelve threads, with no timing taken |
| `batched_and_single_recording_agree` | That batching is available, which is the one cost lever a caller has |
| *(to create)* | Nothing bounds a per-call cost, and the crate's only stated requirement is a cost |
