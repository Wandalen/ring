# The claim gates on the boundary slot's release marker, if the spike's numbers hold

Status: Rejected for promotion — the stated goal (the one-producer gap) is
unmet; the reproducible effects are a mid-band win paid for on single-thread
fill/drain and oversubscription. Two A/B pairs on record; the first was
drift-contaminated and its magnitudes are withdrawn.

## Context

The batched claim removed the contention cliff (ADR 001), but the per-record
path still pays one shared cache line per claim: `Claimer::claim` loads the
consumer cursor (`headroom → slowest()`, `Acquire`) and the consumer keeps
rewriting that line. At one producer the sweep measured 32.5 M/s against
`ArrayQueue`'s 70.7, and 19.6 ns per single-threaded push+pop pair against
rtrb's 4.4 — while the crate wins the single-threaded fill/drain, which says
the instructions are not the cost; the cross-core line transfer is. The
hypothesis, and the spike that tests it, were recorded as open in ADR 001's
alternatives.

The spike is a prototype on its own branch, not a landing. Its question:
does removing the consumer-cursor read close the per-record gap without
regressing the batched path, with the whole correctness suite green?

## Decision under test

- **Release markers.** `Batch::drop` stores each drained slot's stamp as its
  release marker — the sequence it held advanced by `capacity + 1`, the
  next-lap sequence the slot is free for — `Release`, in issue order, before
  the commit. The consumer touches those lines taking the payload anyway.
- **Boundary gate.** The claim loads the claim cursor, then reads the grant's
  boundary slot once, `Acquire`. A first-lap boundary is free without a read.
  A later lap grants exactly on the previous lap's marker. No consumer-cursor
  read on the claim path.
- **Ask-or-fail batches.** `claim_batch` grants the whole ask or nothing,
  replacing `claim_up_to`'s adaptive width, which priced a partial grant from
  the consumer cursor. An ask wider than the capacity is `BatchTooLarge`.
- **Unchanged.** The claim cursor and its CAS loop, `contiguous_end`'s
  equality rule, publish-on-drop guards, the consumer's scan,
  `free_capacity`/`headroom` (still cursor-based, still advisory),
  `ring_claim` and `ring_publish` (untouched).

## Why a marker rather than a clear (the soundness note)

The sketch this spike started from cleared stamps to `UNSTAMPED` on commit.
That is unsound, and the spike found it before measuring: a claim the cursor
has passed but whose guard is still live leaves its stamp at `UNSTAMPED`
(first lap), and a gate that reads a cleared stamp cannot tell that from
free. The claim cursor runs a full lap ahead of publication between one
producer's claim and its publish, so it would regrant the slot out from
under the live guard — two exclusive `&mut`s to one slot. The marker names
the sequence it freed; the gate accepts only the previous lap's marker; the
held slot is refused by arithmetic alone, with no claim-side stamp write and
no consumer-cursor read. Pinned by
`a_slot_whose_previous_lap_claim_was_never_published_is_never_regranted` and
a loom model of the same race, both verified red under the cleared-stamp
mutation.

The memory-ordering pair is one direct edge: the consumer's marker store
(`Release`, after the payload take) and the gate's stamp load (`Acquire`).
Releases of a batch run in issue order before the commit, so a boundary
marker implies every earlier slot of the run is released too — the prefix
argument the boundary-only check rests on. The claim cursor cannot outrun
the release frontier by a full lap, because the boundary marker certifies
the previous lap's consumption exactly.

## Semantic deltas (spike, recorded)

- `claim_batch` is ask-or-fail: a ring with fewer than `max` slots free now
  answers `Full` where the adaptive grant answered with a prefix;
  `push_batch` is all-or-nothing and a wider-than-capacity ask is
  `BatchTooLarge`.
- `free_capacity`/`headroom` keep their cursor computation but can disagree
  with the gate in flight in either direction; the advisory contract is
  unchanged.
- `stamps()` observers see release markers between a slot's laps, where they
  previously saw the last published sequence.

## Verification gate (this spike)

- ring_mpsc: 43 integration tests (41 + 2 new), 33 doctests, 5 compile_fail,
  clippy `-D warnings`, fmt — green.
- Loom 6/6: the four existing publication models plus the release/reclaim
  edge and the never-regranted counterexample; both new models verified red
  under the cleared-stamp mutation.
- Workspace suite across the family — see the verdict below.

## Measurement (two A/B pairs, same host: 12 logical CPUs, Windows, QPC)

Both pairs compare the spike against a criterion baseline saved on `benches`
minutes earlier, same session, no cross-run absolute comparisons. The first
pair (2026-10-04, late evening) was taken on a machine still recovering from
hours of continuous compilation, and its base side measured fast: the
untouched control candidates drifted 19–23% at one producer (arrayqueue
−19.5%, sync_channel −28.8% at four) and up to 46% on the batch suite —
error bars the mpsc deltas cannot be read through. The second pair, taken
the same night on an idle machine, holds the controls flat (arrayqueue
±0.2–2.6% across the whole producers sweep), which is what makes it the
authoritative pair; it confirms the drift diagnosis by measuring the spike
side within noise of the first pair while its own base sits ~27 M/s at one
producer where the first pair's base sat at 41.

**The authoritative pair (`relspike2`), mpsc rows, change against the saved
baseline:**

| producers → | 1 | 2 | 4 | 8 | 11 |
|---|---:|---:|---:|---:|---:|
| per record (`push1_popN`) | **26.8 M/s (−2.8%)** | 19.4 (+29.4%) | 13.8 (+37.5%) | 8.8 (+16.3%) | 7.6 (+8.7%) |
| batched (`push32_popN`) | 263.9 (−12.3%) | 254.9 (+12.1%) | 245.8 (+34.0%) | 215.7 (−3.1%) | 203.5 (−10.2%) |
| capacity, 4 producers (`push1` / `push32`) | — | — | 13.4 (+32.2%) / 147.0 (+43.5%) | — | — |

And the reproducible costs, both pairs agreeing:

- **Single-threaded fill/drain: −8% to −15%** (all three capacities, both
  shapes) — the consumer's per-record marker store, with no contention to
  hide it in.
- **The refusal micro (`push_full`) doubled: +107–109%** — a refused claim
  reads the boundary stamp, a line the consumer owns at that moment. Real
  and reproducible, but it prices a path healthy workloads rarely hit.
- **Oversubscribed (24 producers on 12 CPUs): push32 −14.5%/−16.8% across
  both pairs** — a consistent ~−15%. The per-record row moved +1.4%/−29.9%
  across the pairs and is recorded as unknown rather than claimed either
  way; that suite's control rows drift too much to read it.
- Ping-pong one-way: flat across all runs.

**What the first pair claimed and the second withdraws:** the one-producer
regression (−35.7%) and the across-the-board batched regression
(−17% to −28%) were drift, not protocol. The clean pair reads the batched
sweep as mixed — +34.0% at four producers, −12.3%/−10.2% at the extremes,
−3.1% at eight — inside the suite's noise rather than a uniform loss.

## Verdict

**Not promotable, for the reason the spike set out to test — and for that
reason only.** The spike's stated goal was closing the per-record gap at one
producer; the clean pair holds that row flat (−2.8%, 26.8 against
`ArrayQueue`'s ~75 M/s). The hypothesis underneath — that the consumer-cursor
line read was the dominant per-record cost — is now cleanly refuted: with
the read gone and a sound marker protocol in its place, the one-producer
number does not move.

What the measurements do establish, reproducibly, is a trade rather than a
win: the two-to-eight-producer per-record band and the four-producer
capacity row improve 16–43% (per-slot gates read disjoint lines where the
old gate read one cursor line across every producer), paid for by −8…−15%
single-threaded fill/drain, a doubled refusal micro, and ~−15%
oversubscribed batched throughput. The crate's still-losing comparison to
`ArrayQueue` on the per-record path (4.8× behind at four producers) does not
close anywhere.

Per the measured-before-adopted requirement, the cursor-gated claim stays.
The honest record of the trade is this file; the revisit trigger narrows to
what would make it promotable — a release step the consumer can amortise
across a batch the way the cursor commit is, or a caller profile that
weights the two-to-eight-producer band heavily enough to buy the
single-thread cost. The spike branch (`mpsc-release-protocol-spike`,
6715e57 + the harness clamp + this record) is kept for reference and is not
promoted.

## Consequences

- `ring_mpsc` on `benches`/`master` keeps the cursor-gated claim and the
  adaptive batched grant; nothing lands from the spike. The spike branch
  also carries the one harness fix the ask-or-fail semantics forced (the
  mpsc adapter clamps its batch ask to the capacity — an ask wider than the
  ring is a permanent refusal under the gate, and the driver's produce loop
  spins forever on a permanent refusal); `master`'s adapter is untouched
  because the adaptive grant there never refuses a within-capacity ask.
- The soundness analysis above survives as the reference for any future
  stamp-gate attempt: the cleared-stamp design is unsound (the
  counterexample and its loom model are on the spike branch), and the marker
  design is the sound variant of it — measured, and rejected on the numbers.
