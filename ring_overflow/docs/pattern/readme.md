# pattern

Two patterns, both stated in the source and both slightly misdescribed by it. The
pure/effectful pair is named after purity and justified by purpose, and the two do
not select the same half for the same caller. The named-outcome type is argued for
against a boolean, and the crate has a stronger argument available that it does not
make.

Neither misdescription causes a defect. What they cost is that the crate's one
consumer, which is using both patterns correctly, reads on the documentation's own
terms as if it were not.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_pure_effectful_pair.md) | The Pure/Effectful Pair | The `would_` convention, its frequency, and what really divides the halves |
| [002](002_a_named_outcome_instead_of_a_boolean.md) | A Named Outcome Instead of a Boolean | The stated argument, the two booleans beside it, and the consumer's reduction |

## A Convention With a Sample Size of One

`would_resolve` is the only function anywhere under `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/` whose name begins
`would_`, and the census finds no `peek_`, `try_`, or `dry_` pairing either. So the
family's first instance of "decide without recording" was solved here, well, and
recorded only as one function's name.

The division the doc comment draws is preview-versus-perform: `would_resolve` is
"for a factory validating a configuration, a test tabulating the mapping — rather
than handling a real full-ring event." The division the code uses is
records-versus-does-not. `ring_core` handles real full-ring events and takes the
pure half, because its manifest does not declare `ring_stats` and it has no
`&RingStats` to pass.

## A Type Argued for on the Weaker Ground

The module comment says `Resolution` "makes the alternative outcomes explicit
rather than leaving them to a boolean" — and the crate then ships two booleans as
the way to read it. The two are finer than the one that was rejected and jointly
lose nothing, which is the better argument and the one not made.

The consumer's use is measurable rather than hypothetical, since there is exactly
one: three named outcomes go in, one variant is bound, two share a second arm,
and one bit comes out. The pattern's payoff here is in the writing — a fourth
policy breaks both `match` sites — not in the reading.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every would_* under the crate tree --'
command grep -rn 'fn would_' --include=*.rs */src/ 
echo '  -- and the argument the module comment makes --'
command grep -m1 -A1 -F '//! kept the item. [`Resolution`] below is the type that makes the alternative' ring_overflow/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| OV41 | `ring_overflow` | n/a — doc gap | `would_resolve` is the only function anywhere under `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/` whose name begins `would_`, and of the three prefixes a second crate might reach for instead — `peek_`, `dry_`, `preview_` — the census finds zero of each, while the fourth and most likely candidate is worse than absent: `try_` is already spoken for 72 times over, meaning *fallible attempt* rather than *hypothetical result*; so a legible and correct naming convention for "decide without recording" exists with a sample size of one, is recorded only as one function's name, and the next crate facing the same tension has no precedent to follow and one actively misleading prefix to reach for first |
| OV42 | `ring_overflow` | **misleading doc** | `would_resolve` is documented as being "for a factory validating a configuration, a test tabulating the mapping — rather than handling a real full-ring event", framing the pair as preview-versus-perform, while the division the code uses is records-versus-does-not — and `ring_core`, whose manifest declares no `ring_stats` and which therefore has no `&RingStats` to pass, is handling exactly a real full-ring event, so the one correct production use reads on the doc's own terms as a mistake |
| OV43 | `ring_overflow` | n/a — observation | The module comment argues `Resolution` "makes the alternative outcomes explicit rather than leaving them to a boolean" and the crate ships two `-> bool` readings on that type — finer than the rejected one and jointly injective, so nothing is lost — meaning the crate has the stronger claim available in its own source (three variants is the smallest encoding of two questions with one answer forbidden) and states the weaker one |
| OV44 | `ring_overflow` | n/a — observation | The sole consumer binds one variant by name, groups two in a second arm, and returns `Result< (), T >` — three named outcomes in, one bit out — so with exactly one consumer the pattern's payoff is measurable rather than hypothetical, and it lands in the writing (a fourth policy breaks both `match` sites) rather than in the reading, which is a real and smaller benefit than "makes the alternative outcomes explicit" suggests |
