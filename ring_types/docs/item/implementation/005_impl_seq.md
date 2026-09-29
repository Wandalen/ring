# impl Seq

## Representation

The inherent implementation block on [`Seq`](../struct/002_seq.md) — the crate's
largest, holding one associated constant
([`ZERO`](../associated_constant/001_seq_zero.md)) and three associated
functions ([`next`](../associated_function/007_seq_next.md),
[`advanced_by`](../associated_function/008_seq_advanced_by.md),
[`distance_to`](../associated_function/009_seq_distance_to.md)).

**Its four members divide cleanly into three that trust the caller and one that
does not.** `next` and `advanced_by` are plain `+` — they panic in a debug build
and wrap in release, and the type's guarantee is that no reachable workload gets
near `u64::MAX`. `distance_to` is `saturating_sub`, which is the block's only
defensive operation and the only one whose behaviour at the boundary is
specified rather than argued.

That asymmetry is deliberate and its rationale is in the doc: "the caller that
needs the direction has already compared the two, and every caller that does not
wants a count." A signed return would push a branch onto every call site to
recover an ordering the caller already knows.

## Kind

Implementation (§ Item Kind Taxonomy : Stable Item Kinds #12)

## Definition

`ring_types/src/id.rs:27`

```rust
impl Seq
{
  pub const ZERO : Self = Self( 0 );                    // :30
  pub const fn next( self ) -> Self                     // :45
  pub const fn advanced_by( self, n : u64 ) -> Self     // :65
  pub const fn distance_to( self, later : Self ) -> u64 // :82
}
```

The constant is declared first, before the functions — the same ordering the two
policy impls use for their `ALL` arrays. All four members are `const`.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/id.rs` | 27, 30, 45, 65, 82, 86 | **Block header (27)**; `ZERO` (30); `next` (45); `advanced_by` (65); `distance_to` (82); closing brace (86) |

The block spans 27-86 — half the file. `SlotIndex` and its own impl occupy the
other half from line 88.

Test-only references: `ring_types` — four of the nineteen tests in
`tests/types_test.rs` exercise this block (`seq_orders_by_count`, `seq_advances`,
`seq_does_not_wrap_within_any_reachable_workload`,
`seq_distance_saturates_backward`), plus 20 consumer suites reaching the members
through `Seq`.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/id.rs` | Declares the block |
| *(eighteen consumers)* | `src/lib.rs` each | Call at least one member — the full `Seq` consumer list is on [`../struct/002_seq.md`](../struct/002_seq.md) |

**`ZERO` is matched on 59 `src/` lines across eleven of the eighteen** — but 53
of those are doc comments, and the production count is **six**
(→ [`../associated_constant/001_seq_zero.md`](../associated_constant/001_seq_zero.md)).
Rings start at zero and *say so* constantly; they set it rarely, because a ring
is constructed once.

**By production references the block's most-used member is
[`advanced_by`](../associated_function/008_seq_advanced_by.md) with 14**, then
[`distance_to`](../associated_function/009_seq_distance_to.md) with 11, then
`ZERO` with 6, then [`next`](../associated_function/007_seq_next.md) with 2. By
raw grep the order is `ZERO` first by a factor of four. **The two orderings
disagree completely, and only one of them describes what the code does.**

**The block also carried the crate's one known documentation defect, now fixed.**
`next`'s doc claimed it "saturates in a release build"; release-mode `u64`
addition wraps. Both halves have since been written: `next`'s paragraph says
"wraps to zero in a release build" and keeps the 584-year figure as the reason
that is survivable, and `advanced_by` — which had no overflow note at all despite
being, by the count above, the block's most-used member in production — now
states that the reachability argument does *not* carry over to it, because `n`
comes from the caller and reaches the wrap in a single call. The reproducing
probe and the mitigation census are in
[`../../pitfall/001`](../../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md).
