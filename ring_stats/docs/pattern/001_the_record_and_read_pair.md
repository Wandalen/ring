# Pattern: The Record-and-Read Pair

### Scope

**Purpose:** Name the crate's organising pattern — one writer and one reader per
counter, mirrored down to the parameter — and record where the mirror is exact, where
it stops, and what obeying it cost.

**Responsibility:** The correspondence between the five recorders and the five
counter-naming readers, the two readers outside it, and the field the pattern reached
by calling it a drop.

**In Scope:** the seven `RingStats` fields, the five recorders and the seven readers
in `ring_stats/src/lib.rs`; `OverflowPolicy::reports_failure` in
`ring_types/src/policy.rs`.

**Out of Scope:** The two derived readers are
[`pattern/002`](002_the_derived_reading_from_separate_loads.md). That the sum counts a
refusal as a loss is
[`data_structure/002`](../data_structure/002_three_drop_counters_behind_one_enum.md)
§ ST12; this instance records why the pattern produced it.

---

## The Mirror, and Where It Stops

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the seven counters --'
command grep -m1 -B1 -A8 -F 'pub struct RingStats' ring_stats/src/lib.rs | command grep -o '^  [a-z_]*'
echo '  -- the five writers --'
command grep -o 'pub fn record_[a-z_]*' ring_stats/src/lib.rs
echo '  -- the readers that name a counter --'
command grep -o 'pub fn \(claimed\|published\|consumed\|dropped\|wait_nanos\)\b' ring_stats/src/lib.rs
echo '  -- and the two that name no counter --'
command grep 'pub fn dropped_total\|pub fn in_flight' ring_stats/src/lib.rs
echo '  -- the field the drop verb reaches, and the policy that reaches it --'
command grep 'failed :\|Self::Fail =>\|reports_failure' ring_stats/src/lib.rs ring_types/src/policy.rs
```

Live output:

```
  -- the seven counters --
  claimed
  published
  consumed
  dropped_newest
  dropped_oldest
  failed
  wait_nanos
  -- the five writers --
pub fn record_claim
pub fn record_publish
pub fn record_consume
pub fn record_drop
pub fn record_wait
  -- the readers that name a counter --
pub fn claimed
pub fn published
pub fn consumed
pub fn dropped
pub fn wait_nanos
  -- and the two that name no counter --
  pub fn dropped_total( &self ) -> u64
  pub fn in_flight( &self ) -> u64
  -- the field the drop verb reaches, and the policy that reaches it --
ring_stats/src/lib.rs:  failed : AtomicU64,
ring_stats/src/lib.rs:  pub failed : u64,
ring_stats/src/lib.rs:      failed : AtomicU64::new( 0 ),
ring_types/src/policy.rs:  //   besides. `OverflowPolicy::reports_failure`/`drops_silently` just below
ring_types/src/policy.rs:/// assert!( OverflowPolicy::Fail.reports_failure() );
ring_types/src/policy.rs:/// assert!( !OverflowPolicy::DropNewest.reports_failure() );
ring_types/src/policy.rs:  /// assert_eq!( OverflowPolicy::ALL.iter().filter( | p | p.reports_failure() ).count(), 1 );
ring_types/src/policy.rs:  pub const fn reports_failure( self ) -> bool
ring_types/src/policy.rs:      Self::Fail => true,
ring_types/src/policy.rs:  //   `reports_failure` above — `matches!( self, Self::DropNewest |
ring_types/src/policy.rs:  // Root cause: see `reports_failure` above.
ring_types/src/policy.rs:      Self::Fail => false,
```

---

### ST37 — Five Writers, Five Readers, Mirrored Down to the Parameter

The pattern is one method to add and one method to read, per counter, named after the
counter. Five of each, and the correspondence is exact: `record_claim`/`claimed`,
`record_publish`/`published`, `record_consume`/`consumed`,
`record_wait`/`wait_nanos`, and — the interesting one —
`record_drop( policy, n )`/`dropped( policy )`.

That last pair is what makes the mirror complete rather than approximate. Three of the
seven counters are reached by one method on each side, parameterised by the same
`OverflowPolicy`, with the same three-arm `match` written out twice, byte for byte —
once in `record_drop`, once in `dropped`. The write side and the read side are the
same shape including their branch.

**Finding.** There is no counter you can write and not read, and none you can read and
not write. For a diagnostic type that is the property worth having: a monitor and a
producer refer to the same seven quantities under the same seven names, and a counter
cannot quietly become write-only by drifting away from its accessor.

It is also what makes the exceptions legible. `dropped_total` and `in_flight` are the
only two public readers that name no counter and have no writer, and against a mirror
this exact they stand out on the page rather than needing to be discovered — which is
the whole subject of [`pattern/002`](002_the_derived_reading_from_separate_loads.md).

---

### ST38 — Where the Pattern and the Domain Disagreed, the Pattern Won

The seven counters are `claimed`, `published`, `consumed`, `dropped_newest`,
`dropped_oldest`, `failed`, `wait_nanos`. Six of the names describe what happened to
items. The sixth describes something else: `failed` is the counter for
`OverflowPolicy::Fail`, where the ring refuses the push and hands the item back. No
item is lost. `ring_types` says so directly — `reports_failure` is true for exactly one
of the three policies, and `drops_silently` is true for the other two.

The field is named accordingly. Its accessors are not. It is written by `record_drop`
and read by `dropped`, because the pattern has one writer and one reader per family of
counters and the policy enum has three variants, so all three go under the drop verb.

**Finding.** The mirror is what put a refusal behind a `drop` name, and
`dropped_total` — which folds `OverflowPolicy::ALL` — is what turns that naming into a
number. A ring configured `Fail` that refused a thousand pushes and lost nothing reads
as a thousand items dropped
([`data_structure/002`](../data_structure/002_three_drop_counters_behind_one_enum.md)
§ ST12). The struct field knew the difference; the pattern spent it to stay uniform.

The uniformity was not free elsewhere either, and the trade landed differently each
time. It bought the crate its one genuinely airtight property in ST37, and it cost the
one distinction the domain actually draws between the three policies — a distinction
`ring_types` already exposes as a `const fn`, one call away, unused here.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_stats
command grep -F 'so this sum cannot tell a loss from a refusal' src/lib.rs
```

Live output:

```
    /// policy the same drop verb, so this sum cannot tell a loss from a refusal.
```

**Disposition:** applied — `dropped_total`'s own doc comment now states the cost
this finding traces to the pattern: `OverflowPolicy::Fail` refusals are folded
into the total anyway because the record-and-read mirror gives every policy the
same drop verb, so the sum cannot separate a loss from a refusal — the same fact
`data_structure/002` § ST12 identifies, stated here at the pattern that produced
it rather than at the struct that carries it.
Now prints: `so this sum cannot tell a loss from a refusal`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/002`](002_the_derived_reading_from_separate_loads.md) | The two readers outside the mirror, and how they compute |
| [`data_structure/002`](../data_structure/002_three_drop_counters_behind_one_enum.md) | The three counters behind one enum, and the filter that exists unused |
| [`item/001`](../item/001_five_recorders_and_the_one_nothing_calls.md) | The write half as a family of named items |
| [`api/002`](../api/002_seven_readers_and_no_way_to_read_the_set.md) | The read half, and what it cannot return |

### Sources

| Fact | Where |
|------|-------|
| The seven counter fields | Census above |
| The five recorders and five counter-naming readers | Census above |
| The duplicated three-arm `match` | `RingStats::record_drop` and `RingStats::dropped` |
| `failed` as the `Fail` counter | Census above |
| Exactly one policy reports failure | `ring_types/src/policy.rs:153-160` |

### Tests

| Test | Covers |
|------|--------|
| `each_recorder_moves_exactly_one_counter` | That every writer reaches exactly its own counter |
| `a_drop_lands_under_its_own_policy_only` | That the parameterised pair addresses three counters, not one |
| `refusals_are_counted_even_though_nothing_is_lost` | The naming this finding is about — asserted as correct |
| *(to create)* | Nothing distinguishes a refusal from a loss, because the pattern gave them one verb |
