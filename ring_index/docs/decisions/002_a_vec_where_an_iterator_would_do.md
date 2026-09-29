# Decisions: A `Vec` Where an Iterator Would Do

### Scope

**Purpose:** Record `run`'s return type as a decision, price it against the
alternative that another crate went on to build, and note that the decision has
no recorded rationale anywhere.

**Responsibility:** `run`'s signature — why `Vec< SlotIndex >` rather than
`impl Iterator`, and what each choice forecloses.

**In Scope:** `ring_index/src/lib.rs:86-122`;
`ring_batch/src/lib.rs:358-362`.

**Out of Scope:** the measured allocation is
[`non_functional_requirement/002`](../non_functional_requirement/002_the_one_allocation_and_the_zero_callers.md).
`ring_batch`'s version read as a workaround is
[`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md).

---

## The Two Signatures That Exist

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F 'pub fn run( start : Seq, count : usize, capacity : Capacity ) -> Vec< SlotIndex >' ring_index/src/lib.rs
command grep -m1 -A4 -F 'pub fn drain_order( claim : &BatchClaim, capacity : Capacity )' ring_batch/src/lib.rs
```

Live output:

```
pub fn run( start : Seq, count : usize, capacity : Capacity ) -> Vec< SlotIndex >
{
  ( 0..count as u64 ).map( | n | of( start.advanced_by( n ), capacity ) ).collect()
}
pub fn drain_order( claim : &BatchClaim, capacity : Capacity )
-> impl Iterator< Item = ( Seq, SlotIndex ) > + use< >
{
  claim.sequences().map( move | seq | ( seq, of( seq, capacity ) ) )
}
```

The bodies are the same expression. One ends in `.collect()`; the other returns
the `Map` directly. Everything that differs between the two functions follows
from that one call.

---

### IX27 — The `Vec` Is `run`'s Only Real Decision, and It Was Made Silently

**Finding.** Strip the return type and `run` has nothing left to decide: the
arithmetic is `of`, the iteration order is the natural one, the count is a
parameter. The choice to `collect` is the function.

`run`'s doc comment does not mention it. Three lines of comment explain that a
batch claim is contiguous and therefore wraps at most once, citing
this family's own batch-claim contract — an argument about the
*inputs* the function expects, with nothing about the *output* it produces. No
comment anywhere in the crate says why the slots are materialized rather than
yielded.

That absence is the finding. A `Vec` return in a lock-free substrate crate is a
choice that wants a sentence — even "callers need to index into the result" or
"the borrow checker made the lazy version awkward" would settle it. What exists
instead is a justification for the wrong half of the signature.

---

### IX28 — Every Property the `Vec` Was Presumably Chosen For Is Available Without It

Two things a `Vec` gives you that a bare `impl Iterator` does not: random access,
and a length known before iteration. Neither survives contact with the actual
use case.

**Finding.** A caller who wants the `k`th slot of a run does not need `run` at
all — that is `of( start.advanced_by( k ), capacity )`, one call, no allocation.
A caller who wants the length already has it: `count` is an argument they
passed. The `Vec` re-supplies two facts the caller either does not need or
already holds.

Meanwhile the `Vec` forecloses the one property that matters in this family:
being callable from a path that must not allocate
([`non_functional_requirement/002`](../non_functional_requirement/002_the_one_allocation_and_the_zero_callers.md)
IX23). And a caller who genuinely wants a `Vec` can get one from the iterator
version with `.collect()` at the call site, while a caller who wants the
iterator cannot get one back out of the `Vec` without having already paid for
it. The lazy signature is strictly more general.

`ring_batch` demonstrates this concretely rather than hypothetically:
`drain_order` returns the lazy form, is the only range-folding call site in the
family, allocates nothing, and pairs each slot with its sequence — which the
`Vec` version cannot do without allocating twice as much or losing the pairing.

The decision is recorded here as reversible and unblocked. `run` has no callers
([`api/001`](../api/001_three_functions_three_must_use_one_reached.md) IX6), so
changing its return type would break nothing outside this crate's own four
tests, three of which would need only a `.collect()` added.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](../algorithm/002_a_run_is_the_fold_applied_count_times.md) | The body this return type wraps |
| [`non_functional_requirement/002`](../non_functional_requirement/002_the_one_allocation_and_the_zero_callers.md) | What the `collect` costs, measured |
| [`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md) | The lazy version, shipped in another crate |
| [`api/002`](../api/002_the_three_signatures_and_the_const_they_are_not.md) | `run` as the one function with a real reason not to be `const` |

### Sources

| Fact | Where |
|------|-------|
| `run`'s signature and body | `ring_index/src/lib.rs:119-122` |
| `drain_order`'s signature and body | `ring_batch/src/lib.rs:358-362` |
| `run`'s doc comment, arguing about inputs | `ring_index/src/lib.rs:86-88` |
| Zero callers | `api/001` IX6 census |

### Tests

| Test | Covers |
|------|--------|
| `a_full_capacity_run_covers_every_slot_once` | The output as a sequence, which an iterator would satisfy identically |
| `an_empty_run_is_empty` | `count = 0`, which distinguishes the two return types not at all |
| *(to create)* | Nothing — every existing `run` test would pass unchanged against an iterator plus `.collect()` |
