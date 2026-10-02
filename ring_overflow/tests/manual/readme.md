# ring_overflow manual testing plan

`tests/overflow_test.rs` asserts the three policies resolve correctly. This plan
covers the claim that no test can make on its own: that there is **no fourth
outcome**, and that the absence is required rather than incidental.

The requirement is negative: exactly three outcomes, no overwrite variant. A
negative is what a passing test suite is worst at guaranteeing. A test listing three variants keeps passing after someone adds a
fourth.

Run from the workspace root.

## M1. The resolution has exactly three variants

```bash
grep -nE -A 12 "pub enum Resolution" ring_overflow/src/lib.rs
```

**Expected:** three variants, each documented with what happened to the *data*,
not only whether the call succeeded. No `Overwrite`, no `Replace`, no
catch-all.

## M2. A fourth variant would break the build, not slip through

The exhaustive `match` in `would_resolve` and in the test's `name` helper is what
enforces the count. Confirm neither has a wildcard arm.

```bash
grep -nE -A 8 "pub const fn would_resolve" ring_overflow/src/lib.rs
grep -n "_ =>" ring_overflow/src/lib.rs ring_overflow/tests/overflow_test.rs
```

**Expected:** the first shows three arms and no `_`. The second finds nothing.
A wildcard anywhere would let a fourth variant compile silently, which is the
failure this plan exists to prevent.

## M3. The enum is not `#[non_exhaustive]`

A `#[non_exhaustive]` enum forces downstream wildcards, which would defeat M2
for every consumer even though this crate itself stays clean.

```bash
grep -n -B 3 "pub enum Resolution" ring_overflow/src/lib.rs
```

**Expected:** no `#[ non_exhaustive ]`. (Contrast `RingError` in `ring_types`,
which *is* non-exhaustive on purpose, because new error kinds are expected. New
overflow outcomes are not.)

## M4. The accounting claim holds for every policy

The crate claims exactly one counter moves per `resolve` call, "whichever branch
it takes". A refusal that skipped its counter would make a saturated `Fail` ring
read as idle.

```bash
cd ring_overflow && cargo test --test overflow_test exactly_one_counter_moves_per_call -- --nocapture
```

**Expected:** passes. Then read it. The assertion is `dropped_total() == 1` for
*every* policy including `Fail`, not just the two lossy ones.

## M5. The doc examples are the API's first reader

```bash
cd ring_overflow && cargo test --doc
```

**Expected:** every example passes and reads as an explanation on its own.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | Exactly three variants: `DroppedIncoming`, `EvictedOldest`, `Refused`. Each is documented by what happened to the data. No overwrite variant. |
| 2026-08-28 | M2 | ✅ | `would_resolve` has three arms, no `_`. No `_ =>` anywhere in the crate source or its test, so a fourth variant is a compile error in both places. |
| 2026-08-28 | M3 | ✅ | Derives are `Debug, Clone, Copy, PartialEq, Eq, Hash`; no `#[ non_exhaustive ]`, so downstream matches stay exhaustive too. |
| 2026-08-28 | M4 | ✅ | Passes. The loop runs `OverflowPolicy::ALL`, asserting `dropped( policy ) == 1` and `dropped_total() == 1` for each, `Fail` included. |
| 2026-08-28 | M5 | ✅ | 5 doc tests pass. |
