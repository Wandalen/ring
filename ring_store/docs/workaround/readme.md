# `ring_store` opts out of the workspace unsafe-code deny for two unchecked index expressions

Status: Accepted

## Context

The workspace sets `unsafe-code = "deny"` and `undocumented_unsafe_blocks = "deny"`.
`ring_store::Buffer` owns the slot allocation both rings index into. Two hot
indexing sites — `Buffer::at`/`at_mut` (slot by masked sequence) — paid a bounds
check per access that the compiler cannot remove: it cannot see that `mask() + 1`
equals the allocation length. The `stages` benchmarks measured the check at ~10%
on slot-heavy paths (fused push, batch claim), flat where atomics dominate. A
length-masked safe spelling was benched first and keeps the check; only the
unchecked form measures faster.

`bench_harness/gate/declared/ring/unsafe_allowlist.txt` names this crate, and
gate G6 requires this file to justify the opt-out.

## Decision

`ring_store` carries `#![allow(unsafe_code)]`, and its unsafe code is confined
to these two expressions:

- `Buffer::at` / `Buffer::at_mut`: `get_unchecked` on a mask-folded index.

These rules bound it:

- No unsafe API is public and no aliasing is involved. Both functions return
  shared or exclusive borrows the borrow checker already tracks; the unchecked
  access removes only the range check, and the range is provable from fields
  this crate owns: `of()` folds through `capacity.mask()`, and `Buffer::new`
  allocates exactly `capacity` slots with no reallocation path. This is
  deliberately narrower than the aliasing unsafe the family ruled `ring_store`
  cannot encapsulate (see the allowlist's own history): that verdict is about
  shared `&mut` across threads, which needs cursor knowledge this crate does
  not hold. A range proof needs only the allocation this crate does hold.
- The arbitrary-index `Buffer::get` / `get_mut` stay checked, with their
  documented panic contract unchanged. Only the mask-folded path skips the
  check, and only that path can prove it sound.
- Every block carries a `// SAFETY:` comment naming the two facts above.

## Alternatives considered

- **Keep the check.** Costs ~10% on the slot-heavy paths for a condition that
  is provably always true. The check guards against a caller mixing two rings'
  capacities — but that caller reaches this code only through `at`, which
  folds with this buffer's own capacity, so the guarded case cannot arise here.
- **Mask by the slice's own length (safe).** Benched on `write`, `fused` and
  `claim/mpsc_batch32`: within noise of the checked baseline, well short of
  the unchecked form. The compiler does not remove the check in this call
  shape, so the safe spelling pays for safety it cannot deliver.
- **Move the unchecked fold into the rings.** Duplicates the one fold
  `ring_index` exists to be the only one of, and still needs buffer internals
  the rings cannot reach without breaking the crate boundary.
