# Pattern: A Sequence Stamp as a Lap-Safe Publication Marker

### Scope

- **Purpose**: Name the pattern that lets a wrapping buffer distinguish this lap's record from the previous lap's, and state the condition under which a simpler marker fails.
- **Responsibility**: The problem, the rule, the applicability condition, and the cost.
- **In Scope**: The stamp-equals-sequence test, stated generally enough to reuse.
- **Out of Scope**: The array holding the stamps (→ [`../data_structure/002`](../data_structure/002_a_second_array_of_sequence_stamps.md)); the failure mode when a reader gets the test backwards (→ [`../pitfall/002`](../pitfall/002_a_stale_stamp_reads_as_unpublished_not_as_wrong.md)).

### Problem

A bounded buffer is reused: slot `i` carries sequence `i`, then `i + capacity`,
then `i + 2·capacity`. A reader needs to know whether the payload it is about
to read is the one it wants. A boolean "occupied" flag cannot answer that — it
is `true` for the previous lap's record just as much as for this one, and the
window in which it is wrong is exactly the window in which a producer is
mid-lap.

### The Rule

**Store the sequence, not a flag, and test for equality rather than for truth.**

```rust
// published iff the stamp names the sequence being asked about
fn is_published( &self, seq : Seq ) -> bool
{
  self.stamp( seq ).load( OBSERVE ) == seq
}
```

An unequal stamp is unambiguous in both directions: lower means the slot still
holds an earlier lap's record, and there is no third state to confuse with
either.

### Applicability

Needs all three: the buffer wraps; slots are reused without being cleared; and a
reader can arrive while a producer is partway through a lap. Remove the third
and a flag suffices, because no reader ever sees a slot between laps.

`ring_spsc` has the first two and not the third — one producer and one consumer
whose cursors already bound each other — which is why it uses cursor comparison
and no stamp array at all. **One crate in this family instantiates this pattern
and its sibling does not**, and that difference is the pattern's applicability
condition made concrete.

### Cost

One atomic word per slot (→ [`../data_structure/002`](../data_structure/002_a_second_array_of_sequence_stamps.md)),
and one `Acquire` load per read. The load is not extra — a flag would need the
same edge — so the marginal cost of the pattern over a correct boolean is the
memory, not the synchronization.

### Consequence

The publication test is a pure function of the stamp and the sequence, with no
reference to any cursor. That is what lets `drain` stop at the first
unpublished sequence rather than at the highest published one
(→ [`../algorithm/002`](../algorithm/002_batch_drain_by_cursor_swap.md)) — the
scan needs no coordination beyond the stamps themselves.

### MP42 — `ring_spsc` Has the Same Buffer and Needs No Stamp

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'ring_spsc stamp array:  '; grep -c 'stamps' ring_spsc/src/lib.rs
printf 'ring_mpsc stamp array:  '; grep -c 'stamps' ring_mpsc/src/lib.rs
```

Live output:

```
ring_spsc stamp array:  0
ring_mpsc stamp array:  12
```

One producer and one consumer already bound each other's positions, so no reader
can arrive mid-lap at a slot whose state the cursors do not already determine.
The third applicability condition fails, and the whole array with it.

**Two crates, one storage shape, one stamp array between them.** That is the
cleanest available evidence that the pattern is a response to concurrency rather
than to wrapping.

### MP43 — The Publication Test Is a Pure Function of Stamp and Sequence

`stamp( seq ) == seq` needs no lock, no cursor read, and no agreement with any
other participant. The drain can therefore walk forward from its own position
and stop where the stamps stop, with the only synchronization being the
`OBSERVE` load it was already doing.

That independence is the pattern's real payoff and it is easy to lose: a test
written as "stamp equals seq **and** seq is below the claim cursor" would be
equally correct and would reintroduce a read of shared state per slot.
