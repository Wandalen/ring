# decisions

`ring_batch` makes two decisions worth the name, and they are opposites in
every respect that matters. One is argued at length in a nine-line doc comment
and has no test that could contradict it. The other is argued in three lines and
is pinned by a test that asserts the property rather than the variant — the only
test in the crate that does.

Both decisions are about what a caller is allowed to be told. The ordering
decision withholds a choice, on the grounds that a parameter with exactly one
correct answer is not a parameter. The error decision hands one over, on the
grounds that a caller cannot write a retry loop without knowing which failure
will clear. They point in opposite directions and both are right; what separates
them is that only one of the two can be checked.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_ordering_is_the_callers_except_where_it_is_not.md) | Ordering Is the Caller's, Except Where It Isn't | The `order` parameter's scope, the two hard-coded `Acquire` literals, and the missing `loom` seam |
| [002](002_two_errors_not_one.md) | Two Errors, Not One | `BatchTooLarge` against `Full`, the classification a caller branches on, and what `Full` does not carry |

## The Argued One and the Tested One

Nine lines of prose defend two `Ordering::Acquire` literals — the longest
continuous justification anywhere in the crate, longer than any function body it
governs. Three lines defend the error split. The ratio runs backwards from the
evidence: the nine-line argument survives a patched build that downgrades both
literals to `Relaxed` and runs all 31 tests green, while the three-line argument
is pinned by an assertion on `is_configuration()` and `!is_transient()` that
would fail the moment the classification moved.

That is not an accident of effort. An ordering claim needs an interleaving model
to be falsified, and this crate is not on the family's `loom` seam; an error
classification needs one call and two predicates. The crate argues hardest
exactly where argument is all it has.

## Both Decisions Have a Neighbour That Went Further

Neither decision ends inside this crate. `ring_cursor` restates the ordering
split for `CursorPair` almost word for word, citing this function as precedent,
across a boundary with no dependency edge — so the reasoning travelled while the
code did not. `ring_claim` took the error decision further in the other
direction, retiring `BatchTooLarge` for a new entry point and answering "how much
could I have had" as a request rather than as a payload the error refuses to
carry.

In both cases the neighbour is the one with reach — `ring_cursor` at Tier 3,
`ring_claim` at Tier 5 with both ring implementations behind it — and in neither
case does anything in `ring_batch` record that the conversation happened.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- decision one: nine lines, two literals, no seam --'
command grep -nE 'Ordering::[A-Za-z]+' ring_batch/src/lib.rs | command grep -v '///'
echo "  loom in manifest: $( command grep -c loom ring_batch/Cargo.toml || true )"
echo '  -- decision two: three lines, two variants, one property test --'
command grep -nE 'RingError::(BatchTooLarge|Full)' ring_batch/src/lib.rs | command grep -v '///'
command grep -n 'is_configuration\|is_transient' ring_batch/tests/batch_test.rs
echo '  -- what the neighbours did with each --'
command grep -n 'claim_gated' ring_cursor/src/lib.rs
command grep -n 'pub fn claim_up_to' ring_claim/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BA13 | `ring_batch` | n/a — observation | Nine lines justify the crate's only two `Ordering` literals; the decision is about the parameter's *scope*, and the principle travelled to `ring_cursor` across a boundary with no dependency edge |
| BA14 | `ring_batch` | n/a — coverage | The crate carries no `loom` in its manifest while five others do, so its ordering argument has no falsifier — a patched copy with both gating loads `Relaxed` passes all 21 integration tests and 10 doctests |
| BA15 | `ring_batch` | n/a — observation | The error split's test asserts `is_configuration()` and `!is_transient()` rather than the variant, pinning the property the split exists for; it is the crate's only test of that shape |
| BA16 | `ring_claim` | n/a — observation | `Full` is a unit variant carrying no free count and no cursor position, so a caller cannot trim its request; `ring_claim` answered that with `claim_up_to` and `ring_batch` has no counterpart |
