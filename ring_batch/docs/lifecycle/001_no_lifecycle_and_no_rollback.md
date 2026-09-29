# Lifecycle: No Lifecycle and No Rollback

### Scope

**Purpose:** Record that a `BatchClaim` has no lifecycle at all — no `Drop`, no
release, no rollback — what that means for a claim taken and not used, and where
in the family that rule is actually written down.

**Responsibility:** The absence of a destructor on `BatchClaim`, and the four
`Drop` implementations elsewhere in the family that fill the same role.

**In Scope:** `ring_batch/src/lib.rs`; `ring_tls/src/lib.rs:238-240`;
`ring_mpsc/src/lib.rs:985`, `:1257`; `ring_spsc/src/lib.rs:788`,
`:1130`.

**Out of Scope:** The zero-length claim, which is the one state transition the
type does have, is
[`lifecycle/002`](002_the_empty_claim_as_a_first_class_state.md). The duplicated
range object itself is
[`data_structure/002`](../data_structure/002_the_struct_ring_claim_wrote_again.md).

---

## Four Destructors in the Family, None Here

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every Drop in the family --'
command grep -r '^impl.*Drop for' --include=lib.rs ring_*/src/ | sed 's|ring/||'
echo '  -- and the crate that defines the claim --'
echo "    Drop/release/cancel/rollback in ring_batch : $( command grep -c 'Drop\|release\|cancel\|rollback' ring_batch/src/lib.rs || true )"
echo '  -- the rule, stated by the dependent instead --'
command grep -m1 -A2 -F '  /// feature 175 is about. The buffer is left empty whether or not the returned' ring_tls/src/lib.rs
```

Live output:

```
  -- every Drop in the family --
ring_mpsc/src/lib.rs:impl< S > Drop for Reserved< '_, S >
ring_mpsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Reservation< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
  -- and the crate that defines the claim --
    Drop/release/cancel/rollback in ring_batch : 1
  -- the rule, stated by the dependent instead --
  /// feature 175 is about. The buffer is left empty whether or not the returned
  /// [`Flush`] is fully consumed, because the sequences are already claimed:
  /// abandoning items mid-drain would leave sequences owned by nothing.
```

---

### BA30 — A Claim Cannot Be Given Back, and the Crate Does Not Say So

`fetch_add` moved the cursor. Nothing in `ring_batch` can move it back, and
nothing should: subtracting would hand the same sequences to a second producer,
which is the one thing the protocol exists to prevent. So a claim is permanent
from the instant it is returned, and a caller that takes eight sequences and then
fails has eight sequences that belong to it forever, unwritten.

The consequence is not stated anywhere in this crate. It is stated one crate up:

> abandoning items mid-drain would leave sequences owned by nothing.

**Finding.** `ring_tls::flush_into`'s doc gives the rule in a subordinate clause,
as a justification for draining the buffer unconditionally. That is the right
place for the *consequence* — it explains why `Flush` behaves as it does — but
the *rule* belongs to the type, and `BatchClaim`'s documentation never mentions
it. A reader of `ring_batch` alone learns that `claim` returns a range and is not
told that returning it is irreversible, or what to do when the work fails after
the range has been taken.

There is no good answer available at this tier, which is likely why none is
given. Filling an abandoned range requires publishing something into those slots,
and this crate has no storage dependency by design (see
[`data_structure/001`](../data_structure/001_sixteen_bytes_that_are_not_a_buffer.md)
BA10). What is missing is not a mechanism; it is a sentence saying the mechanism
belongs to the caller.

---

### BA31 — The Family Answered the Same Question Three More Times

Four `Drop` implementations exist, all of them in the two ring assemblies, and
two of them are on a type with the same two fields as `BatchClaim`:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the fourth range type, and its destructor --'
command grep -m1 -A5 -F 'pub struct Batch< '"'"'a, S >' ring_mpsc/src/lib.rs
command grep -m1 -A1 -F 'impl< S > Drop for Batch< '"'"'_, S >' ring_mpsc/src/lib.rs
command grep -m1 -A8 -F '  /// read as a missing publish barrier, arriving from the opposite direction.' ring_mpsc/src/lib.rs | tail -n 8
echo '  -- against the one here --'
command grep -m1 -A4 -F 'pub struct BatchClaim' ring_batch/src/lib.rs
```

Live output:

```
  -- the fourth range type, and its destructor --
pub struct Batch< 'a, S >
{
  ring : &'a Ring< S >,
  start : Seq,
  len : usize,
}
impl< S > Drop for Batch< '_, S >
{
  fn drop( &mut self )
  {
    self
      .ring
      .consumer_cursor()
      .store( self.start.advanced_by( self.len as u64 ), COMMIT );
  }
}
  -- against the one here --
pub struct BatchClaim
{
  start : Seq,
  count : usize,
}
```

**Finding.** `ring_mpsc::Batch` is `BatchClaim` plus a ring reference, and the
reference is what makes the destructor possible — a range that knows which ring
it came from can commit itself on scope exit. `ring_spsc::Batch` is the same
shape again. Neither depends on `ring_batch`.

So the family holds four spellings of "a contiguous range of sequences":
`BatchClaim` at Tier 2 with no lifecycle, `ring_claim::Claim` at Tier 5 with no
lifecycle, and two `Batch` types at Tier 6 that each carry one. The two that
solved the lifecycle problem did it by adding the storage reference the two lower
ones deliberately decline — which is a coherent architecture, and is written down
nowhere. Nothing in `ring_batch` says "a range without a ring cannot commit
itself, so this type stops at the arithmetic"; that sentence would have made all
four types legible as one design instead of four rediscoveries.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/002`](002_the_empty_claim_as_a_first_class_state.md) | The one state the type does model |
| [`data_structure/002`](../data_structure/002_the_struct_ring_claim_wrote_again.md) | Two of the four range types, compared field by field |
| [`data_structure/001`](../data_structure/001_sixteen_bytes_that_are_not_a_buffer.md) | The storage dependency whose absence makes a destructor impossible here |
| [`pattern/001`](../pattern/001_the_range_object.md) | The shape all four share |

### Sources

| Fact | Where |
|------|-------|
| No destructor or release in this crate | Census above |
| The rule, stated by the dependent | `ring_tls/src/lib.rs:238-240` |
| The four family destructors | `ring_mpsc/src/lib.rs:985`, `:1257`; `ring_spsc/src/lib.rs:788`, `:1130` |
| `ring_mpsc::Batch`'s fields and `drop` | `ring_mpsc/src/lib.rs:1143-1148`, `:1257-1272` |

### Tests

| Test | Covers |
|------|--------|
| `claims_are_copied_not_moved_and_compare_by_value` | That the type is `Copy`, which is only sound because there is nothing to release |
| *(to create)* | Nothing exercises an abandoned claim, because there is no operation to exercise |
