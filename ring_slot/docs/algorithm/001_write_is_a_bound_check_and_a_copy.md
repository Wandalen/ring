# Algorithm: Write Is a Bound Check and a Copy

### Scope

**Purpose:** Record the three steps of `BytesSlot::write`, the ordering that
makes a rejected write leave the slot untouched, and the fact that the whole
crate contains no loop.

**Responsibility:** The step sequence of the crate's only non-trivial function.

**In Scope:** `ring_slot/src/lib.rs:347-356`; the absence of any loop,
retry, or branch elsewhere in the crate.

**Out of Scope:** Why the error variant is `BatchTooLarge` rather than something
slot-shaped — that is
[`workaround/002`](../workaround/002_batchtoolarge_borrowed_for_a_different_shape.md).
The emptiness computations are
[`algorithm/002`](002_emptiness_three_ways.md).

---

## The Three Steps

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A9 -F '  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >' ring_slot/src/lib.rs
```

Live output:

```
  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >
  {
    if payload.len() > N
    {
      return Err( RingError::BatchTooLarge { requested : payload.len(), capacity : N } );
    }
    self.bytes[ ..payload.len() ].copy_from_slice( payload );
    self.len = payload.len();
    Ok( () )
  }
```

| Step | Statement | Why it is where it is |
|------|-----------|-----------------------|
| 1 | `if payload.len() > N` → return | Must precede any mutation, or a rejected write half-updates the slot |
| 2 | `copy_from_slice` into `[ ..payload.len() ]` | Copies exactly the payload; the tail is untouched by construction |
| 3 | `self.len = payload.len()` | Publishes the new length; until this line executes, `read()` still reports the old payload |

Three straight-line statements, one early return, no loop.

---

### SL13 — The Rejection Precedes Every Mutation, Which Is What Makes Failure Total

The ordering is the whole of the atomicity argument. Step 1 returns before step 2
runs, so a rejected write performs no copy and no length update. There is no
partial state to unwind because none is ever entered.

The suite asserts this from both starting states:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A23 -F '/// A failed write leaves the previous contents intact — the slot is not' ring_slot/tests/slot_test.rs
```

Live output:

```
/// A failed write leaves the previous contents intact — the slot is not
/// half-updated, so a caller that handles the error still has valid data.
#[ test ]
fn a_failed_write_leaves_the_previous_contents_intact()
{
  let mut slot = BytesSlot::< 4 >::empty();
  slot.write( b"abcd" ).unwrap();
  assert!( slot.write( b"abcde" ).is_err() );

  assert_eq!( slot.read(), b"abcd" );
  assert_eq!( slot.len(), 4 );
  assert!( !slot.is_empty() );
}

/// A failed write onto an empty slot leaves it empty rather than partially
/// filled.
#[ test ]
fn a_failed_write_onto_an_empty_slot_leaves_it_empty()
{
  let mut slot = BytesSlot::< 2 >::empty();
  assert!( slot.write( b"too long" ).is_err() );
  assert!( slot.is_empty() );
  assert_eq!( slot.read(), b"" );
}
```

**Finding.** Both directions are covered — a failure over an occupied slot and a
failure over an empty one — and they are separate tests rather than two
assertions in one, which is what keeps the second from being an afterthought.
The property is cheap here only because the check comes first; the same three
steps in the order copy-check-publish would need a rollback, and the order
check-publish-copy would expose a slot whose length exceeds its written bytes.

The step-3-last ordering also means `read()` is coherent at every point: before
step 3 it returns the previous payload, after it the new one, and never a mixture
— which matters because `&mut self` is exclusive but a panic inside
`copy_from_slice` would leave the slot readable by anything that catches it.

---

### SL14 — The Crate Contains No Loop, No Retry, and No Branch Outside `write`

```sh
cd "$(git rev-parse --show-toplevel)"
# every statement-position control-flow construct in the crate; comments are
# dropped first, so a doctest line can never satisfy the unanchored alternatives
echo '  -- every statement-position control-flow construct in the crate --'
command grep -vE '^[[:space:]]*///?' ring_slot/src/lib.rs \
  | command grep -oE '^[[:space:]]*(if|match|while|loop|for) [^{]*|[[:space:]]loop$|\bwhile [a-z]' \
  | sed 's/^/  /' || echo '    none'

# and, independently: any loop at all, doc comments stripped. `grep -c` exits 1
# on a count of zero, and zero is the answer this instance is about, so the
# count is captured rather than run as the block's last command
echo '  -- and, independently: any loop at all, doc comments stripped --'
printf '    loop/while/for-in outside comments: %s\n' "$( command grep -vE '^[[:space:]]*//' ring_slot/src/lib.rs | command grep -cE '\bloop\b|\bwhile\b|\bfor [a-z_]+ in\b' || true )"
```

Live output:

```
  -- every statement-position control-flow construct in the crate --
      if payload.len() > N
  -- and, independently: any loop at all, doc comments stripped --
    loop/while/for-in outside comments: 0
```

One `if`, in `write`, and nothing else. Every other function in the crate is a
single expression: a field read, an `Option` method call, a slice range, or a
struct literal.

Both halves of the command earn their place. A naive `\bfor\b` would report five
hits, all of them `impl ... for ...` headers rather than loops; and the second
command's comment filter is what keeps a dozen doctest `assert!` lines out of the
count.

**Finding.** `ring_slot` has a cyclomatic complexity of one everywhere except
`write`, where it is two. There is no state machine, no retry, no ordering
concern, and no concurrency — a `Slot` is a plain value and every operation on it
terminates in a bounded number of instructions with no data-dependent branching
beyond the single bound check.

That is worth recording because it locates the crate precisely in the family: the
concurrency lives entirely in the crates that *hold* slots (`ring_cursor`,
`ring_claim`, `ring_publish`, `ring_consume`), and `ring_slot` contributes the
payload container and nothing else. A reader looking for where a ring's
correctness argument lives will not find any of it here, and the absence is the
design working rather than a gap.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](002_emptiness_three_ways.md) | The other computations — three ways to ask whether a slot holds anything |
| [`invariant/001`](../invariant/001_a_read_returns_what_was_written.md) | The property steps 2 and 3 jointly uphold |
| [`workaround/002`](../workaround/002_batchtoolarge_borrowed_for_a_different_shape.md) | Step 1's error variant, and why it is named for batches |
| [`lifecycle/001`](../lifecycle/001_a_slot_across_one_publish.md) | Where these three steps sit in a publish |

### Sources

| Fact | Where |
|------|-------|
| The three steps | `ring_slot/src/lib.rs:347-356` |
| Both failure directions | `ring_slot/tests/slot_test.rs:182-205` |
| The single branch | `ring_slot/src/lib.rs:349` |

### Tests

| Test | Covers |
|------|--------|
| `a_failed_write_leaves_the_previous_contents_intact` | Step 1 preceding step 2, over an occupied slot |
| `a_failed_write_onto_an_empty_slot_leaves_it_empty` | The same, over an empty one |
| `an_oversized_write_is_refused_with_both_numbers` | Step 1's error carrying both figures |
| `a_write_of_exactly_capacity_is_accepted` | The `>` rather than `>=` at the boundary |
