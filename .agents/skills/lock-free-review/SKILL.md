---
name: lock-free-review
description: Review checklist for ring code that touches atomics, memory orderings, unsafe, cursors, gating, claim/publish/commit or the slot lifecycle. Use when writing or reviewing such a change, before opening the PR.
---

# Lock-free review

Applies to `ring_atomic`, `ring_cursor`, `ring_gating`, `ring_claim`, `ring_publish`,
`ring_barrier`, `ring_consume`, `ring_batch`, `ring_store`, `ring_slot`, `ring_spsc`,
`ring_mpsc`, `ring_core`, `ring_handle`, `ring_tls`, `ring_flush`, `ring_shutdown`, `ring_wait`.
Background: the module docs of `ring_mpsc` and `ring_spsc` (`src/lib.rs`) — the invariants and
the ordering constants that carry them.

## Orderings

- [ ] Every ordering is a named constant (`GATING`, `PUBLISH`, `OBSERVE`, `COMMIT`, `HANDOFF`,
      `OWN`) or carries a comment naming the load/store it pairs with.
- [ ] Each `Release` store has an `Acquire` load of the same atomic that needs it, and vice versa.
- [ ] `Relaxed` only for a single writer reading back its own value (`OWN`) or for counters.
- [ ] No `SeqCst` added to make a test pass. x86 hides weak orderings; aarch64 does not.
- [ ] New atomics come from `ring_atomic` so loom sees them.

## Protocol

- [ ] Claims stay gated by `slowest consumer + capacity`; the shared claim re-reads the consumer
      cursor on every CAS retry.
- [ ] A cursor cache (`ring_spsc`: `Producer::cached_head`, `Consumer::cached_produced`;
      `ring_mpsc`: `PrimaryProducer::cached_head`) only lags the real cursor: it is refreshed at
      `GATING` and never extrapolated. Staleness may refuse a ring that has room; it never grants
      a slot the consumer still holds or offers an unpublished record.
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

- [ ] Crate is on `bench_harness/gate/declared/ring/unsafe_allowlist.txt` (never add one on your
      own); the crate's `docs/workaround/` explains the new use.
- [ ] One operation per block; `// SAFETY:` names what makes it sound — no aliasing `&mut`, the
      index in range, the happens-before edge (by constant name) that makes the data visible.
- [ ] `UnsafeCell` per slot, never around the buffer. `unsafe impl Send/Sync` justified next to
      the impl — `ring_spsc` and `ring_mpsc` have the same line for different reasons.

## Evidence

- [ ] A loom model covers the new interleaving, and it fails with the change reverted or the
      ordering weakened. Models are the `--test` files listed in `.github/workflows/loom.yml`:
      `RUSTFLAGS="--cfg loom" cargo test -p <crate> --test <file>`.
- [ ] Where loom cannot see the edge, the comment at the load argues it. Loom linearises each
      step, so a cursor-cache refresh weakened to `Relaxed` still passes there.
- [ ] A test fails without the change.
- [ ] `ring_handle/tests/ui/*.stderr` unchanged, or the change explained.
- [ ] The module docs still state what the code does — the `# Invariant:` sections and the tests
      they cite as **Enforced by**.

## Output

Findings ordered by severity, each with `file:line`, the interleaving or input that breaks it,
and the missing evidence. Say explicitly which boxes could not be checked and why.
