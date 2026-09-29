# integration

Three crates below, four above — and the four above split two ways that matter:
two depend on this crate to build something, two only to test against it.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Three Dependencies and Two Dependents](001_three_dependencies_and_two_dependents.md) | What each of the three supplies, and what the two real dependents take |
| 002 | [The Other Half (`ring_barrier`)](002_the_other_half_of_feature_178.md) | `ring_barrier` — same design, same cursors, opposite direction, and the one dependency that differs |

### The Graph

```
        ring_types ──┐
        ring_cursor ─┼──► ring_gating ──► ring_mpsc   (owns a GatingSet)
        ring_seqno ────┘         │      └─► ring_claim  (borrows one)
                               │
                     dev-only: ring_barrier, ring_publish
```

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^\[dependencies\]/,/^\[/p' ring_gating/Cargo.toml
grep -rln 'ring_gating *=' */Cargo.toml | grep -v ring_gating/
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| GT19 | The four dependents | n/a — observation | Two of them are `[dev-dependencies]`, and `ring_barrier`'s `Cargo.toml` says why in a comment rather than leaving it to be inferred — "the library itself never names it" |
| GT20 | `ring_seqno` | n/a — observation | Declared directly and also reachable through `ring_cursor`, which depends on it too. The direct edge exists for one function, `free_slots`, called from one line of `headroom` — the narrowest justification for a manifest entry anywhere in this crate |
| GT21 | `ring_gating` and `ring_barrier` | n/a — observation | They share two of three dependencies; the third is `ring_seqno` here and `ring_wait` there, which is exactly the split between computing a bound and waiting on one |
| GT63 | The cost of owning | n/a — observation | Owning a `Vec` instead of borrowing a slice costs `const fn`s the sibling gets for free: 1 of 11 methods is `const` here, 4 of 9 in `ring_barrier`. The ownership decision that makes this half safe to share is also what puts most of its surface out of reach of a `const` context |
| GT22 | `ring_barrier` | n/a — observation | `ring_barrier` is also a dependent of this crate, and the edge runs one way. The two halves are peers by design and parent-and-child in the build graph, and only the second relationship is checkable from the source |
