# The producer claims k sequences per gate check and exchange, on the sweep's measured win

Status: Accepted

## Context

A per-record claim pays the contended step — one `Acquire` read of the
consumer cursor and one compare-exchange — once per record. A producer that
writes in groups pays it per member of the group. The `perf` suite now exists
to make the trade measurable instead of arguable, so the batched path was
measured against its own per-record path and against the off-the-shelf
alternatives before being adopted.

## Decision

`Producer::claim_batch( max )` grants `1..=max` contiguous sequences for one
gate check and one exchange; `Producer::push_batch` fuses the grant with
moving records out of a caller's `Vec`; `ReservedBatch` publishes the whole
grant on drop. The grant is adaptive — whatever headroom the gate allows at
the value the exchange runs against, `Full` when nothing — and the drop
publishes every sequence of the grant, written or not, in issue order:

| step | `claim( 1 )` per record | `claim_batch( k )` per grant |
|------|--------------------------|------------------------------|
| gate read (consumer cursor, `Acquire`) | one per record | **one per grant** |
| claim exchange (`AcqRel` CAS) | one per record | **one per grant** |
| publish (`Release` stamp store) | one per record | one per record |

The evidence (2026-10-02, one host: 12 logical CPUs, Windows, QPC clock about
36 ns, ten flat samples per point, self-consistent within the run — an
absolute baseline it is not): the batched path at capacity 1024 measured
232.2 → 198.0 M/s from one to eleven producers against the per-record path's
32.5 → 6.2 — 7.5× to 26× — with full spins per record at eleven producers
falling from 3.43 to 0.18, and 2.8 → 61.4 M/s oversubscribed at 24 producers
on 12 CPUs, the best candidate in the run. In the same sweep it led every
alternative from two producers up: 198–208 M/s against `ArrayQueue`'s flat
about 58, `sync_channel`'s 38–48, and a `Mutex<VecDeque>` batch that declined
234 → 57. At one producer `ArrayQueue` still leads the per-record row (70.7
against 32.5): the single-record claim path is unchanged by this work and
remains the crate's honest weak spot, recorded rather than claimed away.

## Alternatives considered

- **Per-record claim only.** The measured loser at every producer count, by
  the margins above.
- **Fixed-width grants.** A producer asking for more than the ring has would
  stall instead of making progress at the width the ring allows; the adaptive
  grant keeps a pressured producer moving.
- **Gating the claim off the slot stamps**, so the claim's fast path stops
  reading the consumer cursor entirely. That needs the consumer to mark slots
  on commit, which rewrites `contiguous_end`'s stamp-equality rule and every
  pitfall built on it. Recorded as open, not shipped speculatively.

## Consequences

- A held `ReservedBatch` parks the consumer behind its whole range — the
  drain stops at the grant's first unpublished sequence. The contract is
  claim, write, drop; the rustdoc on `claim_batch` states it.
- An unwritten offset of a dropped batch publishes an empty slot, one per
  offset — the single guard's defined outcome, multiplied by the grant width.
- The per-record `claim` is untouched, and a caller may mix both shapes.
- The comparative claim — batched beats per-record at every width, and beats
  every alternative in the sweep from two producers up — is supported by one
  host and one run. Absolute numbers are not portable; the sweep is
  reproducible with `verb/bench suite::mpsc` (see [`../../../perf/readme.md`](../../../perf/readme.md)).
