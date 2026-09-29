# invariant

Two properties hold across `RingStats`, and they sit at different levels. Each
counter individually is monotone — no `fetch_sub` exists anywhere in the crate and
the only `store` is inside `reset`. Between counters, `claimed` is assumed never to
trail `published`, because `in_flight` subtracts one from the other.

The difference between the two is enforcement. The first is guaranteed by
construction and needs nothing from the caller. The second is a contract on whoever
records, is named in a test comment as "a caller bug" when broken, and is checked by
nothing — the crate contains zero `debug_assert`. The two instances here follow one
each, and both end at the same place: the reading `in_flight` gives when the property
does not hold is the reading it gives when everything is fine.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_monotone_per_counter_and_only_per_counter.md) | Monotone Per Counter, and Only Per Counter | The property `Relaxed` supports, `reset` as its one exception, and the derived reading that lacks it |
| [002](002_claimed_never_trails_published.md) | `claimed` Never Trails `published` — Assumed, Named, Enforced Nowhere | The cross-counter contract, who owns it, and the three routes to a zero reading |

## The Strongest Statement, Tested on One Counter

Monotonicity is exactly as strong as the ordering decision permits, and the suite
says so: "the strongest statement `Relaxed` supports per counter". Its test was also,
for a long time, the only one in the suite that read a counter while another thread
wrote — and it reads `published`, alone, which is the one read in the crate with no
composition and no second counter in it. Two later tests read under contention through
a composition and across a `reset`; this one is still the narrowest of the three.

The property does extend to `dropped_total`, whose three loads are each monotone and
strictly ordered. It does not extend to `in_flight`, which is a gauge and moves in
both directions by design — and the test doc's stated purpose, "the one a live
progress display needs", names exactly the caller most likely to sample that gauge.

## An Invariant Whose Violation Reads As Health

`saturating_sub` exists because `published > claimed` is representable. The suite
explains the choice — plain `u64` subtraction would report 18 quintillion slots in
flight, which is worse than useless — and pins the resulting behaviour with two
assertions, both expecting `0`.

That leaves `in_flight() == 0` reachable three ways: a balanced ring, a caller
recording more publishes than claims, and a real leak sampled across the window
between the two loads. `checked_sub` would have separated the second from the other
two for one word. The method's doc addresses only nonzero readings taken at rest.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every operation that can lower a counter --'
command grep -n 'fetch_sub\|\.store(' ring_stats/src/lib.rs
echo '  -- what guards the cross-counter relation --'
printf '    debug_assert in src: %s   assert in src: %s\n' \
  "$( command grep -c 'debug_assert' ring_stats/src/lib.rs || true )" \
  "$( command grep -c '^  *assert' ring_stats/src/lib.rs || true )"
echo '  -- the subtraction that depends on it --'
command grep -m1 -F '    self.claimed().saturating_sub( self.published() )' ring_stats/src/lib.rs
echo '  -- and the only test that reads while a writer runs --'
command grep -m1 -A4 -F '/// Every counter is monotone while writers run: a reader sampling twice never' ring_stats/tests/stats_test.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| ST21 | `ring_stats` | n/a — coverage | `each_counter_is_monotone_while_writers_run` was the only test in the file that read a counter while another thread wrote, and it samples `published` alone — one of seven, in a body whose name and doc both say "every counter" — pointing the crate's window into concurrent behaviour at the one read with no composition, no second counter, and no seam; two later tests read a derived value and a reset window under live writers, leaving this one still the narrowest of the three |
| ST22 | `ring_stats` | **misleading doc** | Monotonicity is stated on the counters, where it holds, and justified by "the one a live progress display needs" — while `dropped_total` inherits it and `in_flight` cannot, being a gauge that moves in both directions and under-reports whenever it does, so the named use case reaches for the one reading the property excludes |
| ST23 | `ring_stats` | **latent hazard** | `claimed >= published` is depended on by `in_flight`, named in a test doc as "a caller bug" when violated, and enforced by nothing — the crate holds zero `debug_assert` — while `saturating_sub` reports the violation as `0`, the value a healthy ring returns, where `checked_sub` would have reported it as `None` for one word of difference |
| ST24 | `ring_stats` | **misleading doc** | `in_flight() == 0` is reachable three ways — balanced ring, caller bug floored by saturation, and real leak masked by the load window — two of which were chosen deliberately, while the method's doc scopes its claim to nonzero readings taken at rest and says nothing about what a zero does or does not establish |
