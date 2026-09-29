# Type: Seven Counters and One Width

### Scope

**Purpose:** Record that all seven counters share one type carrying no unit, that six
of them count items and the seventh counts nanoseconds, and what that costs a reader.

**Responsibility:** `AtomicU64` across all seven fields, `u64` across all seven
readers, the units the family already knows how to express, and what a summed duration
means across threads.

**In Scope:** the two `use` lines, the seven `RingStats` fields, `record_claim`,
`record_wait`, `claimed` and `wait_nanos` in `ring_stats/src/lib.rs`;
`ring_types/src/id.rs`; `ring_types/src/capacity.rs`.

**Out of Scope:** What the derive list says about the type is
[`type/001`](001_a_type_that_cannot_be_copied_compared_or_cloned.md). The 56-byte
footprint and its cache-line consequence are
[`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md).

---

## One Type, Two Kinds of Quantity

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the crate imports, and what ring_types offers beside it --'
command grep '^use \|^pub use ' ring_stats/src/lib.rs
command grep -r 'pub struct Seq\|pub struct Capacity\|pub struct SlotIndex' ring_types/src/
echo '  -- one width for all seven fields --'
command grep -m1 -B1 -A8 -F 'pub struct RingStats' ring_stats/src/lib.rs | command grep -o ' : .*'
echo '  -- the two pairs that differ only in a parameter name --'
command grep 'pub fn record_claim(\|pub fn record_wait(\|pub fn claimed( &self )\|pub fn wait_nanos( &self )' ring_stats/src/lib.rs
echo '  -- and how each of those two readers is documented --'
command grep -m1 -B2 -A1 -F '  pub fn claimed( &self ) -> u64' ring_stats/src/lib.rs
command grep -m1 -B5 -A1 -F '  pub fn wait_nanos( &self ) -> u64' ring_stats/src/lib.rs
```

Live output:

```
  -- what the crate imports, and what ring_types offers beside it --
use core::sync::atomic::{ AtomicU64, Ordering };
use ring_types::OverflowPolicy;
ring_types/src/capacity.rs:pub struct Capacity( usize );
ring_types/src/id.rs:pub struct Seq( pub u64 );
ring_types/src/id.rs:pub struct SlotIndex( pub usize );
  -- one width for all seven fields --
 : AtomicU64,
 : AtomicU64,
 : AtomicU64,
 : AtomicU64,
 : AtomicU64,
 : AtomicU64,
 : AtomicU64,
  -- the two pairs that differ only in a parameter name --
  pub fn record_claim( &self, n : u64 )
  pub fn record_wait( &self, nanos : u64 )
  pub fn claimed( &self ) -> u64
  pub fn wait_nanos( &self ) -> u64
  -- and how each of those two readers is documented --
  /// Slots claimed so far.
  #[ must_use ]
  pub fn claimed( &self ) -> u64
  {
  /// Nanoseconds spent waiting.
  ///
  /// Structurally zero — see [`RingStats::record_wait`] for why nothing writes
  /// it.
  #[ must_use ]
  pub fn wait_nanos( &self ) -> u64
  {
```

---

### ST47 — Six Counters Count Items, One Counts Nanoseconds, and the Type System Cannot Tell

Every field is `AtomicU64` and every reader returns `u64`. Six of the seven count
things that happened to items — slots claimed, items published, consumed, dropped under
each of three policies. The seventh counts elapsed time.

The two are indistinguishable in every position where a compiler could help.
`record_claim( &self, n : u64 )` and `record_wait( &self, nanos : u64 )` differ in one
identifier. `claimed( &self ) -> u64` and `wait_nanos( &self ) -> u64` differ in the
method name alone. Passing a duration to `record_claim` compiles; passing a count to
`record_wait` compiles; adding `claimed()` to `wait_nanos()` compiles and produces a
number with no meaning.

**Finding.** The family already solves this and this crate declines the solution twice
over. `ring_types` — which `ring_stats` declares and imports from — ships
`Seq( pub u64 )`, `Capacity( usize )` and `SlotIndex( pub usize )`, three newtypes
whose whole purpose is making a quantity's kind visible to the compiler. `ring_stats`
imports exactly one item from it, `OverflowPolicy`, and uses bare `u64` for everything
else. `core::time::Duration` is the standard answer for the seventh and is not used
either.

Nothing here is a bug today. It is the reason `record_wait`'s absent producer
([`item/001`](../item/001_five_recorders_and_the_one_nothing_calls.md) § ST26) is
invisible from the type: a caller wiring `ring_wait` to `ring_stats` would find every
signature accepting whatever it passed, in whatever unit, silently.

---

### ST48 — A Summed Duration Across Threads Is Not Elapsed Time, and Nothing Says So

`u64` is the right width for all seven. At the measured ceiling of roughly a hundred
million records per second
([`non_functional_requirement/001`](../non_functional_requirement/001_cheap_enough_to_leave_on.md)
§ ST34) an item counter takes on the order of five thousand years to wrap, and
`wait_nanos` in nanoseconds covers about five hundred and eighty. Overflow is not the
issue and the uniform width costs nothing there.

What the width hides is the aggregation. Every recorder is additive and shared —
`record_wait( &self, nanos : u64 )` takes `&self`, so twelve producers waiting through
the same one-millisecond window add twelve milliseconds between them. The counter is a
sum over waiters, not a span of wall-clock time, and it can exceed the lifetime of the
process it is running in.

**Finding.** `wait_nanos`' entire contract was "Nanoseconds spent waiting." — four
words, no subject. It has since gained two sentences, and they are about a different
gap: that nothing in the workspace writes the counter at all
([`item/001`](../item/001_five_recorders_and_the_one_nothing_calls.md) § ST26). The
unit is exactly as unstated as it was. A reader dividing the value by elapsed time to
get the fraction of the run spent waiting, which is the obvious use and the reason
this counter was originally requested, gets a figure that runs above one as soon as more than one
producer waits concurrently.

Both the fix and the check are unavailable in the current shape. There is no producer
count in the type to divide by, and no way to add one that does not change the struct.
And because `record_wait` has no caller anywhere in the workspace, this has never been
observed: the counter is structurally zero, so its unit ambiguity has never had a
chance to produce a wrong number. It is the one finding in this corpus that cannot yet
be measured, only read off the signature.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/001`](001_a_type_that_cannot_be_copied_compared_or_cloned.md) | What the derive list says the type is |
| [`item/001`](../item/001_five_recorders_and_the_one_nothing_calls.md) | `record_wait`'s missing producer |
| [`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md) | What seven `u64` fields cost as a layout |
| [`non_functional_requirement/001`](../non_functional_requirement/001_cheap_enough_to_leave_on.md) | The record rate the wrap estimate is taken against |

### Sources

| Fact | Where |
|------|-------|
| All seven fields are `AtomicU64` | Census above |
| The crate imports only `OverflowPolicy` | Census above |
| `ring_types` ships three quantity newtypes | `ring_types/src/id.rs:25`, `:99`; `ring_types/src/capacity.rs:23` |
| The two indistinguishable recorder signatures | Census above |
| `wait_nanos`' four-word summary | Census above |

### Tests

| Test | Covers |
|------|--------|
| `each_recorder_moves_exactly_one_counter` | All five recorders, `record_wait` among them, in one unit-free shape |
| `batched_and_single_recording_agree` | The additivity that makes the summed duration a sum |
| `counts_are_exact_under_contention` | Concurrent recording, on counters where the sum is the right answer |
| *(to create)* | Nothing records waits from more than one thread, which is where the summed duration stops meaning elapsed time |
