---
name: lock-free-review
description: Review checklist for ring code that touches atomics, memory orderings, unsafe, cursors, gating, claim/publish/commit or the slot lifecycle. Use when writing or reviewing such a change, before opening the PR.
argument-hint: "[diff, crate, or file]"
---

# Lock-free review

Applies to `ring_atomic`, `ring_cursor`, `ring_gating`, `ring_claim`, `ring_publish`,
`ring_barrier`, `ring_consume`, `ring_batch`, `ring_spsc`, `ring_mpsc`, `ring_core`, `ring_tls`,
`ring_flush`, `ring_shutdown`, `ring_wait`. Background: `ARCHITECTURE.md` § How a record moves,
`ring_mpsc/docs/invariant/002_publication_ordering.md`.

## Orderings

- [ ] Every ordering is a named constant (`GATING`, `PUBLISH`, `OBSERVE`, `COMMIT`, `HANDOFF`,
      `OWN`) or carries a comment naming the load/store it pairs with.
- [ ] Each `Release` store has an `Acquire` load of the same atomic that needs it, and vice versa.
- [ ] `Relaxed` only for a single writer reading back its own value (`OWN`) or for counters.
- [ ] No `SeqCst` added to make a test pass. x86 hides weak orderings; aarch64 does not.
- [ ] New atomics come from `ring_atomic` so loom sees them.

## Protocol

- [ ] Claims stay gated by `slowest consumer + capacity`, re-read on every CAS retry.
- [ ] Every path from a claim reaches publish. A dropped `Claim` strands its range and stalls
      the consumer forever — look for early returns and `?` between claim and publish.
- [ ] The slot is written before it is published; nothing reads a slot before its publish
      is observed.
- [ ] MPSC drain compares stamps for equality with the expected `Seq` (never `>=` or
      `!= UNSTAMPED`); drain stops at the first gap.
- [ ] Commit only moves forward and never past what was drained.
- [ ] One consumer: `Consumer` stays `!Clone` and `!Sync`; new "must not compile" rules get a
      trybuild case in `ring_handle/tests/ui/`.
- [ ] Overflow: `DropNewest` returns `Ok` for a dropped record — callers must not read `Ok` as
      kept; `DropOldest` stays refused on in-house backends.

## Arithmetic and layout

- [ ] `Seq` stays unfolded; the index comes from `ring_index` (`seq & mask`), never `%`.
- [ ] Capacity remains a power of two (`ring_types::Capacity`).
- [ ] Saturating reads cannot hide a broken ring as "empty and healthy" — `ring_debug` checks.
- [ ] Contended atomics sit on their own line (`CacheAligned`, `PaddedCursor`); no new hot field
      shares a line with a cursor. MPSC stamps are unpadded on purpose.
- [ ] No allocation, lock or unbounded wait on the write path; waits go through `ring_wait`.

## `unsafe`

- [ ] Crate is on the allowlist; `docs/workaround/` explains the new use.
- [ ] One operation per block; `// SAFETY:` says why no aliasing `&mut` exists and which
      happens-before edge (by constant name) makes the data visible.
- [ ] `UnsafeCell` per slot, never around the buffer. `unsafe impl Send/Sync` justified next to
      the impl — `ring_spsc` and `ring_mpsc` have the same line for different reasons.

## Evidence

- [ ] A loom model covers the new interleaving (`./verb/loom crate::<name>`), and it fails with
      the change reverted or the ordering weakened.
- [ ] A test fails without the change.
- [ ] `ring_handle/tests/ui/*.stderr` unchanged, or the change explained.
- [ ] The crate's `docs/invariant/` or `docs/algorithm/` instance still states what the code
      does.

## Output

Findings ordered by severity, each with `file:line`, the interleaving or input that breaks it,
and the missing evidence. Say explicitly which boxes could not be checked and why.
