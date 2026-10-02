# ring_store manual testing plan

`tests/buffer_test.rs` asserts the buffer stores and addresses correctly. This
plan covers the one claim a passing test cannot establish. That the buffer
**holds no cursor and no ordering state** is a claim about what the type does
*not* contain, and no amount of exercising a type demonstrates an absence.

The test file has two stand-ins for it, a `size_of` pin and an out-of-order
write sweep, and names the holes in both. What closes the question is a human
reading a short struct. That reading is this plan.

Run from the workspace root.

## M1. The struct has exactly two fields, and neither is a cursor

```bash
grep -n -A 6 "pub struct Buffer" ring_store/src/lib.rs
```

**Expected:** `slots : Box< [ S ] >` and `capacity : Capacity`, and nothing
else. A cursor, a sequence, a flag or a lock appearing here is the failure this
whole plan exists to catch.

## M2. No cursor, ordering or atomic appears in executable code

The words should appear only in prose explaining why they are absent.

```bash
# all lines mentioning them
grep -cE "cursor|Ordering|atomic" ring_store/src/lib.rs
# the same, restricted to executable code
grep -nE "cursor|Ordering|atomic" ring_store/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*(///|//!|//)"
```

**Expected:** the first count is non-zero, and the second produces no output at
all. A zero first count means the pattern is wrong and the check proves nothing.
An empty second result alone cannot tell "the crate is clean" from "the pattern
matches nothing anywhere", which is why the first command exists.

## M3. The panic decision is documented where a caller will hit it

`get`/`get_mut` index a slice directly and so panic out of range. That is a
deliberate choice over returning an `Option`, and a caller deserves the
reasoning rather than a bare `# Panics`.

```bash
grep -nE "# Panics" -A 7 ring_store/src/lib.rs
```

**Expected:** the doc states the reasoning, not just the fact. Specifically, a
`SlotIndex` from `ring_index::of` cannot be out of range, so one that is means
two rings' capacities were mixed, which is a defect rather than a condition.

## M4. `is_empty` cannot disagree with `len`

A hand-written `is_empty` that could drift from `len` is worse than the lint it
silences.

```bash
grep -n -B 14 "pub const fn is_empty" ring_store/src/lib.rs
```

**Expected:** the body reads from the same slice `len` reads from, and the doc
says why the answer is always false, because a `Capacity` cannot be zero.

## M5. The doc examples are the API's first reader

```bash
cargo test -p ring_store --doc
```

**Expected:** every example passes and reads as an explanation on its own, not
as a compiled assertion with an example's shape.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | Exactly two fields at lines 51–52: `slots : Box< [ S ] >`, `capacity : Capacity`. |
| 2026-08-28 | M2 | ✅ | 3 lines mention the words, 0 of them executable. All three are module prose explaining the refusal. |
| 2026-08-28 | M3 | ✅ | Both `# Panics` blocks present; `get`'s carries the full four-line reasoning, `get_mut`'s defers to it with "As [`Buffer::get`]". |
| 2026-08-28 | M4 | ✅ | Body is `self.slots.is_empty()`, the same slice `len` measures; the doc states "Always false, since a `Capacity` cannot be zero". |
| 2026-08-28 | M5 | ✅ | 8 doc tests pass. |
