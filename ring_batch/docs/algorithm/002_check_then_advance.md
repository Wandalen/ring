# Algorithm: Check, Then Advance

### Scope

**Purpose:** Record `claim_gated`'s three steps, why their order is the part
that is tested, and a redundant cast in the middle of them that the family's
lint gate cannot see.

**Responsibility:** `claim_gated`'s body, the `free_slots` it calls, and the
`as usize` between them.

**In Scope:** `ring_batch/src/lib.rs:315-329`;
`ring_seqno/src/lib.rs:95-99`.

**Out of Scope:** What happens when two producers execute these steps at once is
[`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md).
The choice to expose two functions rather than one is
[`pattern/002`](../pattern/002_two_functions_where_one_would_have_hidden_it.md).

---

## Three Steps, in This Order

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A17 -F '  order : Ordering,' ring_batch/src/lib.rs | tail -n 15
command grep -m1 -A4 -F 'pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize' ring_seqno/src/lib.rs
```

Live output:

```
{
  if count > capacity.get()
  {
    return Err( RingError::BatchTooLarge { requested : count, capacity : capacity.get() } );
  }

  let at = producer.load( Ordering::Acquire );
  let behind = consumer.load( Ordering::Acquire );
  if ( free_slots( at, behind, capacity ) as usize ) < count
  {
    return Err( RingError::Full );
  }

  Ok( claim( producer, count, order ) )
}
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
{
  let in_flight = consumer.distance_to( producer );
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
}
```

Step one is a pure function of the arguments. Step two reads both cursors. Step
three is `claim` — the single `fetch_add` of
[`algorithm/001`](001_one_fetch_add_whatever_the_count.md).

---

### BA3 — The Size Check Comes First, and That Is the Behaviour With a Test

Putting `count > capacity.get()` ahead of the cursor reads is not stylistic. It
is what makes an impossible request report `BatchTooLarge` on an *empty* ring
rather than succeeding by accident of there being room:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A15 -F '  assert!( RingError::Full.is_transient(), "and must retry the other" );' ring_batch/tests/batch_test.rs | tail -n 13
```

Live output:

```
#[ test ]
fn an_oversized_request_is_refused_before_the_ring_is_even_consulted()
{
  // Ordering matters: checking capacity first means an impossible request on an
  // empty ring still reports BatchTooLarge rather than succeeding by accident.
  let capacity = cap( 4 );
  let producer = CountingSeq::default();
  let consumer = CountingSeq::default();

  assert!( claim_gated( &producer, &consumer, 5, capacity, Ordering::AcqRel ).is_err() );
  assert_eq!( producer.counts().total, 0, "the cursors were never read" );
  assert_eq!( consumer.counts().total, 0 );
}
```

**Finding.** The test asserts step ordering by counting operations that did not
happen — `counts().total == 0` on both cells. That is a stronger assertion than
checking the error variant, because a body that read the cursors and *then*
checked the size would still return `BatchTooLarge`; only the operation count
distinguishes the two.

It is also the only place in the crate where an ordering decision inside a
function body is pinned by a test rather than by a comment. The two `Acquire`
loads at lines 321–322 are argued for in ten lines of doc comment and asserted
nowhere — see [`decisions/001`](../decisions/001_ordering_is_the_callers_except_where_it_is_not.md).

---

### BA4 — A Redundant Cast the Lint Gate Structurally Cannot See

Line 323 reads `( free_slots( at, behind, capacity ) as usize ) < count`.
`free_slots` returns `usize`. The cast is `usize as usize`.

`clippy::unnecessary_cast` is warn-by-default and exists for exactly this. The
workspace runs clippy under `-D warnings` and the crate is clean. Both are true
at once, because the lint does not fire across a crate boundary:

```
--- same-crate: three variants, all warn ---
warning: casting to the same type is unnecessary (`usize` -> `usize`)
  --> ./-cast.rs:11:6
warning: casting to the same type is unnecessary (`usize` -> `usize`)
  --> ./-cast.rs:12:6
warning: casting to the same type is unnecessary (`usize` -> `usize`)
  --> ./-cast.rs:13:6
warning: 3 warnings emitted

--- cross-crate: the identical cast, function moved to a dependency ---
  clippy exit: 0
  binary: ./-user 4387928 bytes
room
```

The three same-crate variants differ only in the callee's shape — no arguments,
arguments, and a `saturating_sub` body identical to `free_slots`' own. All three
warn. Move the function into a separate crate and call it through `--extern`,
change nothing else, and the warning disappears while the build still succeeds.

**Finding.** The cast is harmless — it compiles to nothing and cannot change the
result. What it records is a limit on what "clippy is clean under `-D warnings`"
proves in a 33-crate workspace where nearly every call crosses a crate boundary.
This family's lint gate is blind to same-type casts on every one of those calls,
and this is the one instance of the pattern that survived to be found by reading.

The cast is also evidence of its own kind: someone wrote `as usize` because they
expected `free_slots` to return something else. It returns `usize`, and has
since it was written — so the cast is a fossil of an interface that was never
shipped, preserved because nothing in the toolchain could point at it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md) | What two producers do to these three steps |
| [`decisions/001`](../decisions/001_ordering_is_the_callers_except_where_it_is_not.md) | Why the two loads are `Acquire` and the advance is not |
| [`decisions/002`](../decisions/002_two_errors_not_one.md) | Why step one and step two return different errors |
| [`algorithm/001`](001_one_fetch_add_whatever_the_count.md) | Step three, in full |

### Sources

| Fact | Where |
|------|-------|
| `claim_gated`'s body | `ring_batch/src/lib.rs:315-329` |
| `free_slots`' signature and body | `ring_seqno/src/lib.rs:95-99` |
| The step-ordering test | `ring_batch/tests/batch_test.rs:256-268` |
| The clippy blind spot | `clippy-driver` probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `an_oversized_request_is_refused_before_the_ring_is_even_consulted` | Step one runs before step two, by operation count |
| `a_full_ring_refuses_with_full_and_advances_nothing` | Step two's refusal leaves the cursor alone |
| `a_consumer_advancing_reopens_the_gate` | Step two re-reads rather than caching |
| *(to create)* | That the two gating loads are `Acquire`, which no test observes |
