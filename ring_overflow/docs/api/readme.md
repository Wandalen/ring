# api

One public type, an associated const, and four public functions, in 237 lines.
Three of the four functions are `const` and carry `#[ must_use ]`; the fourth
is neither, and both absences are correct rather than inconsistent — it touches
an atomic, and it returns a `Result`, which is already `#[ must_use ]` in
`core`.

The surface is small enough to read at once, which makes what is missing from it
easy to see: no inverse from a resolution back to the policy that produced it,
and no caller anywhere in the workspace for either of the two readings the type
ships.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_four_declarations_three_of_them_const.md) | Four Declarations, Three of Them `const` | Every attribute on the surface, and why the one unmarked function is unmarked |
| [002](002_two_predicates_and_no_caller.md) | Two Predicates, and No Caller Outside the Crate | Eighteen calls, all in-crate, seventeen of them on a literal |

## Every Mark Load-Bearing, No Mark Explained

`lost_an_item`, `accepted_incoming` and `would_resolve` are `pub const fn` with
`#[ must_use ]`. `resolve` is a plain `pub fn` with no attribute. The difference
is one statement — `stats.record_drop( policy, 1 )` — which is an atomic
`fetch_add` and therefore not const-callable, and a return type that carries the
mark already.

Both facts are true and neither is written at the declaration. This is the
inverse of the family's usual pattern, where the attribute is applied by position
and the consequence is not checked; here the consequence was checked and the
reasoning is not recorded.

## Readings Nothing Reads

Eighteen calls of the two predicates exist and all eighteen are inside this
crate — seven doctests, eleven test assertions. Seventeen name a variant
literally, so they ask a question the compiler could answer. The one exception,
`policy_self_description_agrees_with_the_handler`, is the file's most valuable
test: it pins `ring_overflow`'s handler against `ring_types`' own
`reports_failure()` and `drops_silently()`, which is the drift two independently
editable crates would otherwise be free to develop.

Even that one reaches two of the three variants — `Fail` produces an `Err`, so
`is_ok_and` short-circuits and `Resolution::Refused` is never constructed.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the whole surface, with attributes --'
# the third alternative is `pub const ` and not `pub const fn ` on purpose:
# `pub const ALL` is part of the surface too, and the narrower form cannot see
# it — the `-B1` line above it is a doc-comment fence, not an attribute
command grep -n -B1 'pub enum \|pub fn \|pub const ' ring_overflow/src/lib.rs | command grep -v '^--$'
echo '  -- and every predicate call outside this crate --'
command grep -rl 'lost_an_item\|accepted_incoming' --include=*.rs . | command grep -v '^ring_overflow/' | wc -l
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| OV5 | `ring_overflow` | n/a — doc gap | Three of four functions are `pub const fn` with `#[ must_use ]` and `resolve` is neither, for two correct reasons the declaration does not state — `stats.record_drop` is an atomic `fetch_add` and so forecloses `const`, and `Result` already carries `#[ must_use ]` in `core` — so an attribute set in which every mark is load-bearing and every absence is right reads at the declaration as if it were inconsistent |
| OV6 | `ring_overflow` | n/a — observation | Every entry point takes a policy or a resolution and there is no inverse from `Resolution` back to `OverflowPolicy`, though `distinct_policies_give_distinct_resolutions` establishes exactly the injectivity such a function would rest on — so a caller holding a resolution cannot name the configuration that produced it, and the one consumer does not need to because it kept the policy |
| OV7 | `ring_overflow` | n/a — coverage | `lost_an_item` and `accepted_incoming` are called eighteen times across the workspace, all eighteen inside this crate — seven doctests, eleven test assertions — and seventeen of them name a variant literally, so the partition the two predicates define has never had to hold for a value whose variant was not already known at compile time |
| OV8 | `ring_overflow` | n/a — observation | The one test that reaches a predicate with a computed resolution pins this crate's handler against `ring_types`' own `reports_failure()` and `drops_silently()`, which is the drift two independently editable crates are otherwise free to develop — and it passes without constructing `Resolution::Refused`, because `resolve( Fail )` returns `Err` and `is_ok_and` short-circuits to the `false` that `drops_silently()` also returns |
