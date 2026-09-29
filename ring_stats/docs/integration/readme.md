# integration

Two crates in thirty-three declare `ring_stats`, and the interesting question is not
how few that is but what they do with it. Traced end to end, the write path leads to
a function nothing imports, and the read path leads to a benchmark harness that fills
every counter once, on one thread, after the clock has stopped.

The two instances here follow those paths separately. The first counts every
production line that moves a counter and finds two callers named in documentation
that do not exist in the workspace — `ring_core` reaching for the counter-free half of
`ring_overflow::resolve`, and `RingStats::reset` naming a caller in a crate that does
not depend on this one. The second follows the reads to `ring_bench::Outcome`, whose
own comment says the counters are written from totals after the run, and to two
dependency edges the workspace does not have: one refused in writing with its
reversal condition recorded, one never mentioned anywhere.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_write_path_and_two_callers_that_are_not_there.md) | The Write Path, and Two Callers That Are Not There | Every production recorder call, `resolve`'s importers, and `reset`'s absent caller |
| [002](002_the_read_path_and_the_removed_edge.md) | The Read Path, and the Edge That Was Assigned and Removed | `ring_bench`'s post-hoc totals, `ring_factory`'s recorded refusal, and one dependency |

## A Feature Delivered Beside the Path It Was For

This crate's own contract says these counters "are the only thing that distinguishes a ring under
mild pressure from one that is quietly discarding traffic". The function that would
do the distinguishing is `ring_overflow::resolve`, which takes a `&RingStats` and
records the drop. It is imported by exactly one file in the workspace, and that file
is a test.

`ring_core`, which owns the ring and handles the full-ring case, imports
`would_resolve` — documented as the pure half, "without touching any counters" — and
does not declare `ring_stats` at all. So when a record is refused in production, no
counter moves. Nothing is miswritten; the counting variant simply is not on the route.

## Everything Hazardous Is Switched Off in the Only Real Use

`ring_bench::Outcome` is the family's one live consumer, and its accessor's doc says
plainly that the counters are "written once from totals after the clock stopped".
That single sentence disables every hazard this corpus records: no concurrency, no
tear, `in_flight` fixed at zero by construction, one policy per run so no conflation.

Which makes it a worked example in which none of `RingStats`' actual behaviour is
exercised — and the crate's only other production write site is `ring_overflow`'s
one-drop-at-a-time `record_drop`, reached only from that crate's own tests.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- who declares ring_stats --'
for f in */Cargo.toml ; do if command grep -q '^ring_stats' "$f"; then echo "    $( basename "$( dirname "$f" )" )/Cargo.toml"; fi; done
echo '  -- every production call of a recorder --'
command grep -rn 'stats\.record_' --include=*.rs */src/ 
echo '  -- who imports the counting half --'
command grep -rn 'use ring_overflow::' --include=*.rs */src/ */tests/ | command grep -v '///' 
echo '  -- who calls reset, and what its doc says --'
command grep -rn '\.reset()' --include=*.rs ring_*/
command grep -c 'ring_stats' ring_shutdown/Cargo.toml || true
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| ST17 | `ring_stats` | **latent hazard** | The workspace's only non-benchmark line that moves a counter is `record_drop` inside `ring_overflow::resolve`, which no `src/` anywhere imports — `ring_core` handles the full-ring case through `would_resolve`, the documented counter-free half, and does not declare `ring_stats` — so a record dropped in production moves nothing, and this crate's own promised diagnosis is unreachable from the write path with nothing recording that |
| ST18 | `ring_stats` | **wrong doc** | `RingStats::reset`'s doc states it is "Used by `ring_shutdown`'s reset", and `ring_shutdown` neither declares `ring_stats` in its four dependencies nor contains the string `stats` in any `.rs` file — the call could not compile, and `reset()` is invoked only from two tests in this crate and its own doctest |
| ST19 | `ring_stats` | n/a — coverage | `ring_bench::Outcome` is the family's only live consumer and its accessor's own doc says the counters are "written once from totals after the clock stopped" — one thread, one write per counter, `claimed` equal to `published`, one policy per run — so the sole worked example of `RingStats` in use exercises none of the concurrent behaviour that every finding in this corpus concerns |
| ST20 | `ring_stats` | n/a — observation | `ring_factory` refuses a `ring_stats` edge in five lines of manifest comment carrying the reasoning, the resolved decision reference and the reversal condition — while `ring_stats`' own manifest declares one dependency and says nothing about `ring_align`, whose `CacheAligned` addresses the 2.6×–3.3× its layout costs; the same absence, documented to opposite standards |
