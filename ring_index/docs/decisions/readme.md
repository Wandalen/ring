# decisions

A crate this small makes two choices worth arguing about. One was made upstream
and shapes everything here: capacity must be a power of two, so the fold is a
mask and every function is total. The other was made here and shapes nothing,
because the function it applies to has no callers: `run` returns a `Vec`.

Both are recorded with what they foreclose rather than only what they enable,
because in each case the foreclosure is the part that went unwritten. The
power-of-two decision is justified by a cycle count and not by the totality it
actually buys; the `Vec` decision is justified by nothing at all.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_power_of_two_or_nothing.md) | A Power of Two, or Nothing | The constraint, its single enforcement site, the missing rounding affordance, and the benefits nobody argued |
| [002](002_a_vec_where_an_iterator_would_do.md) | A `Vec` Where an Iterator Would Do | `run`'s return type against the lazy version another crate shipped |

## The Constraint Is Cheap to State and Expensive to Satisfy

`Capacity::new` rejects any non-power-of-two with a named error. That is the
whole enforcement, and it is correct. What the family never provides is the
other half — a way to turn a number a caller actually has into a capacity the
type will accept. `usize::next_power_of_two` is one std call away and appears
nowhere in 33 crates.

So the constraint's cost lands on callers, at runtime, at the outermost API,
with the choice between rounding up (more memory than asked for) and rounding
down (less capacity than asked for) left entirely to them and undiscussed.

## Both Justifications Argue the Wrong Half

The power-of-two comment argues cycles. The real case is structural: totality,
no error paths, a definable aliasing relation, a `mask()` that needs no
validation. If the cycle number were withdrawn — and it is off by roughly 3–6×
on this host — the written case would vanish and the actual case would not
move.

`run`'s comment argues about its inputs, explaining that batch claims are
contiguous. The only decision in the function is what it returns, and about that
there is no sentence anywhere.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'is_power_of_two\|next_power_of_two' ring_*/src/*.rs
command grep -m1 -A12 -F '  pub const fn new( slots : usize ) -> Result< Self, RingError >' ring_types/src/capacity.rs
command grep -m1 -A16 -F '/// A batch claim is contiguous in sequence space, so its slots wrap at most' ring_index/src/lib.rs
command grep -m1 -A4 -F 'pub fn drain_order( claim : &BatchClaim, capacity : Capacity )' ring_batch/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| IX25 | `ring_types` | n/a — observation | The constraint is enforced by rejection at one site and assisted nowhere; `next_power_of_two` appears in no crate, so rounding is every caller's problem |
| IX26 | `ring_index` | n/a — doc gap | The stated justification is the cycle count; totality, the absence of error paths, and the aliasing relation all follow from the same decision and go unmentioned |
| IX27 | `ring_index` | n/a — doc gap | `run`'s only real decision is its return type, and its doc comment discusses its inputs instead |
| IX28 | `ring_index` | n/a — observation | Random access and known length are both available without the `Vec`; the lazy signature is strictly more general, and changing it would break nothing outside this crate |
