# Invariant Doc Definition

### Scope

- **Purpose**: State the properties any winning ring mechanism must hold, so picking an implementation never means renegotiating the contract.
- **Responsibility**: Document `ring_mpsc`'s own invariants and what enforces each at the contract grain.
- **In Scope**: Properties of the producer/consumer contract — publication visibility, exactly-once delivery, ordering, reproducibility.
- **Out of Scope**: Which concrete ring pattern provides them, which stays this family's own open verdict; a prospective consumer's own determinism guarantees, which are that consumer's to state.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Single-Consumer Total Order](001_single_consumer_total_order.md) | Every published element reaches the one consumer exactly once, in one reproducible order | 🔄 |
| 002 | [Publication Ordering — Claim Acquire, Publish Release](002_publication_ordering.md) | The two Release/Acquire handoff pairs that make a published slot's payload actually visible, and a drained slot actually safe to reuse | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc/docs/invariant
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### MP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| MP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  4
# rows in the table below:  4
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| MP22 | consumer uniqueness | n/a — observation | `ends` takes `&mut self` and `split` takes `&mut Ends`, so a second consumer cannot be obtained while a first is alive. |
| MP23 | handle identity | n/a — coverage | The test asserting that every handle points at the same ring is the one that would catch a split producing handles onto different allocations. |
| MP24 | the ordering checks | n/a — unenforced | The loom model needs `--cfg loom` and the 60-run mutation is manual, so an ordinary run checks this invariant only at the level of the constants' declared values. |
| MP25 | `COMMIT` | n/a — observation | Earlier design material advances the read cursor `Relaxed`; this crate uses `Release` and says why at the constant. |
