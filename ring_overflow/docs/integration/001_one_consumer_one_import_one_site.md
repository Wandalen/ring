# Integration: One Consumer, One Import, One Site

### Scope

**Purpose:** Record this crate's entire integration surface — every crate that
declares it, every name it exports across that edge, and every line that uses one.

**Responsibility:** The dependency edges in both directions, and the four lines in
`ring_core` that constitute the whole of this crate's production use.

**In Scope:** `ring_overflow/Cargo.toml`;
`ring_core/src/lib.rs:80`, `:411`, `:413-414`.

**Out of Scope:** The `ring_stats` edge no production path traverses is
[`integration/002`](002_the_stats_edge_and_the_function_nobody_imports.md). What
the call site does with the outcome is
[`algorithm/002`](../algorithm/002_where_the_third_outcome_goes.md).

---

## The Whole Surface, Both Directions

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what this crate depends on --'
sed -n '/^\[dependencies\]/,/^\[/p' ring_overflow/Cargo.toml | command grep '^ring_'
echo '  -- the only crate that declares it --'
for f in */Cargo.toml; do command grep -l '^ring_overflow' "$f"; done 2>/dev/null | sed 's|ring/||;s|/Cargo.toml||'
echo '  -- and every mention of this crate anywhere in it --'
command grep 'ring_overflow\|would_resolve\|Resolution' ring_core/src/lib.rs
```

Live output:

```
  -- what this crate depends on --
ring_types = { path = "../ring_types" }
ring_stats = { path = "../ring_stats" }
  -- the only crate that declares it --
ring_core
  -- and every mention of this crate anywhere in it --
//! crates (`ring_wait`, `ring_overflow`, `ring_types`, `ring_bench`,
use ring_overflow::{ would_resolve, Resolution };
      Err( record ) => match would_resolve( self.overflow )
        Resolution::DroppedIncoming => Ok( () ),
        Resolution::EvictedOldest | Resolution::Refused => Err( record ),
```

---

### OV17 — Two Edges In, One Edge Out, and Four Lines of Contact

`ring_overflow` declares two dependencies — `ring_types` for the policy and the
error, `ring_stats` for the counters — and exactly one crate in the workspace
declares `ring_overflow`: `ring_core`.

That consumer's entire use of this crate is four lines. One `use` at `:69`
importing two of the three exported names, one `match` scrutinee at `:400`, and
two arms at `:402-403`. Nothing else in `ring_core` mentions the crate, the type,
or either function.

**Finding.** So a crate with a two-crate dependency footprint, a public enum, four
public functions, and 305 lines of tests exists to serve four lines in one
consumer — and those four lines use one function.

The measure worth stating is the ratio between what the crate offers and what is
taken: four functions exported, one imported; two predicates exported, neither
imported ([`api/002`](../api/002_two_predicates_and_no_caller.md) § OV7). The
variant count is the one place the ratio is now 3-of-3: the consumer's arms named
`DroppedIncoming` and a `_` wildcard when this instance was written, and the
wildcard was replaced by `EvictedOldest | Refused` in the same commit that made
this crate's own two predicates exhaustive
([`invariant/001`](../invariant/001_the_mapping_is_total_and_injective.md) § OV22).
The surface is not wrong, but outside that one enum nothing in the workspace has
yet needed more than the smallest corner of it.

---

### OV18 — The Consumer Imports the Pure Half and Not the Effectful One

`ring_core` imports `would_resolve` and `Resolution`. It does not import `resolve`,
which is the function this crate's own documentation presents as the main entry
point — the one that applies a policy *and* records the outcome.

The consumer instead computes the resolution without counters and does its own
accounting elsewhere. So the two halves of the pair have opposite fates: the pure
one carries all the production traffic, and the effectful one carries none.

**Finding.** This inverts the relationship the source describes. `would_resolve`
is documented as "the pure half of `resolve`, for callers deciding what a policy
*would* do — a factory validating a configuration, a test tabulating the mapping
— rather than handling a real full-ring event."

But `ring_core:400` is not a factory and not a test. It is exactly a real
full-ring event, handled by the function documented as being for the other case.
Either the consumer is using the wrong half, or the doc comment describes an
intended division of labour the one consumer did not adopt — and no document
records which.

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -A5 -F 'Also the right half for a caller that holds no' ring_overflow/src/lib.rs
```

Live output:

```
/// **Also the right half for a caller that holds no `&RingStats` at all.**
/// The pattern's real division is purity, not purpose: a caller handling a
/// genuine full-ring event still belongs here if it has nothing to record
/// into, since [`resolve`] cannot be called without one. That is why this
/// crate's own sole consumer takes this half for a real full-ring event
/// rather than a hypothetical one.
```

**Disposition:** applied — `would_resolve`'s doc comment in
`ring_overflow/src/lib.rs` now states explicitly that the pure half
also serves a caller handling a genuine full-ring event when it holds no
`&RingStats` to record into, which is why this crate's own sole consumer
takes it for a real event rather than a hypothetical one — closing the gap
between the doc's stated purpose and the one production caller's actual use.
Now prints: `Also the right half for a caller that holds no`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/002`](002_the_stats_edge_and_the_function_nobody_imports.md) | The dependency that edge does not carry |
| [`algorithm/002`](../algorithm/002_where_the_third_outcome_goes.md) | What the consumer does at `:400` |
| [`api/002`](../api/002_two_predicates_and_no_caller.md) | The exported names nothing imports |
| [`lifecycle/001`](../lifecycle/001_a_resolution_computed_matched_and_discarded.md) | The life of the one value that crosses |

### Sources

| Fact | Where |
|------|-------|
| Both outgoing dependencies | `ring_overflow/Cargo.toml` |
| `ring_core` as sole declarer | Census above |
| The import, the scrutinee, the arms | `ring_core/src/lib.rs:80`, `:411`, `:413-414` |
| `would_resolve`'s documented purpose | `ring_overflow/src/lib.rs:208-212` |

### Tests

| Test | Covers |
|------|--------|
| `resolve_agrees_with_would_resolve` | That either half would serve the consumer identically here |
| `would_resolve_touches_no_counters` | The property the consumer relies on by choosing the pure half |
| *(to create)* | No test in either crate pins the import list, so the edge can widen unobserved |
