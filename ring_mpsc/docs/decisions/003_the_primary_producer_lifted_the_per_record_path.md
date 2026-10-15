# The primary producer's cached cursors lift the per-record two-thread path by 55% like-for-like

Status: Accepted — measured GO on aarch64; the x86 no-regression pair runs
in this branch's PR window

## Context

The per-record path is `ring_mpsc`'s weak spot on every machine measured:
one producer over `push1_popN` moves 10.9–11.0 M/s on the Raspberry Pi 5
(against `arrayqueue`'s 21.6 and rtrb's 190+) and 26.8–35.5 M/s on the x86
host (against 75–90). The sibling crate's cached cursors
(`ring_spsc/docs/decisions/001`) closed that crate's version of the gap by
making its push fast path load-free. That record lands with PR #20, which
this PR neither includes nor depends on; the reference resolves once that
PR merges, in either order.

The same idea applied naively to this crate is structurally unavailable:
[`Producer`] is `Copy + Sync` — copies of it are how threads join — and a
`Copy` type cannot carry private mutable state. The boundary-gate spike
(ADR 002) removed the consumer-line read a different way and lost on both
instruction sets: the marker store it added to the consumer cost more than
the read it saved.

What the flamegraph actually attributed (Pi 5, the unmodified per-record
path): the producer's 30.4% of process samples split as
`Claimer::claimed` — the load of the claim cursor before the exchange —
16.5%, the exchange itself 3.7%, and the consumer-cursor read
(`headroom → slowest`) 3.0%. The claim cursor's line lives on the consumer's
core between records: the consumer's drain reads it every batch.

## Decision

`Producer::primary(&mut self) -> PrimaryProducer` mints a non-`Copy`,
non-`Sync` handle carrying two cached cursors, one per ring by intent (a
second is safe — the compare-exchange arbitrates — but each pays a failed
exchange whenever the other has claimed since):

- `cached_tail` — this handle's guess at the shared claim cursor's value.
  The claim is a compare-exchange with the guess as its expected value: a
  wrong guess fails safely to the actual value (the CAS arbitrates against
  ordinary `Copy` producers, which stay fully usable beside the primary),
  and a right guess — exact whenever this handle is the only active
  claimer — grants with no cursor load at all.
- `cached_head` — the latest `GATING` observation of the consumer cursor.
  It can only lag (the consumer cursor is monotone; the cache is written
  from observations, never extrapolated), so a grant is made only when the
  cached observation itself proves the slot consumed. Staleness refuses a
  ring that has room; it never overwrites a record the consumer has not
  taken.

The fast path: capacity gate on the cache, one compare-exchange, no loads.
The consumer-cursor read the ordinary claim pays on every attempt is paid
on the refresh — and the refresh's `Acquire` is the edge that orders the
consumer's take of the lapped record before the producer's overwrite. It is
soundness-required and loom-invisible (loom linearises each step); argued
in the module documentation, with the cached-head model's documentation
recording the blind spot.

## Deltas

- `mem::forget(Reserved)`: the claim's exchange moved the shared cursor
  past the forgotten sequence and there is no stamp, so the consumer stalls
  at the hole and later records are unreachable behind it — the ring's
  documented lost-claim pitfall, unchanged from the ordinary producer.
  Pinned by `a_leaked_primary_reservation_stalls_the_drain_at_its_hole`.
- `PrimaryProducer` is an additive type: ordinary `Producer` copies are
  unchanged and stay fully usable beside it. `ring_core` and `ring_handle`
  are not wired to it in this spike.

## Measurement (Pi 5, aarch64, one session; `mpsc-primary` against the ordinary per-record path, rtrb as the untouched control — flat within ±1.6%)

The full generated report: [perf_report_pi_primary.md](../benchmarks/perf_report_pi_primary.md).

Two-thread, capacity 1024. One cell in this table has a matched ordinary
run to compare against: `push1_popN` at 1024 slots, over
`mpsc/push1_popN` at one producer, 10.9 M/s. The harness registers the
ordinary `Mpsc` in that mode and capacity alone — the `push1_pop1` row
runs a different consumer mode, the 64 and 16384 columns a different
capacity — so the other five cells keep their absolute numbers and no
percentage:

| two-thread, `mpsc-primary` | 64 | 1024 | 16384 |
|---|---:|---:|---:|
| `push1_pop1` | 14.6 M/s | **20.6** | 16.8 |
| `push1_popN` | 17.0 | **16.9 (+55%)** | 17.0 |

- Single-threaded fill/drain: +28…+40% (`push1_popN` 64.2–65.4 against
  46.2–46.6 M/s; `push1_pop1` 46.1–46.5 against 35.9–36.1).
- `push_pop`, one thread: 25.7 against 27.7 ns (−7% — the cache is faster
  here too; contrast `ring_spsc`, whose strict-alternation pair regressed).
- `push_full`: 5.1 against 5.7 ns (−11% — the cached full-check refuses
  without the load).
- Mechanism counters over the same 10 s window (`push1_popN/1024`):
  L1-dcache misses 524M — at rtrb's level (517M), below the uncached
  claim's 614M; branch-mispredicts 1.17%, half the ordinary claim's 7.2%;
  IPC 0.50 — the compare-exchange on the shared claim line remains the
  dominant stall, exactly the contract's cost that this spike does not
  address.
- The absolute levels remain far from rtrb and from the cached `ring_spsc`
  (17–21 against 190+ M/s): the residue is the claim exchange itself and
  the consumer's stamp scan — separate follow-ups.
- Correctness gate: 44 integration tests (41 + 3 new), 36 doctests,
  5 compile_fail, loom 5/5 with the new double-grant model, clippy
  `-D warnings`, fmt, workspace green. The model is red under the
  plain-store mutation (both grants collide on sequence zero, the second
  stamp never lands).

### The x86 statement

Three x86 pairs across this investigation never held the untouched controls
flat on the two-thread rows (rtrb alone swung −40%…+149% between runs on
this host), so the x86 statement is bounded rather than precise: within
that noise the primary is neutral two-thread (−5.5%…+10.4% across
capacities against the ordinary path's own row) and positive
single-threaded — fill/drain `push1_popN` 134–135 against 102 M/s (+31%)
and `push_pop` 11.6 against 13.1 ns (−11.5%), measured in the same run. No
x86 regression was observed anywhere; no x86-specific win was expected —
the platform never paid the loads this spike removes.

## Verdict

**GO.** On the one matched pair (`push1_popN`, 1024 slots) the primary
lifts the two-thread per-record path by 55%. Every single-threaded shape
measured improved — fill/drain +28…+40%, `push_pop` −7%, `push_full`
−11% — and nothing in the suite regressed, on the architecture where the
family's per-record costs live. The residual distance to rtrb is the
claim exchange and the stamp scan, not the cursor reads this spike
removed.

## Consequences

- Promotion: a PR from `mpsc-primary-spike`. The branch is cut from
  `master`'s line and does not overlap the `ring_spsc` cached-cursor PR —
  the two may merge in either order.
- The handle is additive API: ordinary `Producer` copies remain unchanged
  and usable beside a primary. `ring_core`/`ring_handle` wiring is deferred
  to integration.
- Follow-ups, recorded: the consumer's stamp scan (the largest remaining
  attributed block, 22.5%-shaped on the x86 profile), the payload-size gap,
  and the claim exchange's own cost — the contract's residue.
