# api

The published contract. `ring_index` exports three free functions and nothing
else — no type, no trait, no constant, no module. That is small enough that the
whole surface fits in one census, which is what the first instance is; the
second reads the same three signatures for what they decline rather than what
they offer.

Two things are worth saying up front because they shape both instances. The
first is that two of the three functions have no caller anywhere in the 33-crate
family — `of` is imported five times, `aliases` and `run` zero. The second is
that none of the three is a `const fn`, in a tier where `ring_types` is twelve
of twelve, and at least two of them could be.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_three_functions_three_must_use_one_reached.md) | Three Functions, Three `must_use`, One Reached | The surface as an inventory; the uniform attribute; the reach census |
| [002](002_the_three_signatures_and_the_const_they_are_not.md) | The Three Signatures and the `const` They Are Not | What the signatures give up: `const`, and an inverse |

## A Surface With No Types Is a Surface With No Versioning Problem

Every argument and every return value belongs to `ring_types`. `ring_index`
introduces no name of its own into any caller's namespace beyond the three
function names, which means a caller cannot hold a `ring_index` value, cannot
store one in a struct, and cannot name one in a signature.

That is what makes the crate substitutable. A caller that writes
`of( seq, capacity )` and a caller that writes `( seq.0 as usize ) & capacity.mask()`
have identical types at every boundary — which is exactly why `ring_mpsc` could
write the second one without anything noticing
([`integration/001`](../integration/001_two_dependents_and_a_third_that_did_it_again.md)
IX12). A one-owner boundary enforced only by convention is enforced by nothing.

## The Attribute That Is Uniform and the Keyword That Is Absent

All three functions carry `#[ must_use ]`; none carries `const`. The first is
uniform because the crate has no state, so no function has a result that is
incidental to an effect. The second is uniform in the other direction and has no
such structural reason: `of`'s body compiles as `const` unchanged, and
`aliases`' would too if it called the `const` accessor `ring_types` already
publishes for that purpose.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -n '^pub \|^#\[ must_use' ring_index/src/lib.rs
command grep -rn 'use ring_index\|ring_index::' ring_*/src/*.rs ring_*/tests/*.rs \
  | command grep -v '^ring_index/'
for c in ring_types ring_config ring_slot ring_align ring_overflow ring_stats ring_seqno ring_index; do
  t=$( command grep -h '^\s*pub \(const \)\?fn' ring/$c/src/*.rs | wc -l )
  k=$( command grep -h '^\s*pub const fn' ring/$c/src/*.rs | wc -l )
  printf '  %-14s %2s of %2s public fns are const\n' "$c" "$k" "$t"
done
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| IX5 | `ring_index` | n/a — observation | `#[ must_use ]` is 3 of 3, because a stateless crate has no function whose value is incidental |
| IX6 | `ring_index` | n/a — coverage | `aliases` and `run` have no caller in any of the 33 crates; the crate is cited by name five times more widely than it is called |
| IX7 | `ring_index` | n/a — observation | `of` compiles as a `const fn` unchanged and const-evaluates; it is not declared one, in a tier where `ring_types` is 12 of 12 |
| IX8 | `ring_index` | n/a — observation | `aliases` is kept from `const` only by comparing `SlotIndex` rather than calling `SlotIndex::get`, which `ring_types` publishes as `pub const fn` |
| IX9 | `ring_index` | n/a — doc gap | Nothing takes a `SlotIndex` — the fold has no inverse and cannot have one, and the crate never says so |
