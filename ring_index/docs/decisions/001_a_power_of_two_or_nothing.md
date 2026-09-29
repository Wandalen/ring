# Decisions: A Power of Two, or Nothing

### Scope

**Purpose:** Record the decision that makes this crate's fold a mask — capacity
constrained to a power of two — what it costs a caller, and that the family
enforces the constraint with an error and offers no way to satisfy it.

**Responsibility:** The power-of-two constraint as a decision: what it bought,
what it forecloses, and where the cost lands.

**In Scope:** `ring_types/src/capacity.rs:40-52, 75-78`;
`ring_config/src/lib.rs:65-77`; every `is_power_of_two` in the family.

**Out of Scope:** the measured value of the constraint is
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_fold_costs.md).
The fold itself is [`algorithm/001`](../algorithm/001_one_and_of_a_mask.md).

---

## The Decision, and Where It Is Enforced

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -r 'is_power_of_two\|next_power_of_two' ring_*/src/*.rs
```

Live output:

```
ring_align/src/lib.rs:  CACHE_LINE.is_power_of_two(),
ring_types/src/capacity.rs:    if !slots.is_power_of_two()
```

One occurrence in 33 crates. The constraint is decided once, enforced once, and
never assisted.

---

### IX25 — The Constraint Is Enforced by Rejection and Satisfied by Nobody's Help

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A12 -F '  pub const fn new( slots : usize ) -> Result< Self, RingError >' ring_types/src/capacity.rs
echo '  -- and the config that forwards to it --'
command grep -m1 -A7 -F '  pub fn new( slots : usize ) -> Result< Self, RingError >' ring_config/src/lib.rs
```

Live output:

```
  pub const fn new( slots : usize ) -> Result< Self, RingError >
  {
    if slots == 0
    {
      return Err( RingError::CapacityZero );
    }
    if !slots.is_power_of_two()
    {
      return Err( RingError::CapacityNotPowerOfTwo( slots ) );
    }
    Ok( Self( slots ) )
  }

  -- and the config that forwards to it --
  pub fn new( slots : usize ) -> Result< Self, RingError >
  {
    Ok
    (
      Self
      {
        capacity : Capacity::new( slots )?,
        wait : WaitKind::default(),
```

**Finding.** A caller who wants a ring holding about a thousand items writes
`Capacity::new( 1000 )` and gets `CapacityNotPowerOfTwo( 1000 )`. The error is
clear, names the offending value, and tells them nothing about what to do next.

`usize::next_power_of_two` is in std and would make the affordance one line:
a `Capacity::at_least( slots )` that rounds up, or a documented "round it
yourself" note on `new`. The family has neither. `RingConfig::new` propagates
the same error with `?` and adds nothing of its own — so the constraint surfaces
at the outermost API a user touches, as a runtime error, with the rounding left
as an exercise.

The cost of the decision therefore lands entirely on callers, and lands late. A
capacity read from a config file — the obvious source for a tunable like this —
fails at construction with no guidance, and the correct fix depends on which
direction the caller wants to be wrong in: `1024` costs 2.4% more memory than
asked for, `512` gives up 49% of the requested capacity, and only the caller
knows which is acceptable.

---

### IX26 — The Decision Buys More Than the Mask, and Only the Mask Is Argued For

The module comment justifies the constraint entirely by the cost of division:
the fold is a bitmask "rather than a division." That is the smaller half of what
a power-of-two capacity provides.

**Finding.** The larger half is that `x & (n-1)` distributes a monotonically
increasing sequence over slots *uniformly and in strict rotation*, so slot `k`
is revisited exactly every `n` sequences with no clustering, ever. A modulo by a
non-power-of-two would give the same guarantee, so this is not a difference
between mask and division — it is the property that lets
[`invariant/001`](../invariant/001_the_mask_equals_the_modulo.md) hold at all
and lets `aliases` be defined as "a whole number of laps apart."

The constraint also makes `mask()` a total function returning a plain `usize`
with no validation of its own, and makes `of` total, and makes both `const`-able
— the whole totality argument in
[`algorithm/001`](../algorithm/001_one_and_of_a_mask.md) IX1 is downstream of
this one decision.

What is recorded here is that none of that appears in the justification. The
comment argues the cheapest benefit (cycles, with a number nobody measured) and
omits the structural ones (totality, no error paths, a definable aliasing
relation). If the cycle figure were ever challenged — and
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_fold_costs.md)
IX21 challenges it — the written case for the constraint would collapse, while
the actual case would be untouched.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/001`](../algorithm/001_one_and_of_a_mask.md) | The totality this decision buys |
| [`invariant/001`](../invariant/001_the_mask_equals_the_modulo.md) | The identity that holds only under this constraint |
| [`non_functional_requirement/001`](../non_functional_requirement/001_what_the_fold_costs.md) | The measured value of the benefit actually argued for |
| [`workaround/001`](../workaround/001_the_mask_that_lives_one_crate_up.md) | Where the constraint's enforcement was placed, and what that costs to read |

### Sources

| Fact | Where |
|------|-------|
| The single enforcement site | Census above |
| `Capacity::new`'s rejection | `ring_types/src/capacity.rs:40-52` |
| `RingConfig::new` forwarding it | `ring_config/src/lib.rs:65-72` |
| The stated justification | `ring_index/src/lib.rs:10-13` |

### Tests

| Test | Covers |
|------|--------|
| `non_power_of_two_capacity_is_rejected_upstream` | The rejection, asserted from this crate |
| `capacity_one_maps_everything_to_slot_zero` | The smallest legal capacity, which is a power of two |
| *(to create)* | Nothing — the open half of this decision is a missing affordance, not a missing assertion |
