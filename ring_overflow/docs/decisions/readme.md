# decisions

Two choices shaped this crate and neither is written down as a choice. The first
scoped `Resolution` to a ring already known full, which is why a resolution is a
function of the policy alone and why `would_resolve` can be `const`. The second
sent one of the three policies out through the error channel instead of the
outcome channel, which is why the crate's central pair agrees on two policies out
of three.

Both are defensible. What the instances here record is that the reasoning exists
only as consequences in the code — a precondition in one doc line, an `Err` in one
match arm — and that a reader who does not reconstruct it has no way to tell a
deliberate scoping from an omission.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_fourth_variant_that_is_not_there.md) | The Fourth Variant That Is Not There | The full-ring precondition and what scoping to it buys |
| [002](002_fail_returns_an_error_not_a_resolution.md) | `Fail` Returns an Error, Not a Resolution | The third policy's two representations |

## Scoping to a State the Caller Established

`resolve`'s first documentation line is a precondition: it applies a policy "to a
publish that found the ring full." Everything else follows. There is no variant
for a normal successful publish, so a resolution never depends on capacity,
occupancy, or a cursor — only on which policy was configured.

That is what makes the mapping total, injective, and `const`-evaluable. A fourth
variant for the non-full case would have cost all three, and the crate would have
needed ring state to compute an outcome.

## One Event, Two Channels

For `DropNewest` and `DropOldest` the two functions return the same resolution and
the suite asserts it. For `Fail` they do not: `resolve` returns
`Err( RingError::Full )` and `would_resolve` returns `Resolution::Refused`.

`Resolution::Refused` is thus unreachable through `resolve` for every input, and
the agreement test necessarily excludes the one policy where the two halves
disagree. The same real event carries two names in two crates' type systems, and
neither declaration mentions the other.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the precondition that scopes the type --'
command grep -m1 -F '/// Apply `policy` to a publish that found the ring full, recording the outcome' ring_overflow/src/lib.rs
echo '  -- and the one policy the two functions answer differently --'
sed -n '/^    OverflowPolicy::Fail => Err( RingError::Full ),$/p;/^    OverflowPolicy::Fail => Resolution::Refused,$/p' ring_overflow/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| OV13 | `ring_overflow` | n/a — doc gap | `Resolution` has no variant for a publish that found room, because `resolve` is defined only over a ring already known full — which is precisely what makes a resolution a function of the policy alone and `would_resolve` a `const fn` of one argument — yet that reasoning appears nowhere as a decision, only as a one-line precondition on one function's doc comment |
| OV14 | `ring_overflow` | **misleading doc** | `accepted_incoming()` is true for exactly `EvictedOldest`, so the crate's "accepted" means accepted *by evicting something* rather than "the item got in" — a publish that succeeded normally produces no `Resolution` at all and so reads as not accepted — and neither the name nor the doc comment draws that distinction, on a predicate whose one true variant no production build can reach |
| OV15 | `ring_overflow` | n/a — observation | `resolve` returns `Err( RingError::Full )` for `OverflowPolicy::Fail`, so `Resolution::Refused` is unreachable through the crate's main entry point for every input — constructed in fourteen places workspace-wide, thirteen of them doctests or test assertions and the fourteenth `would_resolve`'s own arm — and `resolve_agrees_with_would_resolve` therefore proves agreement on exactly the two policies where the halves do not disagree |
| OV16 | `ring_overflow` | n/a — duplication | One refusal event carries two names in two crates' type systems — `Resolution::Refused` and `RingError::Full` — with neither declaration referencing the other, so a future full-ring error variant in `ring_types` would give `resolve` a choice `would_resolve` has no counterpart for, and the two halves would diverge in the one arm the agreement test already skips |
