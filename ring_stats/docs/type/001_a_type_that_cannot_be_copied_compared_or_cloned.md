# Type: A Type That Cannot Be Copied, Compared or Cloned

### Scope

**Purpose:** Read `RingStats` as a type — what it derives, what it cannot derive, and
what the one derive that reads all seven counters actually produces.

**Responsibility:** The derive list, the field types that determine it, the comparison
with the snapshot type next door, and `Debug`'s seven loads.

**In Scope:** `RingStats`'s derive list, doc comment and `impl` block, and
`StatsCounts`, in `ring_stats/src/lib.rs`; `ring_atomic::OpCounts` in
`ring_atomic/src/lib.rs`.

**Out of Scope:** The counters' width and layout are
[`type/002`](002_seven_counters_and_one_width.md). What the API can and cannot return
is [`api/002`](../api/002_seven_readers_and_no_way_to_read_the_set.md).

---

## Two Types, Two Derive Lists

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what RingStats derives, over what fields --'
command grep -m1 -A10 -F '#[ derive( Debug, Default ) ]' ring_stats/src/lib.rs
echo '  -- and what its doc now says about the one derive that reads all seven --'
command grep -m1 -A5 -F '/// **`{:?}` is seven independent loads, not a snapshot.** `Debug` is derived and' ring_stats/src/lib.rs
echo '  -- the value type this crate now ships beside it --'
command grep -m1 -A24 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash, Default ) ]' ring_stats/src/lib.rs | command grep -v '^  ///'
echo '  -- what the snapshot type next door derives, over what fields --'
command grep -m1 -A13 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq, Default ) ]' ring_atomic/src/lib.rs | command grep -v '^  ///'
echo '  -- and every trait implemented by hand here --'
printf '    inherent impl blocks in ring_stats/src: %s, trait impls: %s\n' \
  "$( command grep -c '^impl ' ring_stats/src/lib.rs || true )" \
  "$( command grep -c '^impl .* for ' ring_stats/src/lib.rs || true )"
```

Live output:

```
  -- what RingStats derives, over what fields --
#[ derive( Debug, Default ) ]
pub struct RingStats
{
  claimed : AtomicU64,
  published : AtomicU64,
  consumed : AtomicU64,
  dropped_newest : AtomicU64,
  dropped_oldest : AtomicU64,
  failed : AtomicU64,
  wait_nanos : AtomicU64,
}
  -- and what its doc now says about the one derive that reads all seven --
/// **`{:?}` is seven independent loads, not a snapshot.** `Debug` is derived and
/// `AtomicU64`'s own `Debug` is a relaxed load, so formatting a shared set reads
/// seven values at seven moments and prints them as one struct literal — which
/// is exactly how a consistent reading would look. Bumping all seven in lockstep
/// from a single writer, over ninety-nine percent of renderings showed a spread
/// the set never held, the widest running to 19,831 on counters that were never
  -- the value type this crate now ships beside it --
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash, Default ) ]
pub struct StatsCounts
{
  pub claimed : u64,
  pub published : u64,
  pub consumed : u64,
  pub dropped_newest : u64,
  pub dropped_oldest : u64,
  pub failed : u64,
  pub dropped_total : u64,
  pub in_flight : u64,
  pub wait_nanos : u64,
}

impl StatsCounts
  -- what the snapshot type next door derives, over what fields --
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Default ) ]
pub struct OpCounts
{
  pub loads : usize,
  pub stores : usize,
  pub fetch_adds : usize,
  pub compare_exchanges : usize,
  pub total : usize,
}
  -- and every trait implemented by hand here --
    inherent impl blocks in ring_stats/src: 2, trait impls: 0
```

---

### ST45 — The Missing Derives Are Missing Because the Type Is the Live Counters

`RingStats` derives `Debug` and `Default`, and nothing else. The crate has two `impl`
blocks and both are inherent — no trait is implemented by hand anywhere in it.

Every absence follows from the fields. `AtomicU64` is neither `Copy` nor `Clone`, so
`#[ derive( Clone ) ]` will not compile; a hand-written `Clone` would have to load seven
counters and build a new set from them, which is a snapshot taken at seven moments.
`PartialEq` is the same problem twice over — fourteen loads to compare two sets that
were each never in the state being compared. `Hash` likewise. The type refuses them
because they are not honestly implementable over live atomics, and that refusal is
correct.

`ring_atomic::OpCounts`, four crates away, derives `Debug, Clone, Copy, PartialEq, Eq,
Default` over five plain `usize` fields. It can, because it is not the live counters —
it is what `counts()` hands back after loading them.

**Finding.** The two types are the two halves of one design, and this crate shipped
only the first. A counter set that cannot be copied, compared or cloned is exactly
right for what `RingStats` is; the type that *can* be — the one every consumer
actually wants to hold, print, diff against a previous reading or send somewhere — did
not exist here.

The derive list was the evidence. `#[ derive( Debug, Default ) ]` is short not because
the type is simple but because five common traits are unimplementable over its fields —
and the standard answer to that, in this workspace, was sitting in `ring_atomic` with
its own doctests
([`api/002`](../api/002_seven_readers_and_no_way_to_read_the_set.md) § ST8,
[`pattern/002`](../pattern/002_the_derived_reading_from_separate_loads.md) § ST39).

`StatsCounts` is now the second half, and it derives one trait more than `OpCounts`
does — `Hash`, which `OpCounts` omits. `RingStats`'s own derive list is unchanged and
deliberately so: the live counters still refuse the five traits, for the same reason
they always did. What changed is that refusing them no longer leaves a consumer with
nothing, because the value the derives would have produced dishonestly is now produced
honestly, once, by `RingStats::snapshot`.

---

### ST46 — `Debug` Is the One Derive That Reads All Seven, and It Tears

The type refuses `Clone` and `PartialEq` because a consistent read of seven atomics is
not available. `Debug` is derived anyway, and `AtomicU64`'s own `Debug` implementation
is a relaxed load. So `{:?}` on a shared `RingStats` performs seven independent loads
and formats them as one struct literal — the very snapshot every other route is closed
against.

**Finding.** Bump all seven counters in lockstep from one writer, so the true spread is
never more than one, and format the set two hundred thousand times:

```
  -- seven counters bumped in lockstep, formatted with {:?} --
    renderings taken                       200000
    integers found in each rendering       7
    showing a spread the set never had     199302
    widest impossible spread               1940
  -- the widest one, as printed --
    RingStats { claimed: 4532286, published: 4532288, consumed: 4532288, dropped_newest: 4532290, dropped_oldest: 4532290, failed: 4534202, wait_nanos: 4534226 }
```

```
  -- seven counters bumped in lockstep, formatted with {:?} --
    renderings taken                       200000
    integers found in each rendering       7
    showing a spread the set never had     198548
    widest impossible spread               19831
  -- the widest one, as printed --
    RingStats { claimed: 4363210, published: 4363211, consumed: 4363211, dropped_newest: 4363213, dropped_oldest: 4363214, failed: 4382931, wait_nanos: 4383041 }
```

Over ninety-nine percent of renderings show a spread the set never held, running to
1,940 and 19,831 on counters that were never more than one apart. The printed example
makes it plain: `claimed` and `wait_nanos` are bumped one after the other inside the
same loop iteration, and the formatter prints them two thousand apart.

The magnitude is not incidental. Formatting allocates and writes, so `{:?}` holds the
widest window of any read in the crate — it is simultaneously the slowest way to read
all seven and the only one the type permits. And it is the natural thing to reach for:
a log line, an assertion message, a panic payload, a debugger watch. Every one of those
prints a set that never existed, and prints it as a single value with field names, which
is precisely how a consistent snapshot would look.

Nothing marked it. `Debug` carried no note, the module comment's ordering rationale is
about single reads
([`decisions/001`](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md)
§ ST13), and the suite never formats a `RingStats` beside a running writer.

The derive stays. Removing it would take away the log line, the assertion message and
the debugger watch, and give back nothing — none of those wanted a consistent read,
they wanted *a* read. What was missing was the sentence saying which of the two they
were getting.

**Disposition:** applied — `RingStats`'s own doc comment now opens the subject
directly: that `{:?}` is seven independent loads formatted as one struct literal,
which is exactly how a consistent reading would look; the measured
ninety-nine-percent-of-renderings figure and the 19,831 spread from the runs above;
and that `Clone`, `Copy` and `PartialEq` are refused for precisely the reason `Debug`
is kept. It closes by naming `RingStats::snapshot` as the reading to take when the
numbers have to agree with each other — which is the route that did not exist when
this finding was written. Now prints: `/// the set never held, the widest running to 19,831 on counters that were never`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/002`](002_seven_counters_and_one_width.md) | What the seven fields are made of, and what that costs |
| [`api/002`](../api/002_seven_readers_and_no_way_to_read_the_set.md) | The snapshot method the type has no way to offer |
| [`pattern/002`](../pattern/002_the_derived_reading_from_separate_loads.md) | `OpCounts` as the shape this crate is missing |
| [`pitfall/002`](../pitfall/002_the_total_that_counts_a_refusal_as_a_loss.md) | The same tearing, on the three counters someone reads deliberately |

### Sources

| Fact | Where |
|------|-------|
| `#[ derive( Debug, Default ) ]` over seven `AtomicU64` | Census above |
| Two `impl` blocks, both inherent | Census above |
| `StatsCounts` derives seven traits over plain `u64` | Census above |
| `OpCounts` derives six traits over plain `usize` | Census above |
| What `RingStats`'s doc now says about `{:?}` | Census above |
| `Debug`'s tear, two runs | Probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_fresh_set_is_all_zero` | The `Default`-equivalent state, via `new` |
| `recording_needs_only_a_shared_reference` | The property that forces the interior mutability behind all this |
| `counts_are_exact_under_contention` | Every counter read individually, after joining |
| `a_snapshot_reaches_every_counter_and_a_reset_clears_every_one` | That `StatsCounts` carries each of the seven, on values distinct enough to catch a field wired to the wrong counter |
| *(not creatable)* | Nothing formats a `RingStats` while a writer runs — a test could only assert the tear is *possible*, which is a rate too rare to require without flakiness; the doc records the measurement instead |
