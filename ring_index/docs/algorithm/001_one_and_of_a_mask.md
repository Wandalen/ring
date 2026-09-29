# Algorithm: One `and` of a Mask

### Scope

**Purpose:** Record the whole computation this crate performs — a single
bitwise `and` against `capacity - 1` — the reason it is total rather than
fallible, and what it measures against the `%` it replaces.

**Responsibility:** `of`'s body, the mask it borrows from `ring_types`, and the
arithmetic identity that makes the two agree.

**In Scope:** `ring_index/src/lib.rs:48-52`;
`ring_types/src/capacity.rs:75-78`.

**Out of Scope:** What the fold costs relative to a division is
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_fold_costs.md).
Expanding a range of sequences is
[`algorithm/002`](002_a_run_is_the_fold_applied_count_times.md).

---

## The Whole of It

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -B1 -A3 -F 'pub fn of( seq : Seq, capacity : Capacity ) -> SlotIndex' ring_index/src/lib.rs
command grep -m1 -A3 -F '  pub const fn mask( self ) -> usize' ring_types/src/capacity.rs
```

Live output:

```
#[ must_use ]
pub fn of( seq : Seq, capacity : Capacity ) -> SlotIndex
{
  SlotIndex( ( seq.0 as usize ) & capacity.mask() )
}
  pub const fn mask( self ) -> usize
  {
    self.0 - 1
  }
```

Two lines of body across two crates. There is no branch, no loop, no error
path, and no state — `of` is a pure function of its two arguments, and the only
arithmetic it performs is one `and`.

---

### IX1 — Totality Is Bought Upstream, One Crate Away

`of` has no `Result` and no `# Panics` section. That is not an omission; it is
the consequence of where validation was put:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F '//! The validation lives upstream in [`ring_types::Capacity`], not here. Because' ring_index/src/lib.rs
command grep -m1 -A11 -F '  pub const fn new( slots : usize ) -> Result< Self, RingError >' ring_types/src/capacity.rs
```

Live output:

```
//! The validation lives upstream in [`ring_types::Capacity`], not here. Because
//! a `Capacity` cannot exist unless it is a power of two, [`of`] needs no check
//! and no error path: it is total.
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
```

**Finding.** The mask identity `x & (n - 1) == x % n` holds only when `n` is a
power of two. `Capacity::mask` computes `self.0 - 1` unconditionally, with no
check of its own, and `of` applies it with no check of its own — so the entire
correctness of this crate rests on a constructor in a *different* crate having
refused every non-power-of-two.

That is the right place for it, and the crate says so. What is worth recording
is how much rides on it: two functions in two crates, neither of which can fail,
neither of which validates, both of which are wrong for every capacity the
upstream constructor might one day let through. The coupling is invisible at
both sites — nothing in `mask`'s body or `of`'s body mentions powers of two, and
`mask` is a `const fn` that would happily produce `6` from a capacity of `7`.

The test that closes this loop lives here rather than upstream:
`non_power_of_two_capacity_is_rejected_upstream` asserts the constructor's
refusals from *this* crate's test file, which is the only place where the
dependency is visible as a dependency.

---

### IX2 — The Identity Holds for Every Input the Type Permits, Including the Last One

The acceptance criterion checks four laps of every capacity to 1024. The
sequence space is larger than that, and its far end is where an identity most
often stops holding:

```
--- (4) the fold near the end of the sequence space ---
  of( u64::MAX, cap 1024 )     = SlotIndex(1023)
  of( u64::MAX - 1, cap 1024 ) = SlotIndex(1022)
```

**Finding.** `u64::MAX` is `2^64 - 1`, so its low ten bits are all ones and the
fold returns `1023` — exactly `(2^64 - 1) % 1024`. The identity does not
degrade at the top of the range, because masking never carries and never
overflows: it reads bits and discards the rest.

This is the one operation in the crate with that property. Its neighbour `run`
reaches the same region by *adding* to a sequence, and adding does carry — see
[`pitfall/001`](../pitfall/001_the_run_that_panics_in_debug_and_wraps_in_release.md)
IX16. So the crate's safety at the end of the sequence space is not uniform: it
is a property of `of` alone, inherited by `aliases` (which is two `of` calls)
and lost by `run` (which is `count` additions before the folds).

No test covers `of` above `u32::MAX`; `derivation_is_a_mask` stops there.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](002_a_run_is_the_fold_applied_count_times.md) | The same fold, applied `count` times, and what changes |
| [`invariant/001`](../invariant/001_the_mask_equals_the_modulo.md) | The identity stated as a property and where it is asserted |
| [`workaround/001`](../workaround/001_the_mask_that_lives_one_crate_up.md) | Why `mask` is upstream and what that costs to read |
| [`pitfall/001`](../pitfall/001_the_run_that_panics_in_debug_and_wraps_in_release.md) | Where the totality argument stops applying |

### Sources

| Fact | Where |
|------|-------|
| `of`'s body | `ring_index/src/lib.rs:48-52` |
| `mask`'s body | `ring_types/src/capacity.rs:75-78` |
| The stated reason for no error path | `ring_index/src/lib.rs:24-26` |
| The fold at the top of the range | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `mask_equals_modulo_over_four_laps_of_every_capacity` | The identity across every capacity to 1024 |
| `derivation_is_a_mask` | That the result is the masked value, not merely a congruent one |
| `non_power_of_two_capacity_is_rejected_upstream` | The upstream refusal this crate's totality rests on |
| *(to create)* | `of` above `u32::MAX`, including `u64::MAX` — the region the probe measured and no test asserts |
