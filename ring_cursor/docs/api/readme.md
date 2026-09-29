# api

The public surface, split by who chooses the memory ordering.

### Overview Table

| ID | Name | Items | Ordering chosen by |
|----|------|------:|--------------------|
| 001 | [The Surface That Forwards](001_the_surface_that_forwards.md) | 7 | the **caller** |
| 002 | [The Surface That Decides](002_the_surface_that_decides.md) | 10 | the **crate** — where one is named at all |

Seventeen items, plus the two types themselves.

**The split is the crate's central tension, and it is deliberate.** A
`PaddedCursor` on its own is a cell: the caller genuinely has a choice, and a
defaulted ordering would make every benchmark meaningless. A `CursorPair`'s
readings are a gate: the caller is about to overwrite a slot on the answer, and
there is exactly one correct ordering to offer.

A reader who meets only one half concludes the other is a bug. Both halves are
argued in the module documentation for that reason, and the argument is
[`decisions/001`](../decisions/001_gating_is_fixed_not_a_parameter.md).

```sh
cd "$(git rev-parse --show-toplevel)"
grep -nE '^(pub |  pub )(const|fn|struct|use)|^impl' ring_cursor/src/lib.rs
```

**Expected: 20 lines** — 17 `pub` declarations and the three `impl` headers
between them. Those seventeen are *not* the seventeen items above, and the two
numbers matching is a coincidence of different arithmetic:

| | |
|---|---:|
| `pub` declaration lines the recipe prints | 17 |
| − the two types themselves, counted separately in the sentence above | −2 |
| − `new`, which is two lines per type but one item: `#[ cfg( not( loom ) ) ] pub const fn` and `#[ cfg( loom ) ] pub fn`, only ever one of them compiled | −2 |
| + the four `SeqCell` methods, which the pattern cannot match because a trait-impl method carries no `pub` | +4 |
| **items** | **17** |

**The four unmatched methods are why this recipe is a starting point and not the
census.** `load`, `store`, `fetch_add` and `compare_exchange` are on the
documented surface (→ [`001`](001_the_surface_that_forwards.md)) and no
`^pub`-anchored pattern will ever see them — a grep over declarations finds what
the crate *declares* public, which is a smaller set than what a caller can reach.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CU5 | `src/lib.rs:65` | **misleading doc** | The re-export was justified by "would put a dependency in six manifests to import one trait". Measured against library code, four crates import `SeqCell` through this crate — `ring_claim`, `ring_consume`, `ring_publish`, `ring_spsc`. Three more name it only inside doctests and `ring_shutdown` imports it from neither route. **Disposition: applied** — doc comment now says "four manifests" |
| CU6 | The `SeqCell` impl | n/a — observation | `load`, `store`, `fetch_add` and `compare_exchange` carry no `pub` — they are public because the trait is — so no `^pub`-anchored census reaches them. Any declaration count of this crate is therefore short by four of its most-called items |
| CU7 | `producer` and `consumer` | **latent hazard** | Both hand back `&PaddedCursor`, whose `SeqCell` impl forwards whatever ordering the caller names. So the type that fixes `GATING` for its own three readings returns a cursor with the choice restored, and `ring_spsc` uses exactly that to read at a different ordering through this crate's accessor |
| CU8 | `addr` and `on_distinct_lines` | n/a — observation | These two are the only non-`const` readings on either type; `producer`, `consumer` and `capacity` are all `const fn`. The cause is `addr`, which reads a runtime address and cannot be const-evaluated, and `on_distinct_lines` inherits it by calling `addr` twice. The split is forced rather than designed, and no document says so |
