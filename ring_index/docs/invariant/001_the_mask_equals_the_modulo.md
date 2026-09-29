# Invariant: The Mask Equals the Modulo

### Scope

**Purpose:** State the identity the crate exists to rely on, record where it is
asserted and how far, and record that the `as usize` truncation inside it is
provably harmless for a reason the code never gives.

**Responsibility:** `of( seq, capacity ) == SlotIndex( seq % capacity )`, for
every `Seq` and every constructible `Capacity`, on every target width.

**In Scope:** `ring_index/src/lib.rs:48-52`;
`ring_index/tests/index_test.rs:21-60`.

**Out of Scope:** where the identity's precondition is enforced is
[`decisions/001`](../decisions/001_a_power_of_two_or_nothing.md). Where totality
stops is [`invariant/002`](002_the_fold_is_total_and_the_run_is_not.md).

---

## The Statement

For all `seq : Seq` and all `capacity : Capacity`:

> `of( seq, capacity ).0 == (seq.0 as usize) % capacity.get()`

The identity holds because `capacity` is a power of two — that is the entire
content of the precondition, and it is enforced by `Capacity::new` one crate
upstream rather than here.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A20 -F '/// The acceptance criterion, verbatim: mask equals modulo across four full laps' ring_index/tests/index_test.rs
```

Live output:

```
/// The acceptance criterion, verbatim: mask equals modulo across four full laps
/// of every power-of-two capacity from 2 to 1024.
#[ test ]
fn mask_equals_modulo_over_four_laps_of_every_capacity()
{
  let mut slots = 2usize;
  while slots <= 1024
  {
    let c = cap( slots );
    for seq in 0..( 4 * slots ) as u64
    {
      assert_eq!
      (
        of( Seq( seq ), c ),
        SlotIndex( seq as usize % slots ),
        "capacity {slots}, seq {seq}"
      );
    }
    slots *= 2;
  }
}
```

---

### IX29 — The Identity Is Asserted Exhaustively Over a Prefix and Nowhere Else

**Finding.** The test is exhaustive in the dimension it covers and bounded in
both others: every power-of-two capacity from 2 to 1024, and every sequence from
0 to `4 × capacity`. That is 8,184 assertions and it is the strongest form the
identity has anywhere.

What it does not reach is the rest of the sequence space. The largest sequence
tested is 4095, at capacity 1024. `Seq` is a `u64`, so the assertions cover a
`4096 / 2^64` prefix of the input domain — and the region where an
identity like this would fail, if it were going to, is the far end rather than
the near one.

A second test extends the reach by sampling rather than sweeping:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A17 -F '/// The derivation is a mask, not a division. Asserted structurally: the result' ring_index/tests/index_test.rs
```

Live output:

```
/// The derivation is a mask, not a division. Asserted structurally: the result
/// equals the masked value for every capacity, which only holds when the
/// capacity is a power of two — so a division-based implementation that
/// happened to agree on modulo would still have to satisfy this.
#[ test ]
fn derivation_is_a_mask()
{
  let mut slots = 2usize;
  while slots <= 1024
  {
    let c = cap( slots );
    for seq in [ 0u64, 1, 7, 63, 1023, 1_000_000, u32::MAX as u64 ]
    {
      assert_eq!( of( Seq( seq ), c ), SlotIndex( seq as usize & c.mask() ) );
    }
    slots *= 2;
  }
}
```

This one reaches `u32::MAX` — four billion, and still 32 bits short of the top.
Its doc comment makes the sharper point, though: asserting against the *mask*
rather than the modulo is a structural check that a division-based
implementation would also have to pass, so the two tests together pin both the
result and the method.

Neither goes above `u32::MAX`. The probe in
[`algorithm/001`](../algorithm/001_one_and_of_a_mask.md) IX2 confirms the
identity holds at `u64::MAX`, which is a measurement rather than an assertion:
nothing re-runs it.

---

### IX30 — The Truncating Cast Is Safe for a Reason the Code Never States

`of` casts a `u64` sequence to `usize` before masking. On a 64-bit target that is
a no-op. On a 32-bit target it discards the top 32 bits of every sequence above
four billion:

```
--- (5) the `as usize` cast, on a 32-bit usize ---
  seq 0                     full-width    0   truncated-to-32    0   agree
  seq 1023                  full-width 1023   truncated-to-32 1023   agree
  seq 4294967295            full-width 1023   truncated-to-32 1023   agree
  seq 4294967296            full-width    0   truncated-to-32    0   agree
  seq 18446744073709551615  full-width 1023   truncated-to-32 1023   agree
```

**Finding.** The truncation cannot change the result, and the argument is one
sentence: `capacity` is a `usize`, so `mask = capacity - 1` fits in a `usize`, so
`& mask` clears every bit at or above the target's word width — exactly the bits
the cast already discarded. The cast and the mask throw away an overlapping set,
and the mask's set is the larger one.

This holds at any word width, including a hypothetical 16-bit target (where
`capacity ≤ 2^15`) and a 128-bit one (where the cast widens instead of
truncating). It is not a lucky property of 64-bit hosts.

The finding is that none of this is written down. `of`'s body contains a bare
`as usize` with no comment, and `as` casts that silently truncate are exactly
what a reviewer is trained to flag. The next person to read line 51 has to
reconstruct the argument from scratch or leave a review comment, and the answer
is short enough to sit in the source.

The probe's third row is the demonstration: `seq 4294967295` folds to `1023`
whether the cast keeps 64 bits or 32, because at capacity 1024 only the low ten
bits survive either way.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/001`](../algorithm/001_one_and_of_a_mask.md) | The body the identity describes, and its behaviour at `u64::MAX` |
| [`decisions/001`](../decisions/001_a_power_of_two_or_nothing.md) | The precondition, and where it is enforced |
| [`invariant/002`](002_the_fold_is_total_and_the_run_is_not.md) | The totality claim, and the function it does not cover |
| [`pitfall/002`](../pitfall/002_the_second_fold_nobody_noticed.md) | The second implementation that relies on the same identity |

### Sources

| Fact | Where |
|------|-------|
| The identity, asserted over a prefix | `ring_index/tests/index_test.rs:21-41` |
| The structural mask assertion | `ring_index/tests/index_test.rs:43-60` |
| The cast's behaviour at 32-bit width | Release probe, section 5, quoted above |
| `mask()`'s definition | `ring_types/src/capacity.rs:75-78` |

### Tests

| Test | Covers |
|------|--------|
| `mask_equals_modulo_over_four_laps_of_every_capacity` | The identity, exhaustively, to `seq = 4095` |
| `derivation_is_a_mask` | The method rather than the result, sampled to `u32::MAX` |
| `capacity_one_maps_everything_to_slot_zero` | The degenerate capacity, where mask is `0` |
| *(to create)* | The identity above `u32::MAX`, including `u64::MAX` — measured by probe, asserted by nothing |
