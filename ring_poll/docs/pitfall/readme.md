# Pitfall Doc Definition

### Scope

- **Purpose**: Traps a caller of this surface can fall into, each with the failure it produces and what actually mitigates it.
- **Responsibility**: Two ways a budget's meaning gets over-read.
- **In Scope**: Non-parking mistaken for bounded latency; a budget mistaken for a batch size.
- **Out of Scope**: `Ok` under drop-newest (→ [`ring_shutdown/docs/pitfall/002`](../../../ring_shutdown/docs/pitfall/002_ok_does_not_mean_kept_under_drop_newest.md)), asserted here but documented there.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Non-Parking Is Not Bounded Latency](001_non_parking_is_not_bounded_latency.md) | A legal, compliant, entirely non-parking way to miss a frame | 🔄 |
| 002 | [A Bigger Budget Is Not a Bigger Batch](002_a_bigger_budget_is_not_a_bigger_batch.md) | The batch helper's extra exit, and the assert that fires the first time saturation happens | 🔄 |

**Both instances are one parameter read too generously.** `Budget` is a single
`usize` with two constructors and no upper bound, and each document takes one of
the two ways that number gets over-read: `001` reads a *cost* into it that is not
there — non-parking says nothing about elapsed time — and `002` reads a *count*
into it that is not there either, because the batch helper's early exit means a
budget of nine can buy exactly one attempt.

They are separate because the mitigations have nothing in common. `001`'s is a
grep and a design choice about what constructors exist; `002`'s is reading a
return value correctly and bounding the iterator instead of the budget. A reader
arriving with one misreading has no reason to hold the other.

The four findings split evenly and land on the same shape twice: in both
documents the *warning itself* was the thing that was wrong. `001` stated its
central cost forty times too small, in the direction that made the trap look
affordable (PL42), and its mitigation is a procedure that has never been run
against a real call site, because nothing depends on this crate yet (PL41).
`002` omitted a whole column of its own correction — records the operation
destroys — when the assertion proving it sat inside a test the document already
cited by name (PL43), and its closing advice pointed a publishing caller at the
consuming side's count bound (PL44).

That is worth naming as a property of the definition rather than of either file:
a pitfall document is read by someone who already suspects their model, so it is
trusted harder than the API doc it supplements, and an error in it survives
longer.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll/docs/pitfall
printf 'instances:                   %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:     %s\n' "$( command grep -hoE '^### PL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:     %s\n' "$( command grep -coE '^\| PL[0-9]+ ' readme.md )"
printf 'each instance has a recipe:  %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'the parameter both are about:%s\n' "$( command grep -oE 'pub struct Budget\( [a-z0-9]+ \)' ../../src/lib.rs )"
printf 'the whole Budget surface:    %s\n' "$( awk '/^impl Budget$/{f=1} f&&/^\}$/{exit} f' ../../src/lib.rs | command grep -oE 'pub const fn [a-z_]+' | sed 's/pub const fn //' | tr '\n' ' ' )"
printf 'any upper bound on it:       %s\n' "$( command grep -cE 'MAX_ATTEMPTS|max_attempts|assert.*< *[0-9]{3}' ../../src/lib.rs || true )"
printf 'crates depending on this one:%s\n' "$( cd ../../.. && for f in ring_*/Cargo.toml; do [ "$f" = ring_poll/Cargo.toml ] && continue; command grep -q 'ring_poll' "$f" && printf ' %s' "${f%/Cargo.toml}"; done | wc -w )"
printf 'batch tests reading it back: %s\n' "$( awk '/^fn /{n=$0} /records\.next\(\)/ && n ~ /^fn push_batch_within_/{c++} END{printf "%d of %d", c, g}' g="$( command grep -c 'fn push_batch_within_' ../../tests/poll_test.rs )" ../../tests/poll_test.rs )"
```

Live output:

```
instances:                   2
finding headings inside:     4
rows in the table below:     4
each instance has a recipe:  2
the parameter both are about:pub struct Budget( usize )
the whole Budget surface:    once new attempts 
any upper bound on it:       0
crates depending on this one:0
batch tests reading it back: 2 of 4
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PL41 | the mitigation is a grep, and it has never been run against a call site | n/a — coverage | The Mitigation section settles on making the budget choice *visible* rather than capped — one default, one explicit constructor, no named magnitudes, and a *who chose it* column reading **the caller** on every budgeted row — and the Failure section turns that into a procedure: grep `Budget::new`, treat every hit as a latency decision someone made; run over the family that grep returns seventeen hits in two files, both inside this crate, four in `src` doctests and thirteen in the suite, and no crate depends on `ring_poll` at all, so the column attributes to a caller that does not exist and the numbers chosen so far were chosen to make assertions convenient. |
| PL42 | the warning understated the danger it warns about, by a factor of forty | **misleading doc** | The Trap table's load-bearing row — a million-attempt budget, fully compliant with the non-parking rule, still missing the frame — read *~10 ms of pure spinning* until it was checked against the crate's own measurement: P3 records 20 000 attempts at 0.008 s, so an attempt costs 0.4 µs and a million cost ~400 ms, twenty-five frames rather than two-thirds of one; the likely origin is a first-principles estimate of a `spin_loop` hint at ~10 ns that omits the `try_push` the attempt actually performs, and the corrected figure is now derived in the Regenerate block so the two cannot diverge again. |
| PL43 | the assertion proving the missing column was inside a test the document already cited | **misleading doc** | The Trap table accounted for attempts spent and records published and said nothing about records *destroyed*, so a caller seeing *4 published* of twelve concluded eight remained when six were gone; the number was not in an uncited test but three lines below an assertion the document did read, inside `push_batch_within_spends_a_second_attempt_after_a_productive_first`, whose message spells it out — and the framing *"a pair, one asserts each row"* is what stopped the reading, since a test assigned a row is a test whose job is known; the zero-moved row's figure is asserted nowhere at all, because that test passes an inline `&mut ( 0..4 )` and keeps no binding to read back. |
| PL44 | the mitigation named a count bound that only exists on the other side | **wrong doc** | The closing advice used to end *"and use `drain_up_to`'s `max` when a count is what needs bounding"* — true of `drain_up_to`, which does take `max : usize` and is the only count bound on this surface, and unreachable for the caller being addressed, since `push_batch_within( producer, records, budget )` has no count parameter and the two operations are not alternatives: one consumes and one publishes; the answer that does work, `records.by_ref().take( n )`, appears zero times in this crate's source or tests, so the pitfall pointed a publishing caller at a consuming tool and the one-line fix is unwritten anywhere. |
