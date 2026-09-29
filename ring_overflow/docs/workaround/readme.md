# workaround

External constraints `ring_overflow` absorbs on behalf of its consumers, each with the
cost it imposes and the condition under which it can be deleted.

Two constraints shaped this crate, and both arrived from outside it. An atomic in
`ring_stats` made the recording function impossible to evaluate at compile time,
so the mapping is written twice. A decision to send one policy out through the
error channel put a single outcome into two type systems, so reading it back
requires either a lossy combinator or avoiding the crossing altogether.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_recorder_forecloses_const.md) | The Recorder Forecloses `const` | The atomic two crates away, and the split it forced |
| [002](002_one_outcome_expressed_in_two_type_systems.md) | One Outcome Expressed in Two Type Systems | `is_ok_and` as a bridge, and the consumer's route around it |

## An Atomic Two Crates Away

`resolve` calls `record_drop`, which ends in `fetch_add`. That forecloses `const`
outright, so the mapping cannot be both recorded and compile-time evaluable in one
function. `would_resolve` is the answer, and the hand-maintained duplicate of the
mapping is what it costs.

The chain is sound at every link and no single doc comment spans it: the crate
carries a test whose only job is to keep two copies equal, the copies exist to
preserve `const`-evaluability, and nothing in the workspace evaluates
`would_resolve` at compile time.

## One Outcome, Two Type Systems

`Fail` leaves `resolve` as `Err( RingError::Full )` and `would_resolve` as
`Resolution::Refused`. Reading an outcome across that boundary has exactly two
available shapes, and the crate exercises both.

The suite composes: `outcome.is_ok_and( Resolution::lost_an_item )`, which returns
`false` for two distinct reasons and never constructs `Refused`. The consumer
avoids: it takes `would_resolve`, which has only one type system, and pays by
writing no counter and never constructing `RingError::Full`.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the constraint that forecloses const --'
command grep -m1 -F '    counter.fetch_add( n, Ordering::Relaxed );' ring_stats/src/lib.rs
echo '  -- and the boundary that needs bridging --'
sed -n '/^    OverflowPolicy::Fail => Err( RingError::Full ),$/p;/^    OverflowPolicy::Fail => Resolution::Refused,$/p' ring_overflow/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| OV37 | `ring_overflow` | n/a — doc gap | `would_resolve` exists because `record_drop` bottoms out in `counter.fetch_add( n, Ordering::Relaxed )`, which forecloses `const` on `resolve` no matter how it is written — so the split is a workaround for a constraint imported from `ring_stats`, and neither function's doc comment says so, leaving the obstacle two crates and one indirection away from the code it shapes |
| OV38 | `ring_overflow` | n/a — observation | The `const`-evaluability the split exists to preserve is exercised nowhere — the one production call site is a runtime `match` on a field, and every test call is a runtime assertion or a `HashMap` key — so the crate maintains a duplicate mapping, plus a test defending that duplicate, to keep a property no caller has taken up |
| OV39 | `ring_overflow` | n/a — coverage | `is_ok_and` is the only available bridge across the `Result`/`Resolution` boundary and it returns `false` for two distinct reasons — predicate said no, or the outcome was `Err` and the predicate never ran — so the crate's most valuable test passes for `Fail` by both sides reaching `false` from opposite directions, and recovering the distinction requires abandoning the combinators and matching both levels by hand |
| OV40 | `ring_overflow` | n/a — observation | `ring_core` sidesteps the boundary by taking `would_resolve`, which has only one type system, and returns `Err( record )` rather than a family error — so it hands the caller its item back instead of a diagnosis, writes no counter, and leaves `RingError::Full` with no production construction site via this path, reducing both halves of the pair at once with nothing recording that a trade was made |
