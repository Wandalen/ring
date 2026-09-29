# Lifecycle: Born at Zero, Climbing Until Dropped

### Scope

**Purpose:** Record the whole life of a cell — construction, the only direction it
moves, and what happens at the end — and the gap between the capability the
primitive offers and the one the family exercises.

**Responsibility:** `AtomicSeq`/`CountingSeq` construction, every sequence store in
the family, the absence of a destructor, and what the family means by *reset*.

**In Scope:** `ring_atomic/src/lib.rs:176-180`, `:220`, `:485`;
`ring_consume/src/lib.rs:433`, `:475`; `ring_mpsc/src/lib.rs:1270`;
`ring_spsc/src/lib.rs:794`, `:1137`;
`ring_shutdown/src/lib.rs:633-666`.

**Out of Scope:** That the primitive *permits* going backwards, and what the family
does about it, is [`invariant/001`](../invariant/001_monotonicity_is_relied_on_and_not_required.md).
What happens when the climb reaches the top is
[`pitfall/002`](../pitfall/002_the_wrap_that_reads_as_an_empty_ring.md). The six
ways to start a life are [`item/001`](../item/001_six_constructors_for_two_types.md).

---

## Every Store in the Family, and the Absent Destructor

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every sequence store in the family that is code and not a doctest --'
command grep -rE '\.store\( ' --include=lib.rs ring_*/src/ \
  | command grep -vE ': *//' | command grep -v 'Ordering::Relaxed )' \
  | sed 's|ring/||;s|/src/lib.rs||'
echo '  -- what a cell is born holding --'
command grep -m1  -A5 -F '  /// crate should be pinned to.' ring_atomic/src/lib.rs | tail -n 5
echo '  -- every Drop impl in the 33 crates --'
command grep -r 'impl.*Drop for' --include=lib.rs ring_*/src/ | sed 's|ring/||;s|/src/lib.rs||'
echo '  -- and what the family calls a reset --'
command grep -m1 -A6 -F 'pub fn reset< T : Send >( shutdown : &Shutdown, consumer : &mut Consumer< '"'"'_, T > ) -> usize' ring_shutdown/src/lib.rs
```

Live output:

```
  -- every sequence store in the family that is code and not a doctest --
ring_atomic:    self.0.store( value.0, order );
ring_atomic:    self.cell.store( value, order );
ring_consume:    self.cursor.store( through, COMMIT );
ring_consume:      self.cursor.store( end, COMMIT );
ring_cursor:    self.0.get().store( value, order );
ring_mpsc:    self.ring.stamp( self.seq ).store( self.seq, PUBLISH );
ring_mpsc:      .store( self.start.advanced_by( self.len as u64 ), COMMIT );
ring_shutdown:    self.closed.store( true, Ordering::Release );
ring_shutdown:    self.shutdown.closed.store( false, Ordering::Release );
ring_spsc:    self.ring.cursors.producer().store( self.seq.next(), HANDOFF );
ring_spsc:    self.ring.cursors.consumer().store( self.start.advanced_by( self.len as u64 ), HANDOFF );
  -- what a cell is born holding --
  fn default() -> Self
  {
    Self::new( Seq::ZERO )
  }
}
  -- every Drop impl in the 33 crates --
ring_mpsc:impl< S > Drop for Reserved< '_, S >
ring_mpsc:impl< S > Drop for Batch< '_, S >
ring_spsc:impl< S > Drop for Reservation< '_, S >
ring_spsc:impl< S > Drop for Batch< '_, S >
  -- and what the family calls a reset --
pub fn reset< T : Send >( shutdown : &Shutdown, consumer : &mut Consumer< '_, T > ) -> usize
{
  let stopped = shutdown.close();
  let discarded = stopped.discard_all( consumer );
  stopped.reopen();
  discarded
}
```

---

### AT29 — Five Cursor Writes in 33 Crates, and Every One of Them Computes a Value That Cannot Be Lower

Eleven `store` sites in the family's non-doctest code. Two write an `AtomicBool`
(`ring_shutdown`'s closed flag). Three are the primitives themselves —
`ring_atomic`'s two impls and `ring_cursor`'s forwarding one — which write whatever
they are handed. One (`ring_mpsc:990`) stamps a slot rather than moving a cursor.

That leaves five sites where the family actually decides what a cursor becomes, and
all five write an expression rather than a value: `through` and `end`, both clamped
by `ring_consume`'s commit; `self.start.advanced_by( self.len as u64 )` twice; and
`self.seq.next()`. Not one of them can produce something below what the cursor
already held.

**Finding.** The primitive offers a capability that the entire family declines to
use. `store` accepts any `Seq`, three methods move a cell backwards with no refusal
([`invariant/001`](../invariant/001_monotonicity_is_relied_on_and_not_required.md)
AT21), and across 33 crates the only code that exercises the reverse direction is
`ring_debug`'s doctest, which does it deliberately to trigger a violation.

That is a coherent state of affairs and worth naming as one: the guard the type
does not carry is supplied instead by the *shape of every call site* — an
`advanced_by`, a `next()`, a clamp — and it holds today because five expressions
happen to be written that way. Nothing checks that a sixth would be. The census
above is the check, and it takes four lines of shell.

---

### AT30 — There Is No Destructor, No Rewind, and No Reuse: Slots Recycle and Sequences Do Not

A cell is born holding `Seq::ZERO`, or whatever `new` was handed. It has no `Drop`
impl — the four in the family are all on RAII publish guards (`Reserved`,
`Reservation`, `Batch`), which commit a claim when they fall out of scope and touch
the cursor only forward. When the cell itself goes, the compiler drops eight bytes
and that is the whole teardown.

Nor is there a rewind. The family's own reset operation,
`ring_shutdown::reset`, is close-then-discard-then-reopen: it advances the consumer
until the ring is empty and leaves both cursors exactly where the traffic put them.
Its own doc says the result "behaves as a freshly built one of the same capacity,
on the same allocation" — true of the *slots*, and not of the sequence numbers:

```
  a ring of 4 slots, three full cycles through it

    cycle   claimed        slots used        producer after
        0   [0, 1, 2, 3]   [0, 1, 2, 3]   Seq(4)
        1   [4, 5, 6, 7]   [0, 1, 2, 3]   Seq(8)
        2   [8, 9, 10, 11]   [0, 1, 2, 3]   Seq(12)

  the four slots were reused three times; no sequence was
  and the cursor a fresh Claimer would start from : Seq(0)
  while this one stands at                        : Seq(12)
```

**Finding.** The distinction is the whole point of the design — a monotonic
sequence is what makes a slot's contents unambiguous when the ring wraps — and it
is stated nowhere in this crate, whose four-word `store` contract is the only
description a caller gets of what writing to a cursor means.

The observable consequence is small but real: `ring_core` exposes fourteen public
methods across `Ring`, `Producer`, and `Consumer`, and not one of them returns a
`Seq`. A caller holding only the ordinary ring API cannot read how far its cursors
have climbed; that number is reachable only by holding a `GatingSet` or a `Claimer`
directly. For a long-running process this is the single number that says how much
of the sequence space is spent, and it is behind the lower-level API rather than the
ordinary one.

The climb is not urgent — at the ~14ns per operation measured for a contended cell
([`data_structure/001`](../data_structure/001_one_word_and_five.md) AT10), `u64`
lasts on the order of eight thousand years — so this is an observability gap, not a
countdown. It matters because the *shape* of the lifecycle, one-way and
never-reset, is the assumption every gate in the family is written against, and
nothing in the crate that owns it says so.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/002`](002_from_task_file_to_crate.md) | The other lifecycle here — how the crate itself came to exist |
| [`invariant/001`](../invariant/001_monotonicity_is_relied_on_and_not_required.md) | The backwards capability these five call sites decline to use |
| [`pitfall/002`](../pitfall/002_the_wrap_that_reads_as_an_empty_ring.md) | The far end of the climb, and what reads it wrongly |
| [`item/001`](../item/001_six_constructors_for_two_types.md) | The six ways a cell's life starts |
| [`item/002`](../item/002_counts_the_method_that_is_not_a_snapshot.md) | `reset_counts`, the only rewind in the crate, and what it does not rewind |

### Sources

| Fact | Where |
|------|-------|
| A cell is born at `Seq::ZERO` | `ring_atomic/src/lib.rs:176-180` |
| The five cursor writes, all computed forward | `ring_consume/src/lib.rs:433`, `:475`; `ring_mpsc/src/lib.rs:1270`; `ring_spsc/src/lib.rs:794`, `:1137` |
| Four `Drop` impls, all on publish guards | `ring_mpsc/src/lib.rs:985`, `:1257`; `ring_spsc/src/lib.rs:788`, `:1130` |
| Reset is discard-and-reopen, not rewind | `ring_shutdown/src/lib.rs:633-666` |
| Slots recycle, sequences climb | Release probe, quoted above |
| Fourteen public ring methods, none returning a `Seq` | `ring_core/src/lib.rs:163-652` |

### Tests

| Test | Covers |
|------|--------|
| `a_fresh_cell_reads_zero` | The start of the life |
| `resetting_the_counts_leaves_the_sequence_alone` | That the crate's one rewind does not touch the sequence |
| `store_overwrites_and_load_observes` | That `store` writes what it is given, in either direction |
| *(to create)* | Nothing asserts that every cursor write in the family computes forward, which is the property the five call sites currently hold by construction |
