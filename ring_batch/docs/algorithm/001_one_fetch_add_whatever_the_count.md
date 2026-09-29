# Algorithm: One `fetch_add`, Whatever the Count

### Scope

**Purpose:** Record the whole of `claim` — a single `fetch_add` whose return
value *is* the claim's start — and measure the amortisation this crate's own
contract asks it to deliver.

**Responsibility:** `claim`'s body, the `SeqCell::fetch_add` it delegates to,
and the cost curve across batch sizes.

**In Scope:** `ring_batch/src/lib.rs:221-224`;
`ring_atomic/src/lib.rs:223-226`.

**Out of Scope:** The gated form is
[`algorithm/002`](002_check_then_advance.md). What the amortisation is worth
against the crate's stated target is
[`non_functional_requirement/001`](../non_functional_requirement/001_what_one_batch_actually_buys.md).

---

## The Whole of It

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F 'pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim' ring_batch/src/lib.rs
command grep -m1 -A6 -F '    self.0.store( value.0, order );' ring_atomic/src/lib.rs | tail -n 4
```

Live output:

```
pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim
{
  BatchClaim::new( cursor.fetch_add( count as u64, order ), count )
}
  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
  {
    Seq( self.0.fetch_add( n, order ) )
  }
```

One line. There is no loop, no compare-and-swap, no retry, and no branch — the
entire batch-claim protocol is a single read-modify-write.

---

### BA1 — The Amortisation Is `fetch_add` Returning the Old Value

`fetch_add` is not just "add atomically". It returns what the cell held *before*
the addition, and that return value is exactly the first sequence the caller
owns. So one instruction does two jobs at once: it publishes the caller's claim
on `[old, old + count)` and it advances the cursor past that range so nobody
else can be handed a sequence inside it.

**Finding.** This is the whole reason the crate needs no protocol. There is no
window in which a claim exists but the cursor has not moved, because the two are
the same operation. `BatchClaim::new( start, count )` that wraps it holds no
lock, no token, and no reference to the cursor — the ownership the doc comment
calls "by construction rather than by enforcement" is construction in the
literal sense: the constructor's first argument *is* the atomic's return value.

That also fixes what the crate cannot do. Nothing can give a claim back. The
cursor advanced; there is no `unclaim`, and a caller that decides mid-flight it
does not want the range must still write something into it or leave the
consumer waiting forever — see
[`lifecycle/001`](../lifecycle/001_no_lifecycle_and_no_rollback.md).

---

### BA2 — One Claim Costs the Same at Every Size, and the Stated Claim Understates It

This crate's own module comment states: "the memory fences that make the handshake
correct are paid per operation, not per item, so a batch of sixty-four costs
roughly what a single item costs." Measured on this host, release build, one
uncontended thread, two million iterations per point:

```
--- what one batch buys, uncontended, one thread ---
     1 slots : one claim    6.57 ns   |      1 claims of one     11.54 ns   |   ratio    1.8x
     8 slots : one claim    6.43 ns   |      8 claims of one     87.87 ns   |   ratio   13.7x
    64 slots : one claim    6.42 ns   |     64 claims of one    736.68 ns   |   ratio  114.7x
  1024 slots : one claim    6.41 ns   |   1024 claims of one  11810.39 ns   |   ratio 1841.1x
```

**Finding.** The left column is flat to within 0.16 ns across three orders of
magnitude — a claim of 1024 costs no more than a claim of 1. "Roughly what a
single item costs" is not an approximation here; it is an equality, and the
`count as u64` that carries the size never touches the cost.

The word "roughly" hides the more useful number, which is the right column. At
64 the batch is 115× cheaper than the items it replaces; at 1024 it is 1841×.
This crate's own contract states the property that is true and omits the ratio that makes the
property worth having, which is the number needed to trade batch
size against tail latency.

The 1-slot row is the honest control: 1.8×, not 1.0×, because the "N claims of
one" loop pays its own loop overhead. That is the floor the measurement cannot
go below, and it is what the other three rows should be read against.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](002_check_then_advance.md) | The gated form, and the window the gate opens |
| [`non_functional_requirement/001`](../non_functional_requirement/001_what_one_batch_actually_buys.md) | The same curve, read against what the family promised to measure |
| [`lifecycle/001`](../lifecycle/001_no_lifecycle_and_no_rollback.md) | Why a claim cannot be returned |
| [`invariant/001`](../invariant/001_disjointness_is_free.md) | The disjointness this one instruction buys |

### Sources

| Fact | Where |
|------|-------|
| `claim`'s body | `ring_batch/src/lib.rs:221-224` |
| `fetch_add`'s implementation | `ring_atomic/src/lib.rs:223-226` |
| "roughly what a single item costs" | `ring_batch/src/lib.rs:7-10` |
| The cost curve | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_claim_of_sixty_four_issues_one_operation_not_sixty_four` | One `fetch_add`, counted against `CountingSeq` |
| `the_cost_of_a_claim_does_not_depend_on_its_size` | The same, at 0, 1, 2, 7, 64 and 1024 |
| `the_sequences_returned_are_contiguous` | That the returned range has no gap |
| *(to create)* | The cost *ratio*, the number the batch-size/tail-latency tradeoff needs |
