# non_functional_requirement

The crate states one non-functional requirement and inherits it verbatim from the
feature: the counters are "cheap enough to leave on". It is the only property of
`ring_stats` that is a cost rather than a behaviour, it is the condition on which the
crate's one documented consumer reserves the right to drop it, and it is the single
property the suite does not touch — seventeen tests then, twenty-one now, and not one
of them bounds a cost. There is no `benches/` directory and no `Instant` anywhere in
the suite.

Measured, the claim holds and the reasoning behind it does not generalise. One
`record_claim` costs six to ten nanoseconds over an empty loop — cheap by any reading.
But that number is flat from one thread to twelve, which means the aggregate ceiling is
flat too: about 100M records per second whether one core is recording or twelve. The
two things a caller could do about that — spread the load across counters, or turn the
counters off — are both unavailable, one because the seven fields share a cache line
and one because the crate has no feature gate at all.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_cheap_enough_to_leave_on.md) | Cheap Enough to Leave On | The stated requirement, measured per call and per system |
| [002](002_counters_that_must_not_distort_what_they_measure.md) | Counters That Must Not Distort What They Measure | The requirement the crate does not state, and what it offers a caller who needs it |

## A Cheap Call and a Fixed Ceiling

Read the cost per call and the requirement is met with room to spare. Read the
throughput and the picture changes: twelve threads record about the same number of
claims per second as one, because a `fetch_add` on a single cache line serialises and
parallelism does not widen it. Splitting the threads across `claimed` and `consumed`
measures the same as putting all of them on one counter — two distinct atomics
behaving as a single contention point, which is the packed layout's 2.6×–3.3× penalty
observed from the other direction.

Nothing in this workspace records a ring throughput to compare that ceiling against, so
whether it binds is unknown. What is certain is that nothing pays the cost today:
`ring_bench` writes its counters once per run from totals after the clock stops, and
the only per-item recorder in the family sits inside a function no `src/` imports.

## An Instrument With No Controls

The second requirement is one the crate never states — that a counter must not change
what it counts — and the family has already hit it. `ring_bench` had to publish the
qualification `ring_stats` omits: "The feature's sentence is true and the inference
from it — leave them on here too — is not." It paid for its mitigation in resolution,
giving up any intra-run distribution to keep the counters outside the timed region.

A caller reaching the same conclusion has nothing to reach for. No `[features]`
section, no `cfg( feature = … )`, no no-op form, no sharded recorder — so the only
available response is to stop calling the recorders, which converts a per-event
instrument into a per-run summary. The mitigation that would have raised the ceiling
appears in `ring_bench`'s rejected list, and one of the reasons it was rejected is that
the state it needs is per-thread — which it is, because this crate does not provide it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# the one stated requirement lived in docs/feature/185_ring_stats.md — a
# pre-implementation design corpus external to this repository, unreachable
# since extraction; the quote it once produced is preserved as a historical
# note in non_functional_requirement/001.md
echo '  -- what the crate does to check it --'
printf '    benches/ present: %s   Instant/Duration in the suite: %s\n' \
  "$( test -d ring_stats/benches && echo yes || echo no )" \
  "$( command grep -c 'Instant\|Duration' ring_stats/tests/stats_test.rs || true )"
echo '  -- and what it offers a caller who cannot afford it --'
printf '    [features]: %s   cfg(feature): %s   no-op form: %s\n' \
  "$( command grep -c '^\[features\]' ring_stats/Cargo.toml || true )" \
  "$( command grep -c 'cfg( feature\|cfg(feature' ring_stats/src/lib.rs || true )" \
  "$( command grep -ci 'noop\|no_op\|disabled\|enabled' ring_stats/src/lib.rs || true )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| ST33 | `ring_stats` | n/a — coverage | `record_claim` measures six to ten nanoseconds over an identical loop touching no counter, so the crate's one stated non-functional requirement is met — while the crate itself has no `benches/` directory, no `Instant` anywhere in its suite, and no assertion bounding any cost, so the only property it claims is the only one it never checks |
| ST34 | `ring_stats` | **measured cost** | Per-call cost is flat from one thread to twelve, which fixes aggregate throughput at roughly 100M records per second regardless of core count — and neither escape works: splitting threads across `claimed` and `consumed` measures the same as loading one counter, because all seven fields share a cache line, so the two counters contend as one |
| ST35 | `ring_stats` | n/a — doc gap | The crate repeats "cheap enough to leave on" twice without naming a single condition under which it does not apply, and the one consumer that hit such a condition — `ring_bench`, whose counted events are its timed events — had to write the qualification itself, in its own docs, where it does not generalise to the next consumer |
| ST36 | `ring_stats` | n/a — doc gap | A crate whose entire purpose is instrumentation ships no mechanism for controlling the instrument: no `[features]` section, no `cfg( feature = … )`, no no-op variant and no sharded recorder — so the only way to decline the cost is to stop calling, and the sharding that would have removed the contention entirely was rejected downstream partly because this crate does not provide it |
