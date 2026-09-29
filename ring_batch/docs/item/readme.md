# item

Two of `BatchClaim`'s eight methods carry more documentation than the rest and
both carry something wrong with it. `overlaps` states the reason it exists —
a whole-run disjointness check — and the whole-run check that exists declined to
use it, on a stated technical ground, in a comment 150 lines away. `end()` states
that the value it returns is the one the cursor now holds, which is true for one
producer and measurably false for four.

Between them they also hold the crate's arithmetic. `end()` contains the only
`+` anywhere in the body, and `contains`, `sequences`, and `overlaps` all route
their comparisons through it, as does the contention suite's ordering assertion.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_method_whose_reason_was_declined.md) | The Method Whose Reason Was Declined | `overlaps`: its stated purpose, the test that passed it over, and its load-bearing empty guard |
| [002](002_one_past_the_end.md) | One Past the End | `end()`: the crate's only addition, its four consumers, and the clause that holds only under one producer |

## Documented Reasons Age Worse Than Documented Behaviour

Both findings have the same shape and it is not the shape of a bug. Every
statement in these two doc comments was true when written. `overlaps` really was
added for a whole-run check; `end()` really is the cursor's value on a
single-producer ring, which is what `ring_batch` was first exercised on.

What went stale is the reason, not the behaviour — and a reason has no test.
`a_claim_reports_its_own_extent` pins what `end()` returns and would fail
instantly if the arithmetic changed; nothing pins the sentence about the cell,
and nothing could without a second thread. The parts of a doc comment that
describe the surroundings are exactly the parts the compiler and the suite cannot
reach.

## Coverage Does Not Follow Use

`overlaps` has nine call sites in the test suite, more than any other method, and
zero anywhere outside the crate. Its two tests cover abutting below and above,
one sequence shared at each end, self-overlap, full containment, and three empty
cases — because the `!is_empty()` guard is genuinely subtle: without it, an empty
claim strictly inside another satisfies both halves of the range comparison and
reports an overlap that is not there.

`contains`, three lines above, needs no such guard and gets three tests. The same
edge case, handled two different ways in adjacent methods, both correct, and only
one of the two says why.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two documented reasons --'
command grep -m1 -F '  /// One past the last sequence owned — the value the cell now holds.' ring_batch/src/lib.rs
command grep -m1 -A2 -F '  /// The property a claim protocol must never violate — two overlapping claims' ring_batch/src/lib.rs
echo '  -- the one arithmetic, and everything that routes through it --'
command grep -m1 -F '    Seq( self.start.0 + self.count as u64 )' ring_batch/src/lib.rs
command grep -n '\.end()' ring_batch/src/lib.rs | command grep -v '///'
command grep -n '\.end()' ring_batch/tests/batch_test.rs
echo '  -- call sites per method, in the test suite --'
for m in new start len is_empty end contains sequences overlaps
do
  printf '    %-10s %s\n' "$m" "$( command grep -c "\.$m(" ring_batch/tests/batch_test.rs || true )"
done
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BA26 | `ring_batch` | **misleading doc** | `overlaps` is documented as existing so a whole-run test can assert disjointness; that test exists, names the same property, and states it uses a per-sequence `HashSet` instead because it is stronger |
| BA27 | `ring_batch` | n/a — observation | Nine call sites, more than any other method, all inside `overlaps`' own two tests and none anywhere else in the family; the `!is_empty()` guard those tests cover is load-bearing where `contains`' absence of one is correct |
| BA28 | `ring_batch` | **misleading doc** | `end()`'s "the value the cell now holds" held on all 60,000 claims at one and two threads and failed 2,840 times out of 560,000 at four and above, with the cell up to 596,104 sequences ahead |
| BA29 | `ring_batch` | n/a — observation | `end()`'s `+` is the only arithmetic operator in the crate body, and three of the eight methods plus the ordering assertion route through it |
