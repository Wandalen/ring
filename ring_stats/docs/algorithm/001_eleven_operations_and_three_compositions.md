# Algorithm: Eleven Operations, and the Three That Are Not One

### Scope

**Purpose:** Record what each method of `RingStats` actually executes — how many
atomic operations, in what order — and separate the ten that are a single
instruction from the compositions over several.

**Responsibility:** Every `fetch_add`, `load` and `store` in the crate, and the
methods that issue more than one.

**In Scope:** the recorders, the readers, `RingStats::dropped_total`,
`RingStats::in_flight`, `RingStats::snapshot` and `RingStats::reset` in
`ring_stats/src/lib.rs`.

**Out of Scope:** What the compositions are *documented* to deliver, and what a
caller can therefore be misled into believing, is
[`algorithm/002`](002_in_flight_subtracts_two_moments.md) and
[`pitfall/001`](../pitfall/001_the_leak_in_flight_cannot_see.md). The choice of
`Relaxed` at every site is
[`decisions/001`](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md).

---

## Every Atomic Operation in the Crate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every atomic operation in the crate, by what it does to which counter --'
command grep -o 'self\.[a-z_]*\.\(fetch_add\|load\|store\)(\|counter\.\(fetch_add\|load\|store\)(' ring_stats/src/lib.rs | sort | uniq -c
echo '  -- how many of each, and every ordering used --'
printf '    fetch_add %-4s load %-4s store %-4s  orderings: %s\n' \
  "$( command grep -c 'fetch_add(' ring_stats/src/lib.rs || true )" \
  "$( command grep -c '\.load(' ring_stats/src/lib.rs || true )" \
  "$( command grep -c '\.store(' ring_stats/src/lib.rs || true )" \
  "$( command grep -o 'Ordering::[A-Za-z]*' ring_stats/src/lib.rs | sort -u | tr '\n' ' ' )"
echo '  -- and the four methods that issue more than one --'
command grep -m1 -A3 -F '  pub fn dropped_total( &self ) -> u64' ring_stats/src/lib.rs
command grep -m1 -A4 -F '  pub fn in_flight( &self ) -> u64' ring_stats/src/lib.rs
command grep -m1 -A9 -F '  pub fn snapshot( &self ) -> StatsCounts' ring_stats/src/lib.rs
command grep -m1 -A6 -F '  pub fn reset( &self )' ring_stats/src/lib.rs
```

Live output:

```
  -- every atomic operation in the crate, by what it does to which counter --
      1 counter.fetch_add(
      1 counter.load(
      1 counter.store(
      1 self.claimed.fetch_add(
      2 self.claimed.load(
      1 self.consumed.fetch_add(
      2 self.consumed.load(
      1 self.dropped_newest.load(
      1 self.dropped_oldest.load(
      1 self.failed.load(
      1 self.published.fetch_add(
      2 self.published.load(
      1 self.wait_nanos.fetch_add(
      2 self.wait_nanos.load(
  -- how many of each, and every ordering used --
    fetch_add 5    load 12   store 1     orderings: Ordering::Relaxed 
  -- and the four methods that issue more than one --
  pub fn dropped_total( &self ) -> u64
  {
    OverflowPolicy::ALL.iter().map( | p | self.dropped( *p ) ).fold( 0, u64::saturating_add )
  }
  pub fn in_flight( &self ) -> u64
  {
    self.claimed().saturating_sub( self.published() )
  }

  pub fn snapshot( &self ) -> StatsCounts
  {
    let claimed = self.claimed.load( Ordering::Relaxed );
    let published = self.published.load( Ordering::Relaxed );
    let consumed = self.consumed.load( Ordering::Relaxed );
    let dropped_newest = self.dropped_newest.load( Ordering::Relaxed );
    let dropped_oldest = self.dropped_oldest.load( Ordering::Relaxed );
    let failed = self.failed.load( Ordering::Relaxed );
    let wait_nanos = self.wait_nanos.load( Ordering::Relaxed );

  pub fn reset( &self )
  {
    for counter in self.counters()
    {
      counter.store( 0, Ordering::Relaxed );
    }
  }
```

---

### ST1 — Ten Methods Are One Instruction and the Crate Has No Algorithm

Eleven atomic operations appeared in the source when this was written. Five are
`fetch_add`, one for each recorder; five were `load`, one for each direct reader; one
is a `store`. There is no loop over a retry, no compare-exchange, no arithmetic on a
value read back, and — in the ten methods that touch exactly one counter — no branch
either.

`record_drop` and `dropped` each have a three-arm `match`, and it is not a decision
about the ring: it selects which of three fields to address, on an enum with three
variants and no fallthrough. The generated code is a jump table into three atomics.

**Finding.** Ten of the crate's public methods compile to one atomic operation and a
move — ten of fourteen then, ten of sixteen now. That is what makes the crate's
per-call cost defensible and what makes it uninteresting to read: the recorded value
is exactly the value passed in, at the moment the instruction retires, and nothing
between the caller and the counter can alter it.

The consequence is that every question worth asking about this crate is a question
about the rest — the compositions below and the layout they sit in
([`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md)). It is
the same shape `ring_atomic` has, where the production path is one intrinsic per
method and every finding lives in the shim beside it
([`ring_atomic` § AT1](../../../ring_atomic/docs/algorithm/001_one_intrinsic_or_two.md)).

`RingStats::snapshot` adds seven more `load` sites, taking the crate to eighteen atomic
operations from eleven, and the census above shows every one still `Relaxed`. It joins
the compositions rather than the ten — it is by some distance the largest of them — so
the shape of the finding is unchanged and its denominator moved by one.

---

### ST2 — Three Methods Are Documented as Readings and Executed as Sequences

Three methods issued more than one atomic operation, and none of the three said so.

`dropped_total` folds `OverflowPolicy::ALL` through `dropped`, which is three separate
`Relaxed` loads inside `dropped`, taken at three different moments. `in_flight` is
`claimed().saturating_sub( published() )` — two loads, taken in that order. `reset`
walks an array of seven references and stores zero into each, seven stores from one
`for` statement.

Their contracts read as single readings: "Items lost across every policy", "Slots
claimed but not yet published", "Reset every counter to zero". Each is the correct
description of a state; none is a description of a sequence.

**Finding.** The gap is not that the compositions are wrong — each computes exactly
what it says over the values it loaded. It is that each returns or establishes a
result about *the set*, assembled from operations that are individually atomic and
jointly are not, so the set was never in the state the result describes. The three
differ in how much that costs:

- `dropped_total` cannot be caught out by its own magnitude, because all three
  counters climb and any interleaving lands the sum between its value at the first
  load and at the last. The tear surfaces in the per-policy *breakdown* beside it —
  measured at up to 74,441 impossible spreads per two million reads
  ([`pitfall/002`](../pitfall/002_the_total_that_counts_a_refusal_as_a_loss.md)).
- `in_flight` subtracts one moment from another, so the seam changes the answer
  directly, and always in one direction —
  [`algorithm/002`](002_in_flight_subtracts_two_moments.md).
- `reset` leaves a window in which some counters are zero and others are not,
  observable from outside the call —
  [`lifecycle/001`](../lifecycle/001_zero_to_reset_to_zero.md).

The module comment argues the ordering choice carefully and scopes it to a single
counter: "a stats read is a diagnostic, never a synchronisation point". That is true
of the ten single-operation methods and says nothing about these three
([`decisions/001`](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md)).

There are four now. `RingStats::snapshot` issues seven loads, more than the other three
put together, and it is the one that says so — its own first paragraph opens "**This is
not an atomic snapshot, and no such thing is available here**", names the count as
`RingStats::COUNTERS` loads at that many moments, and states what it does guarantee
instead (that the value's own `dropped_total` and `in_flight` are derived from its own
fields). So the crate now holds both shapes: three compositions documented as readings,
and one documented as the sequence it is. The comparison is the finding's own point
made concrete — what was missing from the three was never a different implementation,
only the sentence.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](002_in_flight_subtracts_two_moments.md) | The composition whose seam changes the value, measured |
| [`decisions/001`](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md) | The ordering argument, and the scope it actually covers |
| [`item/002`](../item/002_the_two_methods_that_are_not_one_operation.md) | Two of the three read as named items rather than as code |
| [`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md) | The layout every operation lands in |

### Sources

| Fact | Where |
|------|-------|
| Every atomic operation site, and its ordering | Census above |
| `dropped_total`'s fold | Census above |
| `in_flight`'s subtraction | Census above |
| `reset`'s seven stores | Census above |
| `snapshot`'s seven loads | Census above |
| The ordering rationale | `ring_stats`'s module comment |

### Tests

| Test | Covers |
|------|--------|
| `each_recorder_moves_exactly_one_counter` | That the five recorders are independent, single-counter operations |
| `recording_zero_changes_nothing` | The degenerate input to every recorder |
| `batched_and_single_recording_agree` | That `record_claim( n )` and `n` calls of `record_claim( 1 )` land the same |
| `a_snapshot_agrees_with_itself_while_writers_run` | That the largest composition's result is internally consistent — which is the strongest thing available, since no composition's result corresponds to a single instant |
