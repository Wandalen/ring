# Seq::next

## Representation

Returns the sequence one past this one. **Two production call sites in the whole
family**, against fourteen for [`advanced_by`](008_seq_advanced_by.md), which
does the same job with a parameter.

That ratio is the item's main fact. `next()` is the obvious operation on a
counter and the family almost never wants it, because the family almost never
advances by one — it claims ranges, drains batches, and computes ends, all of
which take a count. The two survivors are the two places where the step really is
singular: an inner loop walking a contiguous run, and a destructor publishing a
final position.

**The doc comment on this function used to be wrong about release-build
overflow**, and the error is recorded in full at
[`../../pitfall/001`](../../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md).
It used to say `u64` addition saturates; it wraps, and the comment has since
been corrected to say so. Not re-argued here.

## Kind

Associated Function/Method (§ Item Kind Taxonomy : Associated Item Kinds #1)

## Definition

`ring_types/src/id.rs:45`

```rust
#[ must_use ]
pub const fn next( self ) -> Self
```

Body is `Self( self.0 + 1 )` (`id.rs:47`) — plain `+`, not `checked_add`, not
`saturating_add`, not `wrapping_add`. The choice is deliberate and stated in the
doc comment, which now correctly describes what `+` does.

**`self` by value, not `&mut self`.** `Seq` is `Copy`, so the natural
imperative form — `seq.advance()` mutating in place — is absent by design.
Every caller writes `end = end.next()`, which makes the reassignment visible at
the call site. `ring_mpsc:534` is exactly that line.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/id.rs` | 22, 32-38, 40-43, 44-45, 47 | Doc example on the struct itself (22); the doc comment, including the overflow paragraph, now corrected (32-38); doc example `Seq( 41 ).next() == Seq( 42 )` (40-43); `#[ must_use ]` and **the definition (44-45)**; the body (47) |

Test-only references: `ring_types` — 2 in `tests/types_test.rs`.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/id.rs` | Defining crate |
| `ring_mpsc` | `src/lib.rs` | `contiguous_end` walks a run one publication at a time: `end = end.next();` (`:534`) |
| `ring_spsc` | `src/lib.rs` | `Batch::drop` stores the producer cursor one past the last consumed slot (`:725`) |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r '\.next()' ring_*/src | command grep -v '^ring_types/' \
  | command grep -v ':[ \t]*//' | sort
```

Live output:

```
ring_mpsc/src/lib.rs:      end = end.next();
ring_spsc/src/lib.rs:    self.ring.cursors.producer().store( self.seq.next(), HANDOFF );
ring_tls/src/lib.rs:    let item = self.items.next()?;
```

Three lines, in this order.

The third is not this function: the `ring_tls/src/lib.rs` hit, `self.items.next()?`,
is `Iterator::next` — a different trait method on a different type that happens to
share a name. Disambiguating it needs the receiver, not the method, which is the
same limitation the `.get()` census runs into
(→ [`002_capacity_get.md`](002_capacity_get.md)).

**The `sort` is load-bearing, not tidiness.** Without it `grep -r` returns the
three matches in filesystem traversal order, which is not stable across runs —
three consecutive runs of the unsorted form here put `ring_tls` first, first,
then second. An earlier revision published "the third is not this function"
against that unordered output: true on the run it was written from, false on most
others, and nothing in the output reveals which run you got. A recipe whose
published claim is positional must sort, or name the line instead of its
position.

**The two real call sites are in the two ring implementations and nowhere else.**
No policy crate, no gating crate, no config crate ever steps a sequence by one.

## Caller Tree

- *No caller within `ring_types`*
- *External: `ring_mpsc::…::contiguous_end`* (`ring_mpsc/src/lib.rs:534`) — inside a loop extending a contiguous published run
- *External: `ring_spsc::Reservation::drop`* (`ring_spsc/src/lib.rs:794`) — `self.ring.cursors.producer().store( self.seq.next(), HANDOFF )`

**`ring_spsc`'s caller is a `Drop` impl**, which is worth noting because it puts
this function on a path that runs during unwinding. The body cannot panic in a
release build (it wraps) and can panic in a debug build (overflow check) — so in
debug, a `Seq` at `u64::MAX` inside a destructor during an unwind would abort.
The wrap point is unreachable, so this is a curiosity rather than a defect; it is
recorded because "unreachable" is the entire argument, and the argument lives in
a doc comment rather than a check.

## Callee Tree

- *(none)* — `u64::add`, which is a primitive operation rather than a call

**One increment, two callers, and a doc comment that used to describe a third
behaviour the function does not have.** The correction was one word — *wraps* for
*saturates* — applied here alongside the matching fix to
[`advanced_by`](008_seq_advanced_by.md), whose doc comment used to omit
overflow entirely rather than describing it wrongly and now states it
(→ [`../../decisions/readme.md`](../../decisions/readme.md) § Not decisions).
