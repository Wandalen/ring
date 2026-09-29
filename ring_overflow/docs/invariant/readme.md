# invariant

Two properties hold this crate together. The structural one is that the mapping
from policy to resolution is total and injective — a bijection between two
three-variant enums, enforced by two exhaustive `match` blocks and asserted by two
tests. The safety one is that no outcome may keep the incoming item and destroy
unread data without reporting the loss.

Both are correctly stated and correctly implemented. What the instances here record
is where the enforcement stops: the readings that classify a new variant are
assigned by pattern-matching omission rather than by decision, and the safety
assertion is reached by exactly one arm with nothing checking that it was reached.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_mapping_is_total_and_injective.md) | The Mapping Is Total and Injective | The bijection, both variant-count guards, and the unguarded readings |
| [002](002_no_resolution_overwrites_unread_data_silently.md) | No Resolution Overwrites Unread Data Silently | The safety implication and its single witness |

## A Bijection Named in One Direction

Totality comes from the compiler: `would_resolve`'s `match` has no `_` arm.
Injectivity comes from `distinct_policies_give_distinct_resolutions`, which
iterates `OverflowPolicy::ALL` and asserts no resolution repeats. Together they
make the mapping a bijection between the two enums.

The crate exposes one direction of it. The injectivity test's own comment says
"the resolution alone identifies what happened", which is the statement that the
inverse exists — and there is no function performing it.

## Guarded Counts, Unguarded Readings

Both variant counts are pinned, by different mechanisms: `OverflowPolicy` by a
published `ALL` with a length assertion, `Resolution` by an exhaustive `match`
inside a test helper that stops compiling if a fourth variant appears.

Neither *guarded* the two predicates. They were `matches!` over positive
patterns, so a new variant fell through to `false` on both — silently acquiring
`Refused`'s profile, with the compile error pointing at the helper that does not
matter and saying nothing about the two readings that do. Both are now
exhaustive `match` bodies naming all three variants, so a fourth is a compile
error at the readings themselves (→ OV22).

## An Implication With One Witness

The safety criterion is `accepted_incoming() ⟹ lost_an_item()`, asserted inside an
`if`. Only `EvictedOldest` satisfies the antecedent, produced by one arm, reached
by one policy — and that policy is rejected at construction in every production
build. Nothing asserts the assertion ran, so a vacuous pass and a real pass look
identical.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two mappings the invariant constrains --'
awk '/^  stats\.record_drop\( policy, 1 \);$/{ n1 = NR } n1 && NR >= n1 + 1 && NR <= n1 + 6 { print } /^pub const fn would_resolve\( policy : OverflowPolicy \) -> Resolution$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 7 { print }' ring_overflow/src/lib.rs
echo '  -- and the readings a new variant must now name --'
sed -n '/^  pub const fn lost_an_item( self ) -> bool$/,/^  }$/p;/^  pub const fn accepted_incoming( self ) -> bool$/,/^  }$/p' ring_overflow/src/lib.rs | command grep -E '=> true,|=> false,'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| OV21 | `ring_overflow` | n/a — observation | Totality comes from an exhaustive `match` and injectivity from `distinct_policies_give_distinct_resolutions`, so the suite establishes a bijection between two three-variant enums — and the crate exposes one direction of it, though the injectivity test's own comment ("the resolution alone identifies what happened") is precisely the claim that the inverse is well-defined |
| OV22 | `ring_overflow` | **latent hazard** | Both enums' variant counts are guarded — `OverflowPolicy` by a published `ALL` plus a length assertion, `Resolution` by an exhaustive `match` in a test helper that stops compiling on a fourth variant — but `lost_an_item` and `accepted_incoming` were `matches!` over positive patterns, so once that helper was updated a new variant silently read `false, false`, exactly `Refused`'s profile, and an `Overwrite` variant would have been classified as losing nothing and accepting nothing; both are now exhaustive `match` expressions that stop compiling instead |
| OV23 | `ring_overflow` | n/a — doc gap | The safety criterion is `accepted_incoming() ⟹ lost_an_item()` — eviction is permitted, and only *unreported* destruction of unread data is forbidden — which is the crate's single safety property, stated in one test's doc comment and in no module doc, no declaration, and neither predicate's documentation |
| OV24 | `ring_overflow` | **latent hazard** | The safety assertion sits inside `if resolution.accepted_incoming()`, satisfied only by `EvictedOldest`, produced by one arm and reached by one policy that no production build accepts — and nothing asserted the body ever ran, so removing `DropOldest` from `OverflowPolicy::ALL` would have made the crate's central safety test vacuous while leaving it green; the loop now counts its satisfactions and asserts the count, proven by running that exact removal |
