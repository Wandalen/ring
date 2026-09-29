# integration

Three dependencies out, one dependent in, and two edges that live only in test
manifests. The crate sits at Tier 5, one step above `ring_wait`, and declares
nothing `ring_wait` does not already declare.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Three Dependencies and One Dependent](001_three_dependencies_and_one_dependent.md) | Every edge in and out, the dependent that reimplements a method it holds, and the crate that names a barrier without one |
| 002 | [The Dependency That Is Not `ring_seqno`](002_the_dependency_that_is_not_ring_seq.md) | Why the third slot is `ring_wait` where the sibling has `ring_seqno` — one clamp, one crate |

### The Forest, Locally

| | Tier | Depends on |
|--|-----:|------------|
| `ring_types` | 0 | — |
| `ring_seqno` | 1 | `ring_types` |
| `ring_cursor` | 3 | `ring_types`, `ring_seqno`, `ring_atomic`, `ring_align` |
| `ring_wait` | 4 | `ring_types`, `ring_cursor` |
| **`ring_barrier`** | **5** | **`ring_types`, `ring_cursor`, `ring_wait`** |
| `ring_consume` | 5 | `ring_types`, `ring_cursor`, `ring_barrier`, `ring_seqno` |

Tiers are each crate's own declaration in its module documentation, not a
workspace-wide ordering — `ring_consume` declares Tier 5 and depends on
`ring_barrier`, which declares the same. The one edge that matters for the
family's acyclicity is the reverse one, and it does not exist.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -ln 'ring_barrier' ring_*/Cargo.toml          # 3 — publish (dev), consume, itself
grep -vE "^[[:space:]]*(///|//!)" ring_barrier/src/lib.rs | grep -c 'ring_seqno' || true   # 0
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BR2 | `ring_consume` | n/a — duplication | `ring_consume::Consumer::available` re-derives `Barrier::available` term-for-term, and `ring_seqno::pending` is its only `ring_seqno` use — the whole `ring_consume → ring_seqno` edge exists to spell one method by hand |
| BR17 | `ring_flush` | n/a — inconsistency | `ring_flush` names `FlushPolicy::OnBarrier` and `FlushCause::Barrier`, declares no edge to this crate, and its own module doc explains why an edge would not help |
| BR33 | `ring_barrier` | n/a — observation | Three manifests name this crate and only `ring_consume` links it — `ring_publish`'s entry is a dev-dependency, so the handshake test the module doc cites as the reason for the slice signature lives in a crate that, in a release build, does not depend on this one at all |
| BR34 | family | n/a — observation | `ring_seqno` exports the family's five sequence-arithmetic helpers and every one takes a capacity, so the crate named for sequence arithmetic is unreachable from the crate whose only job is a sequence subtraction — for the exact reason that subtraction must stay capacity-free |
