# API: Seven Readers, and No Way to Read the Set

### Scope

**Purpose:** Record what the surface does not offer — any means of obtaining more
than one counter at a time — and what every caller therefore has to build.

**Responsibility:** The seven readers, the traits `RingStats` declines, the absence
of a snapshot type, and one caller assembling a picture out of separate loads.

**In Scope:** the seven single-counter readers, `RingStats::snapshot` and
`StatsCounts` in `ring_stats/src/lib.rs`;
`the_counters_are_the_runs_own_totals` in `ring_bench/tests/bench_test.rs`.

**Out of Scope:** The shape of the surface that *is* offered is
[`api/001`](001_fourteen_methods_and_no_exclusive_borrow.md). What each composition
executes is [`algorithm/001`](../algorithm/001_eleven_operations_and_three_compositions.md).

---

## Seven Ways In, One at a Time

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the seven readers, one load each --'
command grep '  pub fn [a-z_]*( &self[^)]*) -> u64' ring_stats/src/lib.rs
echo '  -- the two public structs, and what each derives --'
command grep -m1 -B1 -A1 -F 'pub struct RingStats' ring_stats/src/lib.rs
command grep -m1 -B1 -A1 -F 'pub struct StatsCounts' ring_stats/src/lib.rs
printf '    pub struct in ring_stats/src: %s\n' "$( command grep -c '^pub struct' ring_stats/src/lib.rs || true )"
echo '  -- and the one reader that hands back more than one counter --'
command grep -F '  pub fn snapshot( &self ) -> StatsCounts' ring_stats/src/lib.rs
echo '  -- a caller assembling one picture from separate loads --'
command grep -m1 -A10 -F '  let stats = outcome.stats();' ring_bench/tests/bench_test.rs
```

Live output:

```
  -- the seven readers, one load each --
  pub fn claimed( &self ) -> u64
  pub fn published( &self ) -> u64
  pub fn consumed( &self ) -> u64
  pub fn dropped( &self, policy : OverflowPolicy ) -> u64
  pub fn dropped_total( &self ) -> u64
  pub fn wait_nanos( &self ) -> u64
  pub fn in_flight( &self ) -> u64
  -- the two public structs, and what each derives --
#[ derive( Debug, Default ) ]
pub struct RingStats
{
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash, Default ) ]
pub struct StatsCounts
{
    pub struct in ring_stats/src: 2
  -- and the one reader that hands back more than one counter --
  pub fn snapshot( &self ) -> StatsCounts
  -- a caller assembling one picture from separate loads --
  let stats = outcome.stats();
  assert_eq!( stats.claimed(), outcome.received() as u64 );
  assert_eq!( stats.published(), outcome.received() as u64 );
  assert_eq!( stats.consumed(), outcome.received() as u64 );
  assert_eq!( stats.dropped( policy ), outcome.dropped() as u64 );
  assert_eq!( stats.dropped_total(), outcome.dropped() as u64 );

  // A slot claimed, published, and drained is a slot nobody still holds. The
  // assertion is on the ring's own vocabulary, not on the workload's: it would
  // read 240 if the refused records were mapped through the lifecycle.
  //
```

---

### ST7 — Every Question About Two Counters Is a Question the Caller Has to Assemble

Seven readers return a `u64` each. `RingStats` was the crate's only public struct and
it is the live counter set, not a copy of one: it derives `Debug` and `Default`, and
that is all. No `Clone`, no `Copy`, no `PartialEq`, and — at the time of this finding —
no separate snapshot type anywhere in the file.

So there was no operation on this API that answered a question about two counters. A
caller who wanted to know whether `claimed` and `published` agree called both and
subtracted. A caller who wanted to compare the set now against the set a second ago had
nothing to hold the earlier one in. Even `{:?}` is no exception — a derived `Debug`
formats each field in turn, so printing the struct is seven independent reads taken
at seven moments, not a picture of the set at one.

**Finding.** The crate demonstrates the cost of this in its own source. Its three
composing methods are precisely the three places where someone needed more than one
counter and had to assemble it from separate loads — and all three of the crate's
recorded hazards live in exactly those three methods
([`algorithm/001`](../algorithm/001_eleven_operations_and_three_compositions.md)
§ ST2). `dropped_total` folds three loads and tears in the breakdown beside it;
`in_flight` subtracts two and understates; `reset` stores seven and leaves a window.
Nothing else in the crate can be wrong, because nothing else touches two counters.

`ring_bench` shows the same thing from outside. Six reader calls in nine lines —
which is nine underlying atomic loads, since `dropped_total` is three and `in_flight`
is two — asserted against one `outcome` as though they described a single instant.
The test is correct, because it runs after the workload has finished and the counters
are at rest. But its shape is what every caller has to write, and the reason it is
safe is a property of *when* it runs, which the API gives it no way to state.

---

### ST8 — The Family Has the Type This Crate Would Need, and Does Not Use It Here

`ring_atomic` faced the same problem and answered it. `CountingSeq::counts()` returns
an `OpCounts` — a plain four-field value deriving `Debug, Clone, Copy, PartialEq, Eq,
Default`. It is assembled from four separate loads and therefore tears, exactly as any
multi-counter read must
([`ring_atomic` § AT44](../../../ring_atomic/docs/pitfall/001_the_snapshot_that_never_happened.md)).

The difference is where the tear lives. With a snapshot type, there is one function
that assembles it, one place to document what its result does and does not mean, and
a `Copy`, comparable value the caller can hold, pass, diff against a previous one, or
log. Without one, the same tear is reproduced in every caller's own code, in whatever
order that caller happened to write its reads, with no single place where the caveat
could be attached even if someone wanted to write it.

**Finding.** `ring_stats` has seven counters and the family already shipped the
pattern for handing several out at once, one crate over, with the same author and the
same conventions. It was not used here. That was not obviously the wrong call — a
snapshot of seven `u64` is 56 bytes to copy and a monitor sampling one counter should
not pay for the other six — but the choice was nowhere recorded, and its consequence
was that the crate's only multi-counter operations were the three it wrote for itself,
each of which turned out to be a finding.

`RingStats::snapshot` is now the eighth reader and the only one that hands back more
than one counter, returning a `StatsCounts` — nine plain `u64` fields deriving
`Debug, Clone, Copy, PartialEq, Eq, Hash, Default`. The seven single-counter readers
are unchanged and still cost one load each, so the objection above still holds and is
now answered by having both: a caller sampling one counter pays for one, a caller
wanting a picture calls `snapshot` and gets a value whose own parts agree with each
other. What the snapshot does *not* buy is atomicity, and the type says so in its own
first paragraph — it is still seven loads at seven moments, and a caller needing a
reading the ring genuinely held must still quiesce it first, which is what both
`ring_bench` and `ring_stats`' own contention test do.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/001`](001_fourteen_methods_and_no_exclusive_borrow.md) | The surface that is offered: every method shared-reference, none exclusive |
| [`type/001`](../type/001_a_type_that_cannot_be_copied_compared_or_cloned.md) | The type itself, its two derives, and its seven private fields |
| [`pattern/002`](../pattern/002_the_derived_reading_from_separate_loads.md) | The same assemble-from-loads shape across the family |
| [`algorithm/001`](../algorithm/001_eleven_operations_and_three_compositions.md) | The three places the crate itself needed two counters |

### Sources

| Fact | Where |
|------|-------|
| Seven readers, one `u64` each | Census above |
| `RingStats` declines Clone, Copy and PartialEq | Census above |
| `StatsCounts` derives all three, plus `Eq` and `Hash` | Census above |
| `snapshot` as the one reader returning more than a counter | Census above |
| Six calls, nine loads, asserted as one picture | Census above |
| `OpCounts` derives `Clone, Copy, PartialEq, Eq` | `ring_atomic::OpCounts` |
| `counts()` returns it | `ring_atomic::CountingSeq::counts` |

### Tests

| Test | Covers |
|------|--------|
| `a_fresh_set_is_all_zero` | Seven readers checked one at a time on a quiesced set |
| `reset_returns_every_counter_to_the_fresh_state` | The same, after `reset` |
| `ring_bench`'s `the_counters_are_the_runs_own_totals` | `ring_bench`'s six-call assembly, at rest |
| `a_snapshot_agrees_with_itself_while_writers_run` | The one multi-counter reader, exercised against a set that is still moving |
