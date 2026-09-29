# pub use id::{ Seq, SlotIndex }

## Representation

Re-exports both position types from the private [`id`](../module/003_id.md)
module in a single brace-list declaration. The first of the crate's two
multi-name re-exports, and the pairing is not cosmetic: `Seq` and `SlotIndex`
are the two halves of one idea — a position before the fold and after it — and
exporting them together is the closest the export surface comes to saying so
(→ [`../../data_structure/001`](../../data_structure/001_two_position_types_and_the_fold_between_them.md)).

**The two names have wildly different reach through this one line.**
[`Seq`](../struct/002_seq.md) resolves for eighteen sibling crates' `src/`;
[`SlotIndex`](../struct/003_slot_index.md) for three. Fifteen crates therefore
import a brace list and use half of it — which is invisible at this line and
visible only in the per-type Crate Usage tables.

## Kind

Use Declaration (§ Item Kind Taxonomy : Stable Item Kinds #3)

## Definition

`ring_types/src/lib.rs:43`

```rust
pub use id::{ Seq, SlotIndex };
```

Names in declaration order rather than alphabetical — `Seq` is declared at
`id.rs:25` and `SlotIndex` at `id.rs:99`, and the re-export follows the file. Its
neighbour at line 44 does the opposite (`OverflowPolicy` before `WaitKind`, which
is alphabetical and reverses the declaration order in `policy.rs`), so the crate
is not consistent about this. Neither ordering means anything to the compiler.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/lib.rs` | 43 | **The declaration.** Its only occurrence |

Test-only references: 20 consumer test suites for `Seq` and 3 for `SlotIndex`,
plus `ring_types`' own, all resolving through this line without naming it.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/lib.rs` | Declares the re-export |
| *(eighteen consumers)* | `src/lib.rs` each | Resolve `ring_types::Seq` — full list on [`../struct/002_seq.md`](../struct/002_seq.md) |
| *(three consumers)* | `src/lib.rs` each | Resolve `ring_types::SlotIndex` — `ring_batch`, `ring_store`, `ring_index` |

**Eighteen and three are not eighteen plus three.** The `SlotIndex` consumers are
a subset of the `Seq` consumers — every crate that folds a position also holds
the unfolded one — so this line serves eighteen crates in total, three of which
use both names.
