# workaround

Two language constraints shape this crate, and it works around both correctly. `u64`
has no negative values, so `in_flight` floors its subtraction rather than wrapping to
eighteen quintillion. Rust has no field iteration, so `reset` walks an array of
references to all seven counters. Neither workaround has a better version available;
both are what a careful author would write.

What both share is that the guard was added and the check that the guard still does its
job was not. The floor absorbs a condition far more common than the one it is
documented against, and the array is one of four hand-maintained lists of seven that
nothing relates to each other.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_floor_that_absorbs_more_than_it_was_built_for.md) | A Floor That Absorbs More Than It Was Built For | `saturating_sub`, the caller-bug rationale, and what it actually catches |
| [002](002_seven_counters_enumerated_four_times_by_hand.md) | Seven Counters, Enumerated Four Times by Hand | `reset`'s array, the three lists beside it, and the family's own check |

## A Guard Documented Against the Wrong Condition

`saturating_sub` is the crate's only saturating, checked or wrapping operation. The
suite explains it as protection against a caller bug — publishing more than was claimed
— and tests exactly that, twice, on one thread.

On a ring where no such bug exists, where every producer claims before it publishes
always, the floor fires on one to two percent of readings, by as much as 36,141. The
cause is the read order: `claimed` is loaded first, so a producer running between the
two loads can publish past a `claimed` already in hand. The guard is load-bearing for
ordinary operation, not a rare backstop.

And it floors at zero, which is the reading a healthy ring gives — so three distinct
conditions arrive at the caller as the same value. `checked_sub` separates the
impossible one from the other two for the cost of one word, and is not discussed
anywhere.

## Four Lists the Compiler Only Half Checks

`reset`'s array-of-references loop is the idiomatic answer to a language that cannot
iterate fields. Its consequence is four hand-written enumerations of the same seven
counters — the struct, `new`, the array, and the reset test — of which the compiler
enforces two. A counter left out of the array survives every reset silently and stays
plausible and monotone.

Three of the seven are exempt: `record_drop` and `dropped` dispatch through an
exhaustive `match` on a three-variant enum, so a fourth policy would produce two
compile errors naming both sites. Within one struct, the drop counters are protected
and the other four are not.

`ring_types` solved this for its own hand-maintained sets — `ALL` published as a
constant with `assert_eq!( OverflowPolicy::ALL.len(), 3 )` as both doctest and test,
and the reasoning written up in its corpus. `ring_stats` applies the convention to none
of its four lists.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the crate every saturating, checked or wrapping operation --'
command grep -n 'saturating_\|checked_\|wrapping_' ring_stats/src/lib.rs
echo '  -- the four hand-maintained lists of seven --'
printf '    struct fields                  %s\n' "$( command grep -m1 -B1 -A8 -F 'pub struct RingStats' ring_stats/src/lib.rs | command grep -c 'AtomicU64' || true )"
printf '    new() initialisers             %s\n' "$( command grep -m1 -A10 -F '    Self' ring_stats/src/lib.rs | command grep -c 'AtomicU64::new' || true )"
printf "    reset()'s array                %s\n" "$( command grep -m1 -A4 -F '    [' ring_stats/src/lib.rs | command grep -o '&self\.[a-z_]*' | wc -l )"
printf '    assertions in the reset test   %s\n' "$( command grep -m1 -A25 -F 'fn reset_returns_every_counter_to_the_fresh_state()' ring_stats/tests/stats_test.rs | command grep -c 'assert_eq!( stats\.' || true )"
echo '  -- and the length check the family applies to its own hand-maintained sets --'
command grep -n 'ALL.len()' ring_types/tests/types_test.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| ST49 | `ring_stats` | **measured cost** | `saturating_sub` is documented in the suite as protection against a caller bug, and on a ring where every producer claims before it publishes — no caller bug anywhere — it fires on one to two percent of two million readings at each of three producer counts, underflowing by as much as 36,141, because `claimed` is loaded before `published` and an ordinary correct producer can publish past the value already in hand |
| ST50 | `ring_stats` | n/a — doc gap | The floor maps three distinct conditions onto `0` — a healthy ring, a reading taken across a seam, and a genuine `published > claimed` — where `checked_sub` returning `Option< u64 >` separates the third for one word at the definition, an alternative the suite's own rationale never weighed because it reasons only about the wrap that `saturating_sub` already beats; `StatsCounts::checked_in_flight` now offers that word on the snapshot, leaving `in_flight` itself still mapping all three onto zero |
| ST51 | `ring_stats` | **latent hazard** | `reset` walks an array of references because Rust cannot iterate fields, which is correct — and left the seven counters written out four times by hand, of which the compiler enforced two, so an eighth counter omitted from the array survived every reset silently, read plausibly, stayed monotone, and turned no test red; the array now lives in `RingStats::counters` behind a declared length of `RingStats::COUNTERS` with a `const` size assertion beside it, so the same omission is now two compile errors rather than a green suite |
| ST52 | `ring_stats` | n/a — coverage | `ring_types` publishes its hand-maintained sets as `ALL` constants and asserts their length in both a doctest and a test, documenting the reasoning in its own corpus — and `ring_stats`, with four hand-maintained sets of seven, applied the convention to none; `RingStats::COUNTERS`, the size assertion beside it and the fixed-length array `counters` returns now convert a silently-missed counter into two compile errors rather than a test that stays green |
