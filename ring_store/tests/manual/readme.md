# ring_store — manual testing plan

`tests/buffer_test.rs` asserts the buffer stores and addresses correctly. This
plan covers the one clause of
`docs/feature/168_ring_buffer_storage.md` that a passing test cannot establish:
**"holds no cursor and no ordering state"** is a claim about what the type does
*not* contain, and no amount of exercising a type demonstrates an absence.

The test file has two stand-ins for it — a `size_of` pin and an out-of-order
write sweep — and both have holes it names openly. What actually closes the
question is a human reading a thirty-line struct. That reading is this plan.

Run from the workspace root.

## M1 — the struct has exactly two fields, and neither is a cursor

```bash
grep -n -A 6 "pub struct Buffer" ring_store/src/lib.rs
```

**Expected:** `slots : Box< [ S ] >` and `capacity : Capacity`, and nothing
else. A cursor, a sequence, a flag or a lock appearing here is the failure this
whole plan exists to catch.

## M2 — no cursor, ordering or atomic appears in executable code

The words should appear only in prose explaining why they are absent.

```bash
# all lines mentioning them
grep -cE "cursor|Ordering|atomic" ring_store/src/lib.rs
# the same, restricted to executable code
grep -nE "cursor|Ordering|atomic" ring_store/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*(///|//!|//)"
```

**Expected:** the first count is non-zero — otherwise the pattern is wrong and
the check proves nothing — and the second produces no output at all.

## M3 — the panic decision is documented where a caller will hit it

`get`/`get_mut` index a slice directly and so panic out of range. That is a
deliberate choice over returning an `Option`, and a caller deserves the
reasoning rather than a bare `# Panics`.

```bash
grep -nE "# Panics" -A 7 ring_store/src/lib.rs
```

**Expected:** the reasoning is stated, not just the fact — specifically that a
`SlotIndex` from `ring_index::of` cannot be out of range, so one that is means
two rings' capacities were mixed, which is a defect rather than a condition.

## M4 — `is_empty` cannot disagree with `len`

A hand-written `is_empty` that could drift from `len` is worse than the lint it
silences.

```bash
grep -n -B 14 "pub const fn is_empty" ring_store/src/lib.rs
```

**Expected:** the body reads from the same slice `len` reads from, and the doc
says why the answer is always false — a `Capacity` cannot be zero.

## M5 — the doc examples are the API's first reader

```bash
cargo test -p ring_store --doc
```

**Expected:** every example passes and reads as an explanation on its own, not
as a compiled assertion with an example's shape.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | Exactly two fields at lines 51–52: `slots : Box< [ S ] >`, `capacity : Capacity`. |
| 2026-08-28 | M2 | ✅ | 3 lines mention the words, 0 of them executable — all three are module prose explaining the refusal. |
| 2026-08-28 | M3 | ✅ | Both `# Panics` blocks present; `get`'s carries the full four-line reasoning, `get_mut`'s defers to it with "As [`Buffer::get`]". |
| 2026-08-28 | M4 | ✅ | Body is `self.slots.is_empty()`, the same slice `len` measures; the doc states "Always false — a `Capacity` cannot be zero". |
| 2026-08-28 | M5 | ✅ | 8 doc tests pass. |

M2's first command exists because of how the check failed on its first draft:
the executable-only grep returned nothing, which is indistinguishable between
"the crate is clean" and "the pattern matches nothing anywhere". Counting all
lines first makes the check non-vacuous — the same discipline
`docs/invariant/001_gate_non_vacuity.md` applies to the gates.
