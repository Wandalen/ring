---
name: concurrency-debug
description: Structured debugging for ring — a hang, a lost, duplicated or reordered record, stale slot data, a flaky test, a loom model failure or a trybuild mismatch. Use when behaviour diverges from a crate's documented invariant.
---

# Concurrency debug

## 1. Reproduce

- The exact command, crate, features, thread count, capacity and overflow policy.
- Flaky? Loop it: `for i in $(seq 200); do ./verb/test_only crate::<c> filter::<t> || break; done`.
- Try `--release` too: timing changes which interleavings show up.
- Can the interleaving be written as a loom model? Then do that first — a deterministic failure
  beats a thousand runs (`./verb/loom crate::<c>`).

## 2. Classify

| Symptom | Check first |
|---|---|
| "Lost" record | Overflow policy. Under `DropNewest`, `try_push` returns `Ok` for a dropped record (`ring_testkit::Outcome::vanished`). Only then suspect a race |
| Hang | A `Claim` dropped without publishing strands the consumer; a consumer that never commits stalls producers at `slowest + capacity`; a wait predicate that can never become true |
| Duplicate | Commit moved backwards, or a stamp compared with `>=` so a previous lap reads as published |
| Reorder | MPSC drains in `Seq` order; the interleaving of producers is a race and not a bug. Crossbeam order is unspecified |
| Stale or torn data | An ordering too weak on one side of a pair; an atomic that bypasses `ring_atomic` (invisible to loom) |
| Only on aarch64 / only in CI | A missing `Release`/`Acquire` pair; or a test asserting on timing |
| Wrapped counters look healthy | Saturating `Seq` reads; run `ring_debug::check` on the live ring |
| trybuild mismatch | `-->` path or `note:` lines vs a real `error[E…]` change (see `verify`) |

## 3. Instrument

- `ring_debug::check` / `Watch`: consumer ahead, more than a lap, cursors moving backwards.
- `ring_trace`: an operation log with no cost when disabled.
- `ring_stats`: drops per policy.
- `ring_testkit::Script` for a deterministic single-thread replay of the claim/drain sequence.

## 4. Fix

- Root cause, not symptom. Never strengthen to `SeqCst` or add a sleep to make it pass.
- The failing test (or loom model) lands with the fix, and fails with the fix reverted.
- Record what was learned where the next reader looks: a `pitfall/` or finding in the crate's
  `docs/` (`doc-corpus` skill), not only the commit message.
- Run the `lock-free-review` checklist over the fix.
