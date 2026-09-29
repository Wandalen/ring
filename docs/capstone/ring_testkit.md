# `ring_testkit` — capstone notes

**Role:** Determinism-test fixtures driving scripted claim and drain sequences
**Tier:** 8/10
**Depends on:** `ring_core`, `ring_tls`, `ring_shutdown`, `ring_config`, `ring_types`
**Depended on by:** none — a leaf
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited — test-fixture crates commonly stay `std`-only even when the crate under test is `no_std`; not necessarily worth forcing
**loom:** covered
**External API surface:** no — internal to the family, used by the family's own test suites

## Most relevant topics

- **Topic 8 (extraction debt / loom):** already `loom`-covered, and this
  crate's whole purpose is scripted determinism testing — it may be the
  most natural home for *new* `loom` scenarios written for other crates as
  part of extending coverage, rather than each crate rolling its own.
- **Topic 2 (fuzzing):** "scripted claim and drain sequences" is close in
  spirit to a fuzz corpus's seed inputs — worth checking whether existing
  scripts can be repurposed as fuzz seeds instead of written twice.
