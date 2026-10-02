# ring_slot manual testing plan

`tests/slot_test.rs` asserts both slot shapes behave. This plan covers the
constraint the module doc states, which is architectural rather than
behavioural: "both use the same claim, gating, and drain" and "the difference is
confined to what a slot contains." A test can show both shapes work; only
reading the code shows that nothing downstream *can* tell them apart.

Run from the workspace root.

## M1. The shared trait is the only interface the handshake needs

```bash
grep -nE -A 10 "pub trait Slot" ring_slot/src/lib.rs
```

**Expected:** `is_empty` and `clear`, the occupancy operations a drain needs,
and nothing shape-specific. A trait method returning `Option<T>` or `&[u8]`
would leak the shape back out and defeat the constraint.

## M2. No `unsafe`, and no `MaybeUninit` standing in for it

The module doc claims a partially-filled `BytesSlot` reads back exactly what was
written "without `MaybeUninit`". That claim is checkable.

```bash
grep -rn "unsafe\|MaybeUninit" ring_slot/src/
```

**Expected:** the only hits are inside doc comments stating none is used. The
crate is absent from `bench_harness/gate/declared/ring/unsafe_allowlist.txt`, so
the workspace `unsafe-code = "deny"` lint refuses any `unsafe` here.

## M3. A shorter write cannot leak the previous payload's tail

This is the property that matters across ring laps: slot N holds a 64-byte
message, is reused for a 2-byte one, and a consumer reading 64 bytes would see
62 bytes of the previous world's data.

```bash
cd ring_slot && cargo test --test slot_test a_shorter_write_does_not_leak_the_longer_one a_read_never_returns_the_unused_tail -- --nocapture
```

**Expected:** both pass. The second sweeps every length from 0 to capacity. An
off-by-one in the length tracking leaks exactly one stale byte, which a
single-length test would miss.

## M4. The two `is_empty` paths cannot disagree

`BytesSlot` carries an inherent `is_empty` *and* implements the trait's. Every
ordinary call site resolves to whichever is in scope, so the two could drift
apart without any test noticing.

```bash
grep -nE -B 6 "pub const fn is_empty" ring_slot/src/lib.rs
grep -nE -A 5 "impl< const N : usize > Slot for BytesSlot" ring_slot/src/lib.rs
```

**Expected:** the trait impl delegates to the inherent method rather than
repeating `self.len == 0`. The duplication exists only because clippy's
`len_without_is_empty` requires an inherent `is_empty` alongside `len`; the
delegation is what keeps it from becoming two independent definitions.

## M5. The doc examples are the API's first reader

```bash
cd ring_slot && cargo test --doc
```

**Expected:** every example passes and reads as an explanation on its own.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | `Slot` carries `is_empty` and `clear` only, with no `Option<T>` and no `&[u8]`. Nothing downstream can branch on which shape it holds. |
| 2026-08-28 | M2 | ✅ | Two hits, both in the module doc stating no `unsafe` and no `MaybeUninit` are used. The crate holds an allowlist entry it does not exercise. |
| 2026-08-28 | M3 | ✅ | Both pass. The sweep covers lengths 0–8 against an 8-byte capacity. |
| 2026-08-28 | M4 | ✅ | `Slot::is_empty` for `BytesSlot` is `Self::is_empty( self )`, so it delegates and there is one definition. Added `tests/slot_test.rs::the_inherent_and_trait_emptiness_agree` as the standing guard, since no ordinary call site exercises both paths. |
| 2026-08-28 | M5 | ✅ | 13 doc tests pass (12 before M4's new inherent `is_empty` example). |
