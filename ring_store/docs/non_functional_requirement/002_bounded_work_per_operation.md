# Non-Functional Requirement: Bounded Work per Operation

### Scope

**Purpose:** Record that the four addressers do constant work while the two
sweeps do `capacity` work — measured at exactly 1024 calls each — that neither
sweep has a library caller, and that `all_empty` short-circuits so its worst case
is the case it is asked about.

**Responsibility:** The per-operation cost NFR: which operations are bounded by
what, and what the data-dependence costs a caller polling for quiescence.

**In Scope:** `ring_store/src/lib.rs:98-141, 203-257`.

**Out of Scope:** The mask that makes addressing constant is
[`algorithm/001`](../algorithm/001_one_mask_no_modulo.md). Allocation is
[`non_functional_requirement/001`](001_allocate_once_then_never_again.md).

---

### BF44 — Two Operations Cost `capacity`, and Neither Is on a Hot Path

The two sweeps, side by side:

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^  pub fn clear\( &mut self \)$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 6 { print } /^  \/\/\/ assert!\( buffer\.all_empty\(\) \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 6 { print }' ring_store/src/lib.rs
```

Live output:

```
  pub fn clear( &mut self )
  {
    for slot in &mut self.slots
    {
      slot.clear();
    }
  }
  #[ must_use ]
  pub fn all_empty( &self ) -> bool
  {
    self.slots.iter().all( Slot::is_empty )
  }
```

Counted with a `Slot` implementation that tallies what the buffer asks of it:

```
--- (3) exactly capacity calls per sweep, and a short-circuit ---
  clear() at capacity 1024:            1024 Slot::clear calls
  all_empty() with every slot empty:   1024 Slot::is_empty calls (= true)
```

**Finding.** The cost split is clean and it lines up with use. `get`, `get_mut`,
`at` and `at_mut` are a mask and a slice index — constant work, no branch on
capacity, no loop. `clear` and `all_empty` are `capacity`-bounded, exactly, with
no slack: 1024 calls at capacity 1024, one per slot.

What makes the split matter is who calls what. Both `capacity`-bounded operations
have no library caller anywhere in the family —
[`integration/001`](../integration/001_four_dependents_two_that_build_on_it.md)
BF3 for `all_empty`, [`lifecycle/002`](../lifecycle/002_the_sweep_nothing_calls.md)
BF32 for `clear`. The three functions consumers actually call are `new`, `at` and
`at_mut`, so every operation on a ring's per-message path is constant-time by
construction rather than by discipline: the linear operations exist, are correct,
and are not reachable from the hot path because nothing on the hot path calls
them.

---

### BF45 — `all_empty` Is Cheapest When the Answer Is No, and Nobody Says So

The same counter, with one slot occupied:

```
  all_empty() with slot 0 occupied:    1 Slot::is_empty calls (= false)
```

**Finding.** `Iterator::all` short-circuits, so `all_empty`'s cost is entirely
data-dependent and inverted relative to intuition: finding an occupied slot is
`O(1)` in the best case, and confirming the buffer is fully empty is always the
full `capacity`. The expensive path is the one that returns `true`.

That matters for the use this predicate was built for. A shutdown or drain check
polling `all_empty()` in a loop pays one comparison per iteration while work
remains — cheap, and it is polling — and then pays a full `capacity` scan on the
single iteration where it finally succeeds. That is the right shape and costs
nothing worth avoiding at these capacities, but it is the opposite of what a
reader will assume from the name, and the doc says only *whether every slot is
empty*.

The second-order consequence is worth stating too: the answer is a snapshot with
no synchronization. `all_empty` takes `&self`, walks the slice, and returns —
between the slot it checks first and the slot it checks last, a producer holding
the ring through an `UnsafeCell` may have written to either. So the `true` it
returns means "every slot was empty at the moment it was read", which for a
single-threaded shutdown sequence is exactly right and for a concurrent liveness
check is not a guarantee at all. Neither reading is documented, and the predicate
has no caller to settle which one was intended.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/001`](001_allocate_once_then_never_again.md) | The other NFR, on memory rather than time |
| [`algorithm/001`](../algorithm/001_one_mask_no_modulo.md) | Why addressing is constant |
| [`integration/001`](../integration/001_four_dependents_two_that_build_on_it.md) | That `all_empty` has no caller |
| [`lifecycle/002`](../lifecycle/002_the_sweep_nothing_calls.md) | That `clear` has no caller either |
| [`item/001`](../item/001_the_slot_as_the_buffer_sees_it.md) | The two trait calls these two sweeps make |

### Sources

| Fact | Where |
|------|-------|
| Both sweep bodies | `ring_store/src/lib.rs:120-125, 139-141` |
| The four constant-time addressers | `ring_store/src/lib.rs:203-257` |
| Exactly `capacity` calls per sweep | Release probe with a counting `Slot`, quoted above |
| The short-circuit at one call | Same probe |
| Neither sweep having a library caller | Censuses in `integration/001` and `lifecycle/002` |

### Tests

| Test | Covers |
|------|--------|
| `clear_empties_every_slot_and_keeps_the_allocation` | That `clear` reaches every slot |
| `every_slot_starts_empty` | `all_empty` on its expensive path |
| `storage_survives_being_addressed_out_of_order` | Constant-time addressing under a scrambled order |
| *(to create)* | A counting `Slot` asserting `clear` makes exactly `capacity` calls |
| *(to create)* | The same counter asserting `all_empty` stops at the first occupied slot |
