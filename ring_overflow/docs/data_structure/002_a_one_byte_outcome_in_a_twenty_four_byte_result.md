# Data Structure: A One-Byte Outcome in a 24-Byte `Result`

### Scope

**Purpose:** Measure what `resolve`'s return type costs against `would_resolve`'s,
and record what borrowing a workspace-wide error enum does to the exhaustiveness
this crate maintains everywhere else.

**Responsibility:** `Result< Resolution, RingError >` as a layout, `RingError`'s
nine variants and two payloads, and its `#[ non_exhaustive ]` attribute.

**In Scope:** `ring_overflow/src/lib.rs:192`, `:229`;
`ring_types/src/error.rs:43-80`.

**Out of Scope:** Why the error is used at all rather than `Ok( Refused )` is
[`decisions/002`](../decisions/002_fail_returns_an_error_not_a_resolution.md).
The one-byte half is
[`data_structure/001`](001_one_byte_and_two_hundred_fifty_three_spare_niches.md).

---

## Two Return Types for the Same Three Outcomes

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the two return types --'
sed -n '/^pub fn resolve( policy : OverflowPolicy, stats : &RingStats ) -> Result< Resolution, RingError >$/p;/^pub const fn would_resolve( policy : OverflowPolicy ) -> Resolution$/p' ring_overflow/src/lib.rs
echo '  -- what the error half is --'
command grep 'non_exhaustive' ring_types/src/error.rs
printf '    variants in RingError           %s\n' "$( command grep -m1 -B1 -A35 -F '  /// A capacity of zero was requested. A ring with no slots can never accept' ring_types/src/error.rs | command grep -c '^  [A-Z]' || true )"
echo '  -- the two that carry a payload --'
command grep '^  CapacityNotPowerOfTwo(\|^  BatchTooLarge' ring_types/src/error.rs
echo '  -- and the one this crate returns --'
command grep '^  Full,' ring_types/src/error.rs
```

Live output:

```
  -- the two return types --
pub fn resolve( policy : OverflowPolicy, stats : &RingStats ) -> Result< Resolution, RingError >
pub const fn would_resolve( policy : OverflowPolicy ) -> Resolution
  -- what the error half is --
/// `#[ non_exhaustive ]` reserves the right to add variants, and this is the
/// Root cause: `#[ non_exhaustive ]` announces that variants may be added and
#[ non_exhaustive ]
  // Root cause: `#[ non_exhaustive ]` on this very enum documents that variants get
    variants in RingError           9
  -- the two that carry a payload --
  CapacityNotPowerOfTwo( usize ),
  BatchTooLarge
  -- and the one this crate returns --
  Full,
```

The size table is the one quoted in
[`data_structure/001`](001_one_byte_and_two_hundred_fifty_three_spare_niches.md).
These are the closing lines of that same probe run, on
`aarch64-unknown-linux-gnu`:

```
    would_resolve returns 1 byte(s); resolve returns 24
    the ratio is 24x, for the same three outcomes
```

---

### OV11 — The Effectful Half Returns Twenty-Four Times the Bytes for the Same Information

`would_resolve` returns `Resolution` — one byte, alignment one.
`resolve` returns `Result< Resolution, RingError >` — twenty-four bytes,
alignment eight. Both convey which of three things happened.

The factor of twenty-four is entirely `RingError`'s. That enum has nine variants
for the whole family, two of which carry payloads: `CapacityNotPowerOfTwo( usize )`
and `BatchTooLarge { requested : usize, capacity : usize }`. Two `usize` and a
discriminant is twenty-four bytes, and every `Result` mentioning `RingError`
inherits it.

**Finding.** `resolve` uses exactly one of the nine — `RingError::Full`, which
carries nothing — and pays for the largest. Substituting a unit error makes the
whole return one byte again: `Result< Resolution, () >` measures 1/1 in the same
run. The alignment change from one to eight is the part that actually shows up,
since it forces the return into a register pair rather than a byte.

Nothing about this is wrong. `RingError` is the family's error type and a
handler returning a different one would be worse. It is recorded because the
crate's own justification for `Resolution` is that named outcomes are cheap
enough to prefer over a boolean — and the shape it hands them back in is the one
thing here that is not cheap, measured, unmentioned, and twenty-four times the
alternative it sits next to in the same file.

**Disposition:** declined — this instance's own text states "Nothing about
this is wrong" and traces the full cost to `RingError`'s shape in
`ring_types/src/error.rs`, a workspace-wide error enum shared by all
nine variants; shrinking it to fix this one crate's return size is a decision
in `ring_types`, not a defect in this crate's own source or
`data_structure/002_a_one_byte_outcome_in_a_twenty_four_byte_result.md`.

---

### OV12 — Crossing Into `Result` Is Where the Crate's Exhaustiveness Stops

Every `match` inside `ring_overflow` is exhaustive with no `_` arm, and that is
the crate's main structural guarantee: a fourth `OverflowPolicy` variant produces
two compile errors naming both sites
([`algorithm/001`](../algorithm/001_two_mappings_over_three_policies.md) § OV1).

`RingError` is `#[ non_exhaustive ]`. A caller outside `ring_types` matching on
`resolve`'s error arm must therefore include a `_`, permanently — not because
this crate has more than one error to give, but because the shared enum reserves
the right to grow.

**Finding.** So `resolve`'s return is exhaustive on the `Ok` side, where two of
three outcomes live, and structurally inexhaustible on the `Err` side, where the
third does. A caller writing the natural three-way handler gets compile-time
totality for `DroppedIncoming` and `EvictedOldest` and a mandatory catch-all for
`Full`, which is the one outcome the caller most needs to distinguish, since it
is the only one where the caller still holds the item.

`would_resolve` has none of this: it returns a three-variant enum that is not
`non_exhaustive`, so a caller matching on it gets full exhaustiveness across all
three. The pure half is the one with the stronger type, and the crate documents
it as the convenience form — "for callers deciding what a policy *would* do" —
rather than as the shape that keeps the guarantee.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/001`](001_one_byte_and_two_hundred_fifty_three_spare_niches.md) | The one-byte value this wraps |
| [`decisions/002`](../decisions/002_fail_returns_an_error_not_a_resolution.md) | The choice that put an error here |
| [`workaround/002`](../workaround/002_one_outcome_expressed_in_two_type_systems.md) | The duplication this creates |
| [`api/001`](../api/001_four_declarations_three_of_them_const.md) | Why the `Result` return needs no `must_use` |

### Sources

| Fact | Where |
|------|-------|
| The two return types | `ring_overflow/src/lib.rs:192`, `:229` |
| `RingError` is `non_exhaustive` | `ring_types/src/error.rs:43` |
| The two payload-carrying variants | `ring_types/src/error.rs:51`, `:65-71` |
| `RingError::Full`'s own declaration | `ring_types/src/error.rs:54` |
| Sizes and alignments | Probe; table in `data_structure/001`, ratio quoted above |

### Tests

| Test | Covers |
|------|--------|
| `fail_hands_the_decision_back_as_an_error` | That `Full` is what arrives in the `Err` arm |
| `resolve_agrees_with_would_resolve` | The `Ok` side, on both its variants |
| `policy_self_description_agrees_with_the_handler` | `is_err()` against the policy's own claim |
| *(to create)* | No test asserts a size, so widening the error would pass silently |
