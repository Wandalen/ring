# type

One public type, seven private fields, all the same width, and a derive list of two.
Read as a type rather than as an API, `RingStats` is defined mostly by what it cannot
be: not `Copy`, not `Clone`, not `PartialEq`, not `Hash`, because `AtomicU64` forecloses
each and a hand-written version of any of them would be a snapshot taken at seven
moments. That refusal is correct, and it is the same refusal that leaves the crate with
no way to hand a caller all seven counters at once.

The one trait that does read all seven is `Debug`, and it does exactly what the others
are forbidden from doing. The one field that is not a count of items is `wait_nanos`,
and nothing in its type says so.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_type_that_cannot_be_copied_compared_or_cloned.md) | A Type That Cannot Be Copied, Compared or Cloned | The derive list, the traits the fields foreclose, and `Debug`'s seven loads |
| [002](002_seven_counters_and_one_width.md) | Seven Counters and One Width | One `u64` for two kinds of quantity, and a duration summed across threads |

## The Half of a Design That Shipped

`#[ derive( Debug, Default ) ]` is short because five common traits are unimplementable
over live atomics — and the standard answer is four crates away.
`ring_atomic::OpCounts` derives `Debug, Clone, Copy, PartialEq, Eq, Default` over plain
`usize` fields, because it is not the counters but what `counts()` returns after
reading them. Two types, two halves of one design; this crate ships the first.

`Debug` is derived anyway, and `AtomicU64`'s `Debug` is a relaxed load, so `{:?}`
performs seven independent loads and prints them as one struct literal. Bumping all
seven in lockstep and formatting two hundred thousand times, over ninety-nine percent
of renderings show a spread the set never held — running to 1,940 and 19,831 on
counters never more than one apart. It is the widest-windowed read in the crate and the
only one the type permits, and it is what a log line, an assertion message or a
debugger watch prints.

## One Width, Two Meanings

Six counters count items and one counts nanoseconds, and every signature is `u64`.
`record_claim( &self, n : u64 )` and `record_wait( &self, nanos : u64 )` differ by an
identifier; `claimed()` and `wait_nanos()` differ by a method name. `ring_types` —
which this crate declares and imports from — ships `Seq`, `Capacity` and `SlotIndex`
precisely to make quantity kinds visible, and `ring_stats` imports one enum from it and
nothing else.

The width itself is right: item counters take on the order of five thousand years to
wrap at the measured record rate, nanoseconds about five hundred and eighty. What it
hides is aggregation. `record_wait` takes `&self`, so twelve producers waiting through
one millisecond add twelve, and `wait_nanos`' entire contract is four words with no
subject. Divide it by elapsed time for the fraction of a run spent waiting — the
obvious use, and the one this counter was originally requested for — and the answer can exceed one.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the derive list, and the one hand-written impl --'
command grep -m1 -A1 -F '#[ derive( Debug, Default ) ]' ring_stats/src/lib.rs
command grep -n '^impl ' ring_stats/src/lib.rs
echo '  -- against the snapshot type next door --'
command grep -m1 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq, Default ) ]' ring_atomic/src/lib.rs
echo '  -- one width, two kinds of quantity --'
command grep -m1 -B1 -A8 -F 'pub struct RingStats' ring_stats/src/lib.rs | command grep -o ' : .*'
command grep -n 'pub fn record_claim(\|pub fn record_wait(' ring_stats/src/lib.rs
echo '  -- and the newtypes the family offers but this crate does not import --'
command grep -n '^use ring_types' ring_stats/src/lib.rs
command grep -rn 'pub struct Seq\|pub struct Capacity\|pub struct SlotIndex' ring_types/src/
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| ST45 | `ring_stats` | n/a — observation | `RingStats` derives only `Debug` and `Default` and implements no trait by hand, because `AtomicU64` forecloses `Clone`, `Copy`, `PartialEq`, `Eq` and `Hash` — each would need a consistent read of seven counters — and where `ring_atomic::OpCounts` derives all six over plain `usize` fields, this crate shipped only the live-counter half; `StatsCounts` is now the value half, deriving those five plus `Hash` over nine plain `u64` fields |
| ST46 | `ring_stats` | **latent hazard** | `Debug` is derived and `AtomicU64`'s `Debug` is a relaxed load, so `{:?}` performs seven independent loads and prints them as one struct literal — the consistent snapshot every other route is closed against — and with all seven bumped in lockstep, over ninety-nine percent of two hundred thousand renderings show a spread the set never held, running to 19,831 on counters never more than one apart |
| ST47 | `ring_stats` | n/a — doc gap | All seven fields are `AtomicU64` and all seven readers return `u64`, so the six counters of items and the one counter of nanoseconds are indistinguishable to the compiler — `record_claim( n : u64 )` and `record_wait( nanos : u64 )` differ by an identifier — while `ring_types`, which this crate declares, ships `Seq`, `Capacity` and `SlotIndex` for exactly this purpose and `ring_stats` imports only `OverflowPolicy` |
| ST48 | `ring_stats` | n/a — doc gap | `record_wait` takes `&self` and is additive like every other recorder, so `wait_nanos` is a sum over waiters rather than a span of wall-clock time and can exceed the process lifetime — against a four-word contract, "Nanoseconds spent waiting.", with no subject, no producer count to normalise by, and no caller anywhere in the workspace to have surfaced it |
