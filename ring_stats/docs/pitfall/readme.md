# pitfall

Two readings in this crate are the ones a monitor actually displays — `in_flight`, the
leak detector, and `dropped_total`, the loss figure — and both are defective in the
condition they exist for. Every other reader is a single relaxed load and is exactly
right. The two that get looked at are compositions, and they fail under traffic, which
is the only state in which anyone looks.

Neither failure is loud. `in_flight` fails to the value that means "nothing is wrong",
and `dropped_total` fails to a number that is plausible, monotone, and semantically
inverted for the safest overflow policy. In both cases the reading gives no signal that
it is the wrong reading.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_leak_in_flight_cannot_see.md) | The Leak `in_flight` Cannot See | The detector that reports health, and why swapping the two loads makes it worse |
| [002](002_the_total_that_counts_a_refusal_as_a_loss.md) | The Total That Counts a Refusal as a Loss | The safest policy producing the largest loss figure, and the breakdown that tears |

## A Detector Whose Failure Mode Is Health

`in_flight` loads `claimed`, then `published`, then subtracts and floors at zero. Both
counters climb, so every publish landing between the two loads is taken off a `claimed`
that predates it and the answer comes out too small; `saturating_sub` stops it at zero,
which is what a healthy ring reads.

Against a fixed eight-slot leak with one matched producer beside it, two million
readings understate the leak around one percent of the time and report exactly zero
seven to sixteen times — the crate's own documented signal for *no leak at all*, while
eight slots are permanently outstanding.

The obvious repair fails, and it fails in a way worth recording. Reading `published`
first eliminates every understatement and every zero — and on a ring with **no** leak at
all it reports more than one outstanding slot around three percent of the time, ranging
as high as 832 where at most one ever is. No ordering of two independent loads is
correct. Only a consistent read of both is, and the crate has no method that does one.

## A Loss Figure That Punishes the Safe Choice

`dropped_total` is documented "Items lost across every policy" and folds all three,
including `Fail` — where the ring refuses the push and hands the item back, losing
nothing. So a `Fail` ring under pressure reports the same rising loss figure as a
`DropOldest` ring under the same pressure, and the two mean opposite things. The
operator who chose the policy that loses nothing gets the worst-looking dashboard.

Reading the per-policy breakdown instead does not rescue it. Three `dropped()` calls
are three separate loads at three moments: driven from one writer that keeps them
within one of each other, three to five percent of breakdowns show a spread the ring
never held, the widest running to 3,325.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two readings a monitor displays --'
sed -n '/^  \/\/\/ Items lost across every policy\.$/p;/^  \/\/\/ Slots claimed but not yet published — a nonzero reading here at rest means$/,/^  \/\/\/ a producer took a slot and abandoned it, which is a leak of ring capacity\.$/p' ring_stats/src/lib.rs
echo '  -- and how each is computed --'
sed -n '/^    OverflowPolicy::ALL\.iter()\.map( | p | self\.dropped( \*p ) )\.sum()$/p;/^    self\.claimed()\.saturating_sub( self\.published() )$/p' ring_stats/src/lib.rs
echo '  -- the discriminator ring_types already offers --'
command grep -n 'pub const fn drops_silently' ring_types/src/policy.rs
echo '  -- and what the suite spawns while reading either --'
printf '    threads across all in_flight and drop-total tests: %s\n' \
  "$( command grep -m1 -B1 -A124 -F '  let summed : u64 = OverflowPolicy::ALL.iter().map( | p | stats.dropped( *p ) ).sum();' ring_stats/tests/stats_test.rs | command grep -c 'spawn\|thread::scope' || true )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| ST41 | `ring_stats` | **latent hazard** | `in_flight` loads `claimed` before `published` so its error is always downward, and `saturating_sub` floors that error at zero — the exact reading a healthy ring gives — so against a permanent eight-slot leak, two million readings understate it around one percent of the time and report no leak at all seven to sixteen times, with nothing in the value distinguishing the miss from a correct answer |
| ST42 | `ring_stats` | **measured cost** | Swapping the two loads removes every understatement and every zero reading against a real leak, and introduces overstatement on a ring with **no** leak — more than one outstanding slot reported around three percent of the time, as high as 832 where at most one ever is — so neither ordering of two independent loads is correct and the cheap repair makes the common case worse |
| ST43 | `ring_stats` | **misleading doc** | `dropped_total` is documented "Items lost across every policy" and folds `Fail`, where the ring refuses the push and hands the item back losing nothing, so a `Fail` ring and a `DropOldest` ring under identical pressure report identical numbers meaning opposite things — and the operator who chose the policy that loses nothing is the one whose loss figure climbs fastest |
| ST44 | `ring_stats` | **latent hazard** | The per-policy breakdown a reader would check instead is three separate loads at three moments: driven from one writer keeping the counters within one of each other, three to five percent of two million breakdowns show a spread the ring never held, the widest running to 3,325 — so the total is semantically wrong and the decomposition that would correct it is numerically inconsistent, both failing precisely under the traffic that prompts the question |
