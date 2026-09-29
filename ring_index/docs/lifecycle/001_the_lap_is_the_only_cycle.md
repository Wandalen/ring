# Lifecycle: The Lap Is the Only Cycle

### Scope

**Purpose:** Describe the one cycle this crate participates in — a slot's reuse
across laps — locate `ring_index` within it, and record that the gate makes at
most one lap of aliasing reachable while the family ships a function that
measures two.

**Responsibility:** The slot reuse cycle: its phases, its period, and which crate
owns each.

**In Scope:** `ring_index/src/lib.rs:48-52, 73-77`;
`ring_seqno/src/lib.rs:45-55, 65-76`; `ring_slot/src/lib.rs:120-123`.

**Out of Scope:** the crate's own absence of a lifecycle is
[`lifecycle/002`](002_no_initialization_and_no_teardown.md). The aliasing claim
as a doc defect is [`item/001`](../item/001_the_fold_itself.md) IX33.

---

## The Cycle, and Who Owns Each Phase

| Phase | What happens | Owner |
|-------|--------------|-------|
| Gate | Is the slot this sequence would land on free? | `ring_seqno::may_claim` |
| Address | Which slot does this sequence land on? | **`ring_index::of`** |
| Occupy | Write the payload into that slot | `ring_slot`, via `ring_store` |
| Publish | Make the write visible to a consumer | `ring_publish` / stamps |
| Consume | Read it and advance the consumer cursor | `ring_consume` |
| Reuse | The same slot returns, exactly `capacity` sequences later | **`ring_index::of`**, again |

---

### IX53 — The Crate Defines the Cycle's Period and Participates in No Phase of It

**Finding.** `ring_index` appears twice in the table and does the same thing both
times: it answers where a sequence lands. It never observes occupancy, never
waits, never publishes, never advances anything. The reuse row is not a second
behaviour — it is the identical function called again, on a sequence one lap
larger, returning the same answer.

What that means structurally is that this crate sets the cycle's *period* and
takes no part in its *phases*. The period is `capacity`: the slot addressed by
sequence `n` is addressed again by `n + capacity`, and by nothing between. Every
other crate in the table acts within one turn of the cycle; `ring_index` is the
only one whose contribution is the same at every point of it.

That is the concrete reason the crate is testable by enumeration rather than by
scenario. Nothing in the table above needs to exist for `of` to be exercised: the
tests construct a `Capacity` and call a function, ten times over, and never build
a ring
([`lifecycle/002`](002_no_initialization_and_no_teardown.md) IX56).

It is also why `aliases` exists and has no callers
([`item/002`](../item/002_the_two_that_nothing_calls.md) IX35). `aliases` reports
that two sequences share a slot — the period expressed as a predicate. Every
crate that could care about that fact instead consults the gate, which answers
the operationally useful question directly.

---

### IX54 — The Gate Makes Exactly One Lap Reachable, and the Family Measures Two

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the gate --'
command grep -m1 -A6 -F '/// assert!( may_claim( Seq( 4 ), Seq( 1 ), cap ) );  // consumer moved on' ring_seqno/src/lib.rs | tail -n 5
echo '  -- what it computes distance with --'
command grep -m1 -A6 -F '  /// assert_eq!( Seq( 10 ).distance_to( Seq( 4 ) ), 0 );' ring_types/src/id.rs | tail -n 5
echo '  -- and the sibling that measures laps --'
command grep -m1 -A11 -F '/// use ring_seqno::laps_between;' ring_seqno/src/lib.rs | tail -n 11
```

Live output:

```
  -- the gate --
#[ must_use ]
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
{
  consumer.distance_to( producer ) < capacity.get() as u64
}
  -- what it computes distance with --
  #[ must_use ]
  pub const fn distance_to( self, later : Self ) -> u64
  {
    later.0.saturating_sub( self.0 )
  }
  -- and the sibling that measures laps --
///
/// let cap = Capacity::new( 8 ).unwrap();
/// assert_eq!( laps_between( Seq( 0 ), Seq( 7 ), cap ), 0 );
/// assert_eq!( laps_between( Seq( 0 ), Seq( 8 ), cap ), 1 );
/// assert_eq!( laps_between( Seq( 0 ), Seq( 17 ), cap ), 2 );
/// ```
#[ must_use ]
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
{
  earlier.distance_to( later ) / capacity.get() as u64
}
```

**Finding.** `may_claim` is a strict `<`, so a producer is refused at exactly
`distance == capacity` — the moment it would reach one full lap ahead of the
consumer. While the gate holds, the live sequence window is never wider than
`capacity`, so two sequences that alias by *two* laps are never simultaneously
in flight. One lap of aliasing is the entire reachable state space.

That materially changes how [`item/001`](../item/001_the_fold_itself.md) IX33
should be read. `of`'s comment says two sequences "exactly one lap apart" return
the same slot; the arithmetic property is any whole number of laps. The comment
is narrower than the mathematics and *exactly as wide as the gate allows*, which
is why the narrowing has never produced a bug. It remains a defect for the reason
IX33 gives — a reader reasoning about a gate that has fallen behind needs the
general form, and `run` is not gated at all — but it is a defect in the
documentation of a case the running system cannot reach.

The mirror image sits in `ring_seqno` itself. `laps_between` returns a lap count
and its doctest asserts the value `2`, for a distance of 17 slots at capacity 8.
That is a distance `may_claim` — the very next function in the same file, twenty
lines down — exists to prevent. The family therefore ships a measurement of a state its own gate makes
unreachable, and documents the measurement with an example drawn from that
unreachable region.

Neither function is wrong. What is worth recording is that `ring_index` and
`ring_seqno` disagree about which half to document: this crate states the one-lap
case and computes the general one, and its sibling states the multi-lap case and
guards against it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/002`](002_no_initialization_and_no_teardown.md) | Why the crate has no lifecycle of its own, and what that buys the tests |
| [`item/001`](../item/001_the_fold_itself.md) | The one-lap comment, as a doc defect |
| [`item/002`](../item/002_the_two_that_nothing_calls.md) | `aliases`, the period expressed as a predicate nobody consults |
| [`invariant/002`](../invariant/002_the_fold_is_total_and_the_run_is_not.md) | `run`, which is not gated by anything and repeats slots freely |

### Sources

| Fact | Where |
|------|-------|
| The gate's strict `<` | `ring_seqno/src/lib.rs:73-76` |
| The saturating distance it uses | `ring_types/src/id.rs:82-85` |
| `laps_between`'s two-lap doctest | `ring_seqno/src/lib.rs:40-48` |
| The phase table's owners | `ring_slot/src/lib.rs:120-123`; `ring_store/src/lib.rs:220` |

### Tests

| Test | Covers |
|------|--------|
| `aliasing_is_exactly_whole_laps` | The general period, which the gate makes unreachable past one lap |
| `a_sequence_aliases_itself` | The degenerate zero-lap case |
| `a_full_capacity_run_covers_every_slot_once` | One complete turn of the cycle, as a bijection |
| *(to create)* | A test pairing `may_claim` with `aliases`, which is the purpose `aliases`' comment claims |
