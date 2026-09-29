# item

Six public items in 237 lines: one enum, an associated const and two methods
on it, and two free functions. The instances here take them one at a time —
what each declares, what each body contains, and what the differences between
them show.

The crate is small enough that a `diff` of its two free functions is a complete
account of their relationship, and it turns out to be two hunks: one added
statement and three rewrapped arms, of which two are mechanical and one is a
change of type system.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_two_free_functions_one_statement_apart.md) | The Two Free Functions, One Statement Apart | Both bodies whole, and the two-hunk diff between them |
| [002](002_the_enum_and_its_two_readings.md) | The Enum and Its Two Readings | The type's whole inherent surface and both `match` bodies |

## A Mapping Written Twice by Hand

Nothing derives one function from the other. `resolve` could have been
`would_resolve` plus a record and a wrap, and is not. The reverse delegation is
genuinely closed — `resolve` takes a `&RingStats` and touches an atomic, which
would cost `would_resolve` its `const` — but the forward one has no obstacle and
is simply not taken.

What keeps the copies in step is one test covering two of three arms. The third
arm cannot be covered as a relationship, because one side returns `Ok` and the
other returns `Err`, so it is covered as two independent facts instead.

## Exhaustive Everywhere It Decides

Both free functions use an exhaustive `match` with no `_` arm, so a fourth policy
is a compile error naming both sites. Both predicates are the same shape — an
exhaustive `match` over all three variants — so a fourth resolution is a compile
error at both readings too, rather than silently reading as losing nothing and
accepting nothing.

The crate therefore treats exhaustiveness as load-bearing throughout, and
`Resolution::lost_an_item`'s doc comment states the reason at the site: an
unnamed variant would inherit `Refused`'s profile *"by a predicate that never
mentioned it"*. The predicates were `matches!` until OV22 was applied
([`invariant/001`](../invariant/001_the_mapping_is_total_and_injective.md)); that
is the split this section used to record, and it is closed.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every public item --'
# the last alternative is `pub const [A-Z_]*` and not another `fn` form on
# purpose: `pub const ALL` is a public item too, and the lowercase character
# class every other alternative uses cannot match a screaming-snake-case name
command grep -o 'pub enum [A-Za-z]*\|^impl [A-Za-z]*\|pub fn [a-z_]*\|pub const fn [a-z_]*\|pub const [A-Z_]*' ring_overflow/src/lib.rs
echo '  -- and how each body decides --'
printf '    match policy:   %s\n' "$( command grep -c 'match policy' ring_overflow/src/lib.rs || true )"
printf '    match self:     %s\n' "$( command grep -c '^    match self$' ring_overflow/src/lib.rs || true )"
printf '    matches!( self: %s\n' "$( command grep -c 'matches!( self' ring_overflow/src/lib.rs || true )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| OV25 | `ring_overflow` | n/a — observation | A `diff` of the two free functions' bodies reports exactly two hunks — one added `stats.record_drop( policy, 1 )` and one substitution over all three arms — so "the pure half of `resolve`" holds precisely to the extent that wrapping in `Ok` is not a semantic change: true for two arms, false for the third, which is also the only arm the agreement test skips |
| OV26 | `ring_overflow` | n/a — duplication | The policy-to-outcome mapping is written twice in adjacent functions with no mechanism keeping them equal — `resolve` could delegate to `would_resolve` and wrap, which has no obstacle, while the reverse is genuinely closed by the atomic — and the property that both copies agree is verified for two arms and is not statable for the third |
| OV27 | `ring_overflow` | n/a — observation | The type's whole inherent surface is two `pub const fn` taking `self` by value, with no `Display`, `as_str`, conversion, or constructor — while a string form exists and is exercised as a private `name()` inside a test, where the exhaustive `match` doubles as the crate's variant-count guard, so moving it into the crate would give callers the strings and keep the guard, and nothing records that the test is doing both jobs |
| OV28 | `ring_overflow` | n/a — doc gap | All three variants are now answered explicitly in both predicates — an exhaustive `match` at each reading, the same shape the two free functions already used — so `Refused`'s two `false` answers are stated rather than inherited by omission, and a fourth variant stops compiling at both readings instead of silently taking `Refused`'s profile; the split this finding recorded, and the missing rationale for it, were both closed when OV22 was applied, and `lost_an_item`'s doc comment now carries the reason at the site |
