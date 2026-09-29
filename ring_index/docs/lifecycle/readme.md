# lifecycle

Two different lifecycles meet in this crate and neither belongs to it. The first
is the slot reuse cycle — claim, address, occupy, publish, consume, reuse — which
`ring_index` sets the period of and takes no part in. The second is the crate's
own construction and teardown story, which does not exist: no `impl` block, no
constructor, no `Drop`.

The definition records both because the absence is load-bearing. A crate with no
lifecycle can be called before anything is built and after everything is gone,
which is why its central invariant is established by two nested loops instead of
by a producer, a consumer, and a `loom` model.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_lap_is_the_only_cycle.md) | The Lap Is the Only Cycle | The six-phase reuse cycle, and the gate that makes exactly one lap reachable |
| [002](002_no_initialization_and_no_teardown.md) | No Initialization and No Teardown | Zero `impl` blocks, and a ten-test suite that never builds a ring |

## The Period, Not the Phases

`ring_index` appears twice in the reuse cycle — at *address* and at *reuse* — and
does the identical thing both times. The reuse phase is not a second behaviour;
it is `of` called again on a sequence one lap larger, returning the same answer.

That is the crate's whole relationship to time: it fixes the cycle's period at
`capacity` and observes none of the cycle's phases. Nothing else in the family
has that shape — every other crate in the table acts within one turn.

## One Lap Reachable, Two Laps Measured

`ring_seqno::may_claim` is a strict `<` against capacity, so a producer is refused
exactly when it would reach one full lap ahead. The live sequence window is
therefore never wider than `capacity`, and two-lap aliasing is unreachable while
the gate holds.

`ring_seqno::laps_between`, the function immediately above it in the same file, has
a doctest asserting a lap count of `2`. The family ships a measurement of a state its own
gate prevents — and this crate's `of` documents the one-lap case while computing
the general one, which is the same disagreement seen from the other side.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the gate, and the sibling that measures past it --'
awk '/^\/\/\/ assert_eq!\( laps_between\( Seq\( 0 \), Seq\( 17 \), cap \), 2 \);$/{ n1 = NR } n1 && NR >= n1 + 2 && NR <= n1 + 6 { print } /^\/\/\/ assert!\( may_claim\( Seq\( 4 \), Seq\( 1 \), cap \) \);  \/\/ consumer moved on$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 6 { print }' ring_seqno/src/lib.rs
echo "  impl blocks / new / Default / Drop in ring_index : $( command grep -cE '^\s*impl |fn new|Default|Drop' ring_index/src/lib.rs )"
echo "  test fns / rings built                           : $( command grep -c '^#\[ test \]' ring_index/tests/index_test.rs ) / $( command grep -cE 'Buffer|Ring::|thread' ring_index/tests/index_test.rs )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| IX53 | `ring_index` | n/a — observation | The crate sets the reuse cycle's period at `capacity` and participates in none of its six phases; the reuse phase is `of` called again, not a second behaviour |
| IX54 | `ring_seqno` | n/a — observation | `may_claim`'s strict `<` makes one lap the entire reachable aliasing state, so `of`'s narrow comment is exactly as wide as the gate — while `laps_between`'s doctest asserts a two-lap distance the gate prevents |
| IX55 | `ring_index` | n/a — observation | Zero `impl` blocks, constructors, or `Drop` impls; `of` is callable before any ring exists and after every ring is dropped, and nothing here can leak |
| IX56 | `ring_index` | n/a — coverage | Ten tests, two imports, no dev-dependencies, zero rings — the suite proves the arithmetic and proves nothing about whether the arithmetic is reached |
