# Topic 6: `no_std` / embedded-readiness audit

## The gap

Only 3 of the 34 crates currently declare `no_std`: `ring_types`,
`ring_overflow`, `ring_stats`. A hand-built, latency-critical ring buffer is
exactly the kind of primitive that's valuable in `no_std`/embedded
contexts — right now that's almost entirely unexplored beyond those three.

## Likely infeasible, and that's fine — document it

The root [`../../README.md`](../../README.md) itself flags two crates as
probably needing `std` no matter what: `ring_wait` (parking/wait strategies
typically need `std::thread`) and `ring_tls` (thread-locals). A documented
**negative** verdict — "audited, and here's specifically why this crate
can't go `no_std`" — is just as valuable a deliverable as a positive one.
Don't skip a crate just because the answer looks like "no."

## Deliverables

- A crate-by-crate audit of the dependency graph, worked bottom-up from
  Tier 0 (`ring_types`, `ring_align`) through the tiers — a crate can only
  be `no_std` if everything it depends on is too, so the audit order
  matters
- `no_std` feature-gated where it's achievable, with the existing 3
  declared crates (`ring_types`, `ring_overflow`, `ring_stats`) as the
  reference pattern for how to do it
- Explicit, written justification for every crate marked infeasible (start
  with `ring_wait` and `ring_tls`, then check anything that transitively
  depends on either of them, e.g. `ring_barrier`, `ring_shutdown`,
  `ring_flush`)
- A CI job ([Topic 1](01_ci_cd.md)) building the `no_std` subset with
  `--no-default-features`, so a future change can't silently regress a
  crate that was previously audited as `no_std`-clean
