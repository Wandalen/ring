# The producer caches the consumer cursor, and the push fast path loads nothing

Status: Accepted — measured GO on aarch64; the x86 no-regression pair is
re-running after its first attempt failed the controls-flatness check

## Context

The review that opened this investigation recorded: `ring_spsc` loses to
rtrb in every test where a producer and a consumer run concurrently, the gap
at 5–15× is unlikely to be a machine artifact, and confirming it on Linux
with pinned threads would settle that. It also ordered the follow-up: the
two crates are worth investigating, `ring_spsc` first.

The measurement that confirmed it came from a Raspberry Pi 5 (aarch64, 4
cores, governor performance): 4.4–4.7× at `push1_pop1`, 11–18× at
`push1_popN` (12.7 against 244 M/s at capacity 16384), 7–15× across payload
sizes — while the same modes on an x86 host measured 1.2–1.5×. The gap is
real, ARM-sized, and invisible on x86-TSO, where an `Acquire` load is a
plain load and the family's ordering discipline is free.

The mechanism, measured on the same machine: the producer spent 31% of all
samples in `occupancy → is_full` — the `Acquire` read of the consumer cursor
on every push — the consumer 20.4% in `drain_up_to`; per record the pair
paid 4.9 L1-dcache misses against rtrb's 0.27 and ran at IPC 0.18 against
1.63. A branch-misses profile attributed the 0.96 mispredicts per record to
the consumer's control flow (46% in the `drain_up_to` region, 36% in the
payload iteration), not to the producer.

## Decision

`Producer` carries two plain fields — lawful, because the end is
`&mut`-exclusive and neither `Copy` nor `Clone`:

- `cached_tail`, the next sequence to claim — the producer cursor's private
  mirror, advanced by `claim`, published to the shared cursor by
  `Reservation::drop`, and equal to it whenever no reservation is live;
- `cached_head`, the latest `GATING` observation of the consumer cursor.

`claim`'s fast path grants from the two cached values and performs no atomic
load. When the cache reports full, one `GATING` load of the consumer cursor
refreshes it — the single read of the consumer's line on this path — and the
refreshed observation decides honestly. The tail cache starts from the
carried-over cursor on a re-split; `free_capacity`, `available` and
`is_empty` keep their real-cursor binding contract; the consumer end is
untouched.

### Soundness

`cached_head` can only lag the real cursor: the consumer cursor is monotone
and the cache is written from observations, never extrapolated. A lapped
claim is granted only when the cached value itself proves the record
consumed — `cached_head > seq − capacity` — and that value was read at
`GATING`, which pairs with the consumer's `HANDOFF` commit store: the
consumer's take of the old record precedes its commit, the refresh observes
the commit, and the overwrite follows the refresh. Staleness can refuse a
ring that has room; it can never overwrite an unread record.

The refresh's `Acquire` is soundness-required and is the one part of the
protocol the loom models cannot check: loom linearises each step, so a
relaxed refresh that returns the committed value orders identically to an
acquired one there, while on hardware without the edge the overwrite and the
take can race. It is argued in the module documentation, and the cached-head
model's documentation records the blind spot rather than claiming a red it
cannot deliver. The model is red under the overclaim mutations (deleting the
refresh, or letting the cache report progress the consumer never made),
verified before commit.

## Semantic deltas

- `mem::forget(Reservation)` — the one way to keep a claim unpublished. The
  uncached bookkeeping re-granted the leaked sequence to the next claim,
  putting two owners on one slot and stalling the ring. The cached
  bookkeeping skips it: the next publish lands past the leak and the
  consumer receives the never-written slot's contents as a record. Still a
  caller precondition violation; pinned as a defined outcome by
  `a_leaked_reservation_skips_its_sequence_rather_than_regranting_it`.
- `is_full` reads the real cursors and can disagree with a stale cache;
  `claim` resolves the disagreement by refreshing before it refuses. Both
  answers of the check hold — a full report means the next claim fails, a
  not-full report means it succeeds.

## Measurement (Pi 5, aarch64; baseline `spsc_base` on benches, compared on the spike; rtrb, the untouched control, held within ±1.6% everywhere)

| two-thread `push1_popN` | 64 | 1024 | 16384 |
|---|---:|---:|---:|
| ring_spsc | 85.6 M/s (+588%) | **192.3 (+1434%)** | **334.1 (+2536%)** |
| rtrb (control) | 140.3 (−0.5%) | 188.8 (−1.6%) | 244.2 (−0.8%) |

Parity at 1024, a 37% lead at 16384 — the 15× gap the review recorded is
closed, and closed from behind: the crate now leads the reference it was
chasing at large capacity.

- `spsc_payload`: parity at 8 bytes (194.0 against 192.6); still 2.3–2.7×
  behind at 64 and 256 bytes — the residue is payload-copy shaped, not
  protocol shaped.
- Single-threaded fill/drain: +41.8…+46.3% (`push1_pop1`) and
  +116…+122% (`push1_popN`) — the cache pays on the uncontended path too.
- `push_full`: −35% — the cached full-check refuses without the load.
- The two honest regressions, both small: `push1_pop1` two-thread −4.1…−4.3%
  (the strict-alternation mode pays the bookkeeping and buys nothing — there
  is no line transfer to avoid when the ends alternate), and the
  single-threaded `push_pop` pair +25.2% (15.7 against 12.5 ns).
- Mechanism counters over the same 10 s window (`push1_popN/1024`):
  instructions 8.6G → 95.7G at flat cycles (47.6G), IPC 0.18 → 2.01,
  L1-dcache misses per record 4.9 → 0.34, branch-mispredicts per record
  0.96 → 0.01.
- Correctness gate: 30 integration tests (28 + 2 new), 26 doctests,
  5 compile_fail, loom 4/4 with the new cached-head model, clippy
  `-D warnings`, fmt, workspace green (`ring_handle`'s trybuild stderr
  re-blessed — additive, nothing shifted: with `rust-src` installed, as
  this branch's CI now does, rustc quotes the sysroot sources behind its
  `$RUST/...` spans, so the fixture gained the `PhantomData` and
  `Scope::spawn` snippets and no diagnostic text moved).

## Verdict

**GO.** The hypothesis is confirmed by counters and by the clock: the
two-thread gap the review recorded is the consumer-cursor read on the push
path, and caching it — lawfully, inside the `&mut` end — closes it to parity
at moderate capacity and a 37% lead at large capacity, on the architecture
where the gap lived. Single-threaded throughput improved as a side effect,
the refusal micro got cheaper, and the two regressions are confined to the
strict-alternation shapes at 3–25% where the cache buys nothing.

## Consequences

- Promotion: a PR from `spsc-cached-head-spike` carries the prototype, the
  model and this record. The x86 no-regression check took three pairs, and
  none of them held the untouched controls flat on the two-thread rows (the
  third, the best, still swung rtrb from −40% to +149% at one capacity), so
  the honest x86 statement is bounded: within that noise the cached head
  shows no systematic regression — its rows sit inside the control band, and
  single-threaded fill/drain moves in the same direction as the Pi
  (+5…+26%). The x86 platform was never where the gap lived (1.2–1.5×
  there), and the repository's Linux CI runs the full suite on the PR as a
  further gate.
- The remaining honest gaps, recorded rather than claimed away: payload
  64/256 bytes (2.3–2.7× — payload-copy shaped), `push1_pop1`'s strict
  alternation (−4%), and the consumer-side branch mispredicts (secondary;
  a consumer/driver-shaped follow-up).
- H-B — weakening the remaining orderings under loom-proof — is unblocked as
  a separate follow-up: the fast path no longer reads the consumer line, so
  the `Acquire` loads left on the hot path are the consumer's own.
