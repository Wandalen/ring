# Pattern: The Derived Reading From Separate Loads

### Scope

**Purpose:** Record the shape `dropped_total` and `in_flight` share — load several
counters inside the accessor, fold them, return one number — and set it beside the two
other shapes the same workspace uses for the same problem, both of which the crate has
since acquired.

**Responsibility:** The derived-reading form in this crate, `ring_seqno::free_slots` as
the parameterised form, `ring_atomic::CountingSeq::counts` as the snapshot form, and
`RingStats::snapshot` and `StatsCounts::checked_in_flight` as this crate's instances of
the latter two.

**In Scope:** `RingStats::dropped_total`, `RingStats::in_flight`,
`RingStats::snapshot` and `StatsCounts::checked_in_flight` in
`ring_stats/src/lib.rs`; `ring_seqno::free_slots` in
`ring_seqno/src/lib.rs`; `ring_atomic::CountingSeq::counts` in
`ring_atomic/src/lib.rs`.

**Out of Scope:** The mirror the two derived readers sit outside is
[`pattern/001`](001_the_record_and_read_pair.md). What `in_flight`'s seam costs, in
measured terms, is
[`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md).

---

## One Problem, Three Shapes, One Workspace

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the two derived readings, and what each folds --'
command grep -m1 -A3 -F '  pub fn dropped_total( &self ) -> u64' ring_stats/src/lib.rs
command grep -m1 -A4 -F '  pub fn in_flight( &self ) -> u64' ring_stats/src/lib.rs
echo '  -- the same derivation as a free function over already-read values --'
command grep -m1 -A7 -F '/// assert_eq!( free_slots( Seq( 4 ), Seq( 0 ), cap ), 0 );' ring_seqno/src/lib.rs | tail -n 6
echo '  -- and as a field computed inside one snapshot --'
command grep -m1 -A14 -F '  pub fn counts( &self ) -> OpCounts' ring_atomic/src/lib.rs
echo '  -- the snapshot shape, now in this crate as well --'
command grep -m1 -A9 -F '  pub fn snapshot( &self ) -> StatsCounts' ring_stats/src/lib.rs
echo '  -- and the two fields it derives from those locals rather than from fresh reads --'
command grep -m1 -A2 -F '      dropped_total : dropped_newest + dropped_oldest + failed,' ring_stats/src/lib.rs
echo '  -- the parameterised shape, over a value the caller already holds --'
command grep -m1 -A5 -F '  pub const fn checked_in_flight( &self ) -> Option< u64 >' ring_stats/src/lib.rs
```

Live output:

```
  -- the two derived readings, and what each folds --
  pub fn dropped_total( &self ) -> u64
  {
    OverflowPolicy::ALL.iter().map( | p | self.dropped( *p ) ).fold( 0, u64::saturating_add )
  }
  pub fn in_flight( &self ) -> u64
  {
    self.claimed().saturating_sub( self.published() )
  }

  -- the same derivation as a free function over already-read values --
#[ must_use ]
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
{
  let in_flight = consumer.distance_to( producer );
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
}
  -- and as a field computed inside one snapshot --
  pub fn counts( &self ) -> OpCounts
  {
    let loads = self.loads.load( Ordering::Relaxed );
    let stores = self.stores.load( Ordering::Relaxed );
    let fetch_adds = self.fetch_adds.load( Ordering::Relaxed );
    let compare_exchanges = self.compare_exchanges.load( Ordering::Relaxed );
    OpCounts
    {
      loads,
      stores,
      fetch_adds,
      compare_exchanges,
      total : loads + stores + fetch_adds + compare_exchanges,
    }
  }
  -- the snapshot shape, now in this crate as well --
  pub fn snapshot( &self ) -> StatsCounts
  {
    let claimed = self.claimed.load( Ordering::Relaxed );
    let published = self.published.load( Ordering::Relaxed );
    let consumed = self.consumed.load( Ordering::Relaxed );
    let dropped_newest = self.dropped_newest.load( Ordering::Relaxed );
    let dropped_oldest = self.dropped_oldest.load( Ordering::Relaxed );
    let failed = self.failed.load( Ordering::Relaxed );
    let wait_nanos = self.wait_nanos.load( Ordering::Relaxed );

  -- and the two fields it derives from those locals rather than from fresh reads --
  -- the parameterised shape, over a value the caller already holds --
  pub const fn checked_in_flight( &self ) -> Option< u64 >
  {
    self.claimed.checked_sub( self.published )
  }
}
```

---

### ST39 — The Family Has Three Shapes for This and This Crate Picked the Opaque One

Deriving a reading from several atomics is not a novel problem, and this workspace has
already solved it twice, differently.

`ring_seqno::free_slots` takes the values as parameters. It performs no atomic operation
at all: the caller reads the two cursors, decides when, and hands them in. Whatever
consistency the result has is the consistency the caller arranged, visibly, at the call
site. The derivation and the reading are separate concerns and the function only does
the first.

`ring_atomic::CountingSeq::counts` performs the loads itself and then derives from
locals. Four loads land in four bindings, and `total` is computed from those four
bindings rather than from four fresh reads. Each load may catch a different moment, but
the struct handed back is internally consistent by construction — `total` is always
exactly the sum of the four values printed beside it, and a caller can re-derive it and
get the same answer.

`ring_stats` did neither. `dropped_total` and `in_flight` load inside the accessor and
return the fold alone. The components are not returned, so the caller cannot see which
values produced the number and cannot re-derive it; calling `dropped( p )` afterwards
for the breakdown issues three more loads at three more moments. Both readers still
behave exactly this way — nothing about them was changed.

**Finding.** The third shape is the only one of the three that can hand back a number
no state ever held *and* withhold the evidence. `free_slots` cannot, because it never
reads. `counts` cannot, because the parts come back with the whole. This crate's two
derived readers do both, and for a while they were the only route it offered.

The near-miss is `ring_seqno`, which computes almost exactly this crate's headline
reading and names the intermediate `in_flight` outright — the working version of
`RingStats::in_flight`, taking parameters, four crates away.

The crate now carries all three shapes rather than one. `RingStats::snapshot` is the
`counts` shape, structurally identical: the counters load into locals, then
`dropped_total` and `in_flight` are computed from *those* locals and handed back
beside the components they came from, so the parts arrive with the whole and the
caller can re-derive either one. `StatsCounts::checked_in_flight` is the `free_slots`
shape, performing no atomic operation at all — it derives from two `u64` the caller is
already holding, and returns `None` on the underflow that `in_flight`'s
`saturating_sub` folds onto zero, which is the healthy reading
([`pitfall/001`](../pitfall/001_the_leak_in_flight_cannot_see.md) § ST41). The opaque
form stays, and stays right for a monitor sampling one number; what changed is that it
is no longer the only form, so a caller who needs the evidence alongside the number can
now ask for it ([`api/002`](../api/002_seven_readers_and_no_way_to_read_the_set.md)
§ ST8).

---

### ST40 — The Opaque Shape Is Applied to the Two Readings Anyone Would Actually Read

Five of the seven readers are a single `Relaxed` load and are exact: `claimed`,
`published`, `consumed`, `dropped( policy )`, `wait_nanos`. Whatever they return, the
counter held at the instant the instruction retired.

The two that are not are `dropped_total` and `in_flight` — and those are the two the
crate itself puts forward. The module comment holds up `in_flight` as the reading that
detects a leak: "a nonzero reading here at rest means a producer took a slot and
abandoned it, which is a leak of ring capacity." `dropped_total` is the single number a
monitor prints to answer whether the ring is losing traffic at all.

**Finding.** The exact readings are the ones a caller has least reason to look at, and
the two summary readings — the ones a dashboard shows and an operator reacts to — are
the derived form. That is not a coincidence but it is not examined anywhere either: the
crate's ordering rationale reasons about "a stats read" in the singular
([`decisions/001`](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md)
§ ST13), which is true of the five and is the wrong unit of analysis for the two.

Both were cheap to convert, and both have been. The snapshot form matching
`ring_atomic` is `RingStats::snapshot`, which fixes `dropped_total` against its own
breakdown — that is the whole of what
[`pitfall/002`](../pitfall/002_the_total_that_counts_a_refusal_as_a_loss.md) § ST44
was measuring — and gives the crate the consistent read
[`api/002`](../api/002_seven_readers_and_no_way_to_read_the_set.md) § ST8 recorded as
missing. The parameterised form matching `ring_seqno` is
`StatsCounts::checked_in_flight`, over values already read rather than over parameters,
which is the same idea reached through the snapshot instead of past it.

What the conversions did not change is this finding's own subject. The five exact
readers are still exact and still the ones nobody looks at; `dropped_total` and
`in_flight` are still derived, still opaque, and still what the module comment puts
forward. The two summary readings a dashboard shows remain the two the crate cannot
answer exactly — a caller now has a route that is at least self-consistent, but only
if it knows to take it, and nothing forces it to. The ordering rationale still reasons
about "a stats read" in the singular, which is now the wrong unit for three readings
rather than two.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/001`](001_the_record_and_read_pair.md) | The mirror these two readers sit outside |
| [`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) | The seam measured, and why it errs in one direction |
| [`api/002`](../api/002_seven_readers_and_no_way_to_read_the_set.md) | The snapshot reader the crate lacked, and now has |
| [`pitfall/002`](../pitfall/002_the_total_that_counts_a_refusal_as_a_loss.md) | `dropped_total` against the breakdown beside it |

### Sources

| Fact | Where |
|------|-------|
| `dropped_total` folds three loads | Census above |
| `in_flight` subtracts two | Census above |
| `free_slots` takes its values as parameters | Census above |
| `counts` derives `total` from its own locals | Census above |
| `snapshot` derives both from its own locals | Census above |
| `checked_in_flight` derives from values already read | Census above |
| `in_flight` as the crate's headline reading | `RingStats::in_flight`'s doc comment |

### Tests

| Test | Covers |
|------|--------|
| `in_flight_is_claimed_minus_published` | The derivation, at rest |
| `the_drop_total_is_the_sum_over_every_policy` | That the fold covers all three policies |
| `consuming_does_not_affect_in_flight` | That the derivation reads the right two counters |
| `a_snapshot_agrees_with_itself_while_writers_run` | The snapshot shape's defining property — each derived field asserted against the components returned beside it, across 50,000 reads taken while four writers run |
