# Algorithm: Three Readings of Two Cursors

### Scope

- **Purpose**: Show that `free_slots`, `pending`, and `may_claim` are three presentations of one quantity, and state the argument that keeps two of them from ever disagreeing.
- **Responsibility**: Give each reading's body, reduce all three to `consumer.distance_to( producer )`, and say exactly how far the exhaustive agreement test actually reaches.
- **In Scope**: `CursorPair::free_slots`, `pending`, `may_claim` at `src/lib.rs:352-420`; the `ring_seqno` functions they call.
- **Out of Scope**: The fold over a cursor *slice*, which is [`algorithm/001`](001_the_slowest_fold.md); why the capacity is a field rather than a parameter, which is [`decisions/002`](../decisions/002_the_capacity_is_held_by_the_pair.md).

### The Three Bodies

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/  pub fn free_slots/,/^  }/p;/  pub fn pending/,/^  }/p;/  pub fn may_claim/,/^  }/p' \
  ring_cursor/src/lib.rs
```

Live output:

```
  pub fn free_slots( &self ) -> usize
  {
    ring_seqno::free_slots( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
  }
  pub fn pending( &self ) -> u64
  {
    ring_seqno::pending( self.producer.load( GATING ), self.consumer.load( GATING ) )
  }
  pub fn may_claim( &self ) -> bool
  {
    ring_seqno::may_claim( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
  }
```

```rust
ring_seqno::free_slots( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
ring_seqno::pending   ( self.producer.load( GATING ), self.consumer.load( GATING ) )
ring_seqno::may_claim ( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
```

Two loads and a call, three times. This crate performs no arithmetic of its own
on a sequence — `tests/manual/readme.md` M5's second command is the check that
it stays that way, and it is silent.

### One Quantity, Three Presentations

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/pub fn (may_claim|free_slots|pending)/,/^}/' ring_seqno/src/lib.rs | grep -v '///'
```

Live output:

```
pub fn may_claim(producer: Seq, consumer: Seq, capacity: Capacity) -> bool {
    consumer.distance_to(producer) < capacity.get() as u64
}
pub fn free_slots(producer: Seq, consumer: Seq, capacity: Capacity) -> usize {
    let in_flight = consumer.distance_to(producer);
    (capacity.get() as u64).saturating_sub(in_flight) as usize
}
pub fn pending(producer: Seq, consumer: Seq) -> u64 {
    consumer.distance_to(producer)
}
```

| Reading | `ring_seqno` body | In terms of `d = consumer.distance_to( producer )` |
|---------|-----------------|----------------------------------------------------|
| `pending` | `consumer.distance_to( producer )` | `d` |
| `free_slots` | `( capacity.get() as u64 ).saturating_sub( in_flight ) as usize` | `capacity ⊖ d`, saturating |
| `may_claim` | `consumer.distance_to( producer ) < capacity.get() as u64` | `d < capacity` |

**All three are readings of `d`.** `pending` *is* `d`; the other two are `d`
compared against or subtracted from the capacity. That is why `pending` takes no
capacity argument — it is the one reading that does not need to know how big the
ring is, and `pending_ignores_capacity_and_free_slots_does_not` is the test that
pins the distinction:

```rust
assert_eq!( small.pending(), large.pending(), "same distance, same pending" );
assert_ne!( small.free_slots(), large.free_slots(), "different rings, different room" );
```

### Why `may_claim` Is Not `free_slots() > 0`

Its doc comment says it is: *"Exactly [`free_slots`] being non-zero, expressed as
the question a caller actually asks."* The code does not do that — it calls a
second `ring_seqno` function with its own expression.

The two are equivalent, and the equivalence is an argument rather than a
construction:

> `free_slots > 0` ⟺ `capacity.saturating_sub( d ) > 0` ⟺ `d < capacity`
>
> because `saturating_sub` yields zero exactly when `d >= capacity`.

**An argument can be wrong; a shared expression cannot be.** Writing
`self.free_slots() > 0` would have made the claim true by construction and cost
nothing measurable. Keeping them separate buys a reading that says what it means
at the `ring_seqno` layer — and buys the obligation to test the equivalence, which
is what `may_claim_and_free_slots_never_disagree` discharges over 300 states:

```rust
for producer in 0..24u64
{
  for consumer in 0..=producer
  {
    // …
    assert_eq!( pair.may_claim(), pair.free_slots() > 0, "producer {producer}, consumer {consumer}" );
  }
}
```

### How Far That Test Reaches

**It fixes the state, then reads.** Both calls see the same two cursor values,
so what it verifies is that the *arithmetic* agrees — the `saturating_sub`
argument above, checked rather than asserted.

It does not, and cannot, verify agreement across two calls made while another
thread is running. Each reading performs its own pair of loads, so `may_claim()`
followed by `free_slots()` is four loads of two cursors and the results may come
from different states.

That gap is closed by who the caller is rather than by the test:

| Cursor | Who advances it | Effect on `d` |
|--------|-----------------|---------------|
| producer | the caller itself | grows `d` |
| consumer | the peer | **shrinks** `d` |

A `CursorPair` has one producer — `ring_spsc` is its consumer of record — and
that producer is the thread asking. Between its own two calls it does not move
the producer cursor, and the only thing the peer can do is advance the consumer,
which only adds room. So `may_claim()` reporting `true` cannot be followed by
`free_slots()` reporting `0`: `d` can only have shrunk.

**The invariant is "the peer's writes are monotone in the safe direction",** and
it is a property of the pair's single-producer contract, not of the arithmetic.
A second producer on the same `CursorPair` would break it, and no test in this
crate would notice.

### Both Cursors, Every Time

A reading that consulted only the producer returns the right answer whenever the
consumer is at zero — the state most tests start in.
`the_pair_reads_both_cursors_for_every_reading` moves only the consumer and
asserts all three readings change; `tests/manual/readme.md` M4 counts the loads
directly and expects `6`.

M4 is worth noting as a check that was wrong on first run: it used `grep -c`,
which counts matching *lines*, and each body puts both loads on one line. It
reported `3` against an expected `6`, and the number was read as a finding until
the instrument was corrected to `grep -o | wc -l`.

### CU3 — Three Readings, Three Identical Load Pairs, No Combined Form

```
368:  pub fn free_slots( &self ) -> usize
387:  pub fn pending( &self ) -> u64
417:  pub fn may_claim( &self ) -> bool
occurrences of the identical two-load expression: 3
```

Each of the three bodies opens with the same expression —
`self.producer.load( GATING ), self.consumer.load( GATING )` — and then hands the
pair to a different `ring_seqno` function.

**Finding.** A caller that wants two of the three answers issues four loads for
two cursors, and there is no reading that returns more than one answer. That is
correct but not free: the three readings are individually cheap and jointly
redundant, and the type that could offer a combined form is the one that already
holds both cursors.

---

### CU4 — Three Return Widths for Readings of the Same Two Sequences

| Reading | Returns | Delegates to |
|---------|---------|--------------|
| `may_claim` | `bool` | `ring_seqno::may_claim` |
| `free_slots` | `usize` | `ring_seqno::free_slots` |
| `pending` | `u64` | `ring_seqno::pending` |

The widths are `ring_seqno`'s, not this crate's: every one of the three is
forwarded unchanged from the callee's signature.

**Finding.** A caller relating `pending` to `free_slots` — checking that they sum
to the capacity, say — must cast one of them, and neither crate says why the two
differ. `usize` is a count of slots and `u64` is a count of sequence steps, which
is a real distinction; it is documented in neither signature.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [001_the_slowest_fold.md](001_the_slowest_fold.md) | The same delegation discipline applied where the delegated body is a single `.min()` — and where it costs an allocation |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_cursor_pair.md](../data_structure/002_the_cursor_pair.md) | The three fields these readings consult |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_the_capacity_is_held_by_the_pair.md](../decisions/002_the_capacity_is_held_by_the_pair.md) | Why `capacity` is a field two of the three readings reach for |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_one_cursor_one_line.md](../invariant/001_one_cursor_one_line.md) | The layout the two loads are separated by |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_a_reading_that_consults_one_cursor.md](../pitfall/002_a_reading_that_consults_one_cursor.md) | The failure M4 and `the_pair_reads_both_cursors_for_every_reading` exist to catch |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:352-420` | The three readings |
| `ring_seqno/src/lib.rs:50-115` | `laps_between`, `may_claim`, `free_slots`, `pending` — the arithmetic |
| `ring_spsc/src/lib.rs:177` | The single-producer consumer of `CursorPair` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:231-253` | `may_claim_and_free_slots_never_disagree` — the equivalence, over ~300 fixed states |
| `tests/cursor_test.rs:267-284` | `pending_ignores_capacity_and_free_slots_does_not` |
| `tests/cursor_test.rs:300-318` | `the_pair_reads_both_cursors_for_every_reading` |
| `tests/cursor_test.rs:204-230` | The lap boundary, from both sides |
| `tests/manual/readme.md` M4, M5 | Load count and delegation, read from source |
