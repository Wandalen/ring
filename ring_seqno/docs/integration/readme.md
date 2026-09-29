# integration

What this crate depends on, what depends on it, and how its functions actually
travel.

### Overview Table

| ID | Name | Direction |
|----|------|-----------|
| 001 | [One Dependency, Five Declared Dependents](001_one_dependency_and_five_declared_dependents.md) | Both — the manifest picture, and where it is wrong |
| 002 | [How the Fold Crossed Four Tiers](002_how_the_fold_crossed_four_tiers.md) | Outbound — the mechanism, with its counter-case |

### The Two Answer Different Questions

001 is the manifest census: who names this crate in a `Cargo.toml`, and whether
they use it. It is a mechanical question with a mechanical answer, and the answer
does not match the manifests.

002 is the interesting one. Every function here is a free function, and free
functions are the shape this family has repeatedly failed to share — the
cache-line predicate forked, the gating ordering constant is restated four times.
`ring_seqno`'s did not. 002 works out why, and finds the mechanism in the
*consumers* rather than in anything this crate did.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SQ17 | `ring_atomic`'s edge | n/a — unenforced | `ring_atomic` declares `ring_seqno` in `Cargo.toml` and asserts the edge in three more documents; nothing in the crate uses it, and `unused_crate_dependencies` is not enabled anywhere in the workspace |
| SQ18 | Direct callers | n/a — observation | Only `ring_gating` and `ring_batch` call these functions directly; every other consumer reaches them through `ring_cursor`'s wrappers, and `ring_batch` — the one that bypasses the wrapper layer — is also the one that wrote a redundant cast around the result |
| SQ19 | Crossing a manifest boundary | n/a — observation | A free function crosses a manifest boundary when each tier has a reason to wrap it — a type change supplies that reason, but does not keep the call: tier 2 stopped calling tier 1 in `b7e075ca` because the adaptation cost an allocation per call |
| SQ20 | `Barrier::available` | n/a — duplication | `Barrier::available` has no callers outside its own crate, while `ring_consume` — which holds a `Barrier` — re-expands its body inline |
| SQ21 | `distance_to` density | n/a — observation | The family calls `Seq::distance_to` fourteen times and four of those are here, making a 136-line crate the densest consumer of the one span primitive everything else is built on |

### Regenerate

One dependency down, five declared dependents up:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^\[dependencies\]/,/^\[/p' ring_seqno/Cargo.toml
command grep -rl 'ring_seqno *=' */Cargo.toml | command grep -v ring_seqno/
```

Live output:

```
[dependencies]
ring_types = { path = "../ring_types" }

[lints]
ring_batch/Cargo.toml
ring_consume/Cargo.toml
ring_cursor/Cargo.toml
ring_gating/Cargo.toml
```
