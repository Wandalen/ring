# invariant

Properties that must hold for the crate to be correct, stated as properties
rather than as code. There are two: the mask equals the modulo, and the fold is
defined everywhere. Both are true. Neither is stated in full anywhere in the
source, and the second is true of two functions out of three.

The distinction that organizes this definition is between a property being
*held* and a property being *said*. `of`'s totality is held by construction and
said in the module comment. `run`'s partiality is held by construction and said
nowhere. The `as usize` truncation is harmless for a provable reason, and the
proof exists in no comment.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_mask_equals_the_modulo.md) | The Mask Equals the Modulo | The identity, how far the two tests reach, and why the truncating cast is safe |
| [002](002_the_fold_is_total_and_the_run_is_not.md) | The Fold Is Total and the Run Is Not | Each function's domain, and the boundary that is not where a reader would guess |

## The Identity Is Asserted Over a Prefix and Measured at the End

`mask_equals_modulo_over_four_laps_of_every_capacity` is exhaustive up to
`seq = 4095` — roughly 8,000 assertions, and the strongest statement of the
identity the repository contains. `derivation_is_a_mask` samples out to
`u32::MAX` and, more usefully, asserts against the *mask* rather than the
modulo, so it would catch a division-based implementation that happened to agree
numerically.

Above `u32::MAX` there is nothing. The probe confirms `of( u64::MAX, 1024 )` is
`1023`, which is correct and is not a test.

## Two Domains, One of Them Undeclared

`of` and `aliases` are total: masking discards bits, and discarding never
overflows. `run` adds before it folds, so its domain is
`start.0 + ( count - 1 ) ≤ u64::MAX` for `count > 0` — the last step its
`0..count` loop actually takes, a bound that depends on `start` rather than on
`capacity`, and is therefore invisible from the arguments a caller is usually
thinking about.

The crate has zero `# Panics` sections and one sentence about totality, correctly
scoped to `of` and positioned where it reads as a summary of the whole crate.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^\/\/! The validation lives upstream in \[`ring_types::Capacity`\], not here\. Because$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 2 { print } /^\/\/\/ assert_eq!\( of\( Seq\( 13 \), cap \), SlotIndex\( 5 \) \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 6 { print }' ring_index/src/lib.rs
sed -n '/^\/\/\/ The acceptance criterion, verbatim: mask equals modulo across four full laps$/,/^}$/p;/^\/\/\/ The derivation is a mask, not a division\. Asserted structurally: the result$/,/^}$/p' ring_index/tests/index_test.rs
echo "  # Panics sections in the crate: $( command grep -c '# Panics' ring_index/src/lib.rs )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| IX29 | `ring_index` | n/a — coverage | The identity is asserted exhaustively to `seq = 4095` and sampled to `u32::MAX`; the top 32 bits of the domain are measured by probe and asserted by nothing |
| IX30 | `ring_index` | n/a — doc gap | `seq.0 as usize` truncates on any target below 64 bits and provably cannot change the result, because the mask discards a superset of what the cast does — written down nowhere |
| IX31 | `ring_index` | n/a — doc gap | The one totality claim is correctly scoped to `of` and sits where it reads as covering the crate; `run` is partial and carries no `# Panics` |
| IX32 | `ring_index` | n/a — doc gap | `run`'s real boundary is `start + ( count - 1 )` against `u64::MAX`, not `count` against `capacity` (nor the coarser `start + count`); the doc comment addresses the constraint that is not one |
