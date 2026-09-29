# lifecycle

A counter set has three states and two transitions, and the crate is careful about
both endpoints and silent about the middle. `new` builds an all-zero value with no
atomic operation in it; recording moves counters upward and never down; `reset` walks
seven counters through a shared reference and puts them back. The postconditions are
documented, tested and correct. What is neither documented nor tested is the interval
inside the second transition, during which the set holds a combination of values no
sequence of recordings could produce.

Above that sits a second lifecycle — the crate's own, as the project records it. The
feature specifying `ring_stats` is marked `planned` for an implemented, tested crate,
and the field that says so says the same thing about 402 of 426 feature records. The
two instances here take one lifecycle each, and they meet at the same place: a state
that exists and is observable, on a page that does not mention it.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_zero_to_reset_to_zero.md) | Zero, Counting, Reset, Zero — and the Moment in Between | The runtime states, and the reset window measured from a second thread |
| [002](002_implemented_tested_and_still_planned.md) | Implemented, Tested, and Still Planned | This crate's own feature-record status, the field it sits on, and the one consumer awaiting these counters |

## A Window With Its Own Signature

`reset` clears the counters in field order — `claimed` first, `wait_nanos` last — and
a fill in the same order sets `claimed` before `wait_nanos`. So the two seams read
oppositely: a sample caught mid-fill is `( set, 0 )`, a sample caught mid-reset is
`( 0, set )`, and the second is a combination no complete state can hold. That makes
the window not merely present but *identifiable*, which is what let a probe count it:
one writer looping fill-then-reset against a reader taking two million pairs caught
the reset seam 101, 5, 69, 30, 16 and 16 times across six runs. Always there, never
the same number twice.

Both reset tests run on one thread, where the window cannot appear. Every other
concurrency seam in this crate is tested the same way — read after join, or one
counter sampled with no composition — and here it is sharpest, because this is the
one seam whose intermediate state is not merely stale but structurally impossible.

## A Record That Cannot Say the Crate Exists

This crate's own feature record says `planned`. So do 402 of 426 feature records; the 23 that say
`present` are one contiguous orbital-mechanics run marked once and never revisited.
The field is uniform, so it reports nothing — and a field that did report would be the
natural place for this crate's two half-deliveries to appear: a counter this crate's own feature record
names by name with no producer anywhere, and a `reset` doc citing a caller in a crate
that does not depend on this one.

The one consumer feature that would read these counters asks for `drops` alone, is
itself `planned`, and reserves the right to be dropped "when the cost of the counters
is [not] already being paid" — a cost the crate asserts and never measured.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two transitions --'
command grep -n 'pub const fn new\|pub fn reset' ring_stats/src/lib.rs
echo '  -- the order reset walks --'
command grep -m1 -A4 -F '    for counter in' ring_stats/src/lib.rs
echo '  -- the two tests that observe one, and their thread count --'
command grep -n 'fn reset_returns_every_counter_to_the_fresh_state\|fn a_reset_set_counts_again' ring_stats/tests/stats_test.rs
printf '    threads spawned across both: %s\n' \
  "$( command grep -m1 -B14 -A20 -F '  stats.record_consume( 1 );' ring_stats/tests/stats_test.rs | command grep -c 'spawn\|thread::scope' || true )"
# the crate lifecycle, as the external feature record had it, lived in
# docs/feature/*.md — a pre-implementation design corpus external to this
# repository, unreachable since extraction; the figures it once produced are
# preserved as a historical note in lifecycle/002.md
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| ST29 | `ring_stats` | **latent hazard** | `reset` stores zero into seven counters in field order through a shared reference, and because a fill sets `claimed` before `wait_nanos` while a reset clears them in the same order, a concurrent read caught mid-reset returns `( 0, set )` — a combination no fill produces and no complete state can hold, measured at 101, 5, 69, 30, 16 and 16 catches per two million samples across six runs, against a doc that describes only the postcondition |
| ST30 | `ring_stats` | n/a — coverage | Both reset tests called `reset` and asserted afterwards on a single thread, spawning zero threads between them, which is precisely the condition under which the window cannot appear; `a_reader_beside_a_reset_sees_only_values_the_writer_wrote` now runs a reader through 20,000 fill-then-reset rounds and asserts on every read that `claimed` and `wait_nanos` each hold a value the writer actually wrote, leaving the two originals single-threaded and correct as the postcondition checks they are |
| ST31 | `ring_stats` | **misleading doc** | This crate's own feature record now reads `Status: present`, flipped as part of the contiguous 167-188 block rather than by any check of this crate — `present` is a minority of the records and falls in eight contiguous runs, two of them 22 features long, and the census caught a ninth feature joining one of those runs between two gate runs while nothing about this crate changed; so the field is a per-block marker that cannot carry a per-feature fact, and the crate's two half-deliveries (a named counter with no producer, a documented caller with no dependency edge) still have nowhere to surface |
| ST32 | `ring_stats` | n/a — observation | The one external record that reads these counters rather than produces them names `drops` alone out of seven, is itself `planned`, and quotes its source's own escape clause — included "when the cost of the counters is already being paid and dropped when it is not" — making the crate's sole documented consumer conditional on the one property of the crate that has never been measured |
