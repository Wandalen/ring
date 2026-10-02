# ring_index manual testing plan

`tests/index_test.rs` asserts the fold is correct. This plan covers the two
things an assertion cannot: that the implementation is a mask rather than
a modulo that happens to agree, and that the power-of-two constraint is enforced
somewhere a caller cannot route around.

Run from the workspace root.

## M1. The fold compiles to a mask, not a division

The fold must be computed by mask, not division. A test asserting
`of(seq, cap) == seq % cap` passes either way, because for powers of two the two
agree. The only way to tell is to look at what the code does.

```bash
grep -n -A 6 "pub fn of" ring_index/src/lib.rs
```

**Expected:** the body uses `& capacity.mask()`. No `%` and no `/` anywhere in
the fold.

```bash
grep -n "%" ring_index/src/lib.rs
```

**Expected:** every hit is inside a doc comment (`//!` or `///`), explaining
what the mask is equivalent to and why the division is avoided. None in
executable code. The modulo appears as *executable* code only in the test, as
the independent oracle the mask is checked against.

## M2. A non-power-of-two capacity cannot reach the fold

`of` has no error path. That is only safe if an invalid capacity is
unconstructible upstream, so this checks that the type, rather than convention,
enforces the constraint.

```bash
grep -rn "fn of" ring_index/src/lib.rs
```

**Expected:** the signature takes `Capacity`, not `usize`. A caller cannot
supply `7` without going through `Capacity::new`, which rejects it.

## M3. The aliasing the family is built to survive is visible

Two sequences a whole number of laps apart land on the same slot. That is the
ring's defining behaviour, gated elsewhere, not a bug to fix. It should read as
intentional.

```bash
grep -n -B 8 "pub fn aliases" ring_index/src/lib.rs
```

**Expected:** the doc says aliasing is expected and names what prevents it being
*observable* (the gating in `ring_seqno` / `ring_barrier`), rather than presenting
it as a hazard with no answer.

## M4. The doc examples are the API's first reader

```bash
cd ring_index && cargo test --doc
```

**Expected:** every example passes and reads as an explanation on its own.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | `of` is `SlotIndex( ( seq.0 as usize ) & capacity.mask() )`. No `/` at all; both `%` hits are doc prose (line 11 explains the 20–40 cycle cost avoided, line 25 states the equivalence). The check's original wording demanded `grep -c '%'` be 0. It was corrected above, since that would have failed on documentation that is doing its job. |
| 2026-08-28 | M2 | ✅ | `pub fn of( seq : Seq, capacity : Capacity ) -> SlotIndex`. Capacity is the validated type, so `7` cannot reach the fold. |
| 2026-08-28 | M3 | ⚠️→✅ | Module doc and `of`'s doc both named the gating, but `aliases`'s own doc stopped at "a whole number of laps apart", so a reader landing on that function alone would see a hazard with no answer. Fixed: added the paragraph naming `may_claim` and the barrier as what makes aliasing unobservable. Re-checked. |
| 2026-08-28 | M4 | ✅ | 3 doc tests pass. |
