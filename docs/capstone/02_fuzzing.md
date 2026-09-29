# Topic 2: Fuzzing the lock-free core

## The gap

No `fuzz/` directory, no `cargo-fuzz` (or AFL) anywhere in the repo. `loom`
checks a bounded number of thread interleavings for the handful of crates
that use it; it says nothing about the *input* space — capacity edge cases,
claim/publish/commit sequences that wrap the ring, overflow behavior at the
boundary.

## Primary targets

The three crates actually carrying `unsafe`
(`../../bench_harness/gate/declared/ring/unsafe_allowlist.txt`):

- **`ring_spsc`** — single-producer single-consumer ring API
- **`ring_mpsc`** — multi-producer single-consumer ring API (the harder
  case — concurrent claim storms are the most likely place to find a real
  race)
- **`ring_core`** — the crate that *composes* SPSC and MPSC together with
  overflow/batch/event support. Notably, `ring_spsc` and `ring_mpsc` are
  individually `loom`-covered but `ring_core` is not — a composition bug
  that only shows up when the two backends' invariants interact through
  this crate wouldn't be caught by either backend's own coverage. This is
  the single highest-value fuzz target in the family.

## Supporting targets worth seeding

Crates that feed the primary three and are cheap to fuzz independently even
though they carry no `unsafe` of their own: `ring_seqno` (wraparound
arithmetic), `ring_store`/`ring_slot` (the storage array both `ring_spsc`
and `ring_mpsc` are built over), and `ring_overflow` (full-ring policy —
exactly what should happen when a fuzz run deliberately overflows the ring).

## Deliverables

- A `cargo-fuzz` harness targeting `ring_spsc`, `ring_mpsc`, and `ring_core`'s
  public APIs
- A documented corpus of interesting seed inputs (capacity boundaries, wrap
  sequences, overflow triggers) — check whether `ring_testkit`'s existing
  "scripted claim and drain sequences" fixtures can be repurposed as seeds
  instead of writing them twice
- A CI job (wired in by [Topic 1](01_ci_cd.md)) running a short fuzz pass on
  every PR, with longer scheduled runs separate from the PR-blocking job

This is the topic most likely to turn up a real bug in code everyone
currently trusts.
