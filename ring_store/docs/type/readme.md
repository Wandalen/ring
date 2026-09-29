# type

Four types define this crate's surface: `Buffer< S >` itself and the three
`ring_types` values its signatures name. The instances take them in that order —
first the declared type, its three `impl` blocks and its single derive; then the
three that cross the boundary, their fields, their constructors and what each
guarantees on arrival.

Two things come out of it. The type system here is doing more work than it
appears to: `Capacity`'s private field and fallible constructor are what let
`mask()` exist, what let `new` size a slice without checking, and what make
`is_empty` a constant. And the one derive the workspace's lint forces onto
`Buffer` is the one with the sharpest edge — three million characters for a 1 MB
ring, including bytes a `clear` was believed to have removed.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [One Parameter, Three `impl` Blocks, One Derive](001_one_parameter_three_impl_blocks_one_derive.md) | BF46, BF47 — a bound split two crates now depend on, and a derive that dumps residue |
| 002 | [The Types That Cross the Boundary](002_the_types_that_cross_the_boundary.md) | BF48, BF49 — one validated type of three, and eight derives against one |

### The Three Crossing Types

| Type | Field | Constructor | Derives | Guarantees on arrival |
|------|-------|-------------|---------|----------------------|
| `Capacity` | private | `new` → `Result` | 8 | Non-zero, power of two |
| `Seq` | `pub u64` | tuple | 9 | None — any value is meaningful |
| `SlotIndex` | `pub usize` | tuple | 9 | None — any value is representable |

`Capacity` is a proposition, so it hides its field and rejects what would falsify
it. `Seq` and `SlotIndex` are coordinates, so there is nothing to reject. The
buffer's bounds check on `get` is the only thing between an arbitrary
`SlotIndex` and a slice index, which is what makes `get`'s provenance argument a
claim the type does not support.

One detail encodes the whole split: `Capacity`'s derive list is the other two's
minus `Default`. There is no default capacity, because zero is exactly the value
the constructor exists to reject.

### What `Buffer` Does Not Derive

`Clone`, `PartialEq`, `Eq`, `Hash`, `Ord`, `Default` — all absent, all for good
reasons, none written down. Cloning storage would give payloads a second owner
and copy `N` bytes per slot for the byte shape; comparing two buffers is not an
operation a ring needs and its absence pushes tests through `iter`; `Default`
would need a capacity that does not exist. The only derive present is the one
`missing_debug_implementations` requires, and both rings write manual `Debug`
impls to keep it away from their storage — `ring_spsc` explaining why,
`ring_mpsc` not.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the type, its three impl blocks, and its one derive
sed -n '/^#\[ derive( Debug ) ]$/,/^}$/p;/^impl< S : Default > Buffer< S >$/p;/^impl< S : Slot + Default > Buffer< S >$/p;/^impl< S > Buffer< S >$/p' ring_store/src/lib.rs
grep -n 'derive' ring_store/src/lib.rs

# the three crossing types and their derives
command grep -m1 -A1 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash ) ]' ring_types/src/capacity.rs
awk '/^\/\/\/ assert_eq!\( Seq\( 3 \)\.next\(\), Seq\( 4 \) \);$/{ n1 = NR } n1 && NR >= n1 + 2 && NR <= n1 + 3 { print } /^\/\/\/ assert_eq!\( SlotIndex\( 3 \)\.get\(\), 3 \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 3 { print }' ring_types/src/id.rs

# the one validation among them
sed -n '/pub const fn new/,/^  }/p' ring_types/src/capacity.rs

# the manual Debug impls that keep the derive away from a ring's storage
command grep -m1 -A8 -F 'impl< S > core::fmt::Debug for Ring< S >' ring_spsc/src/lib.rs
command grep -m1 -A10 -F 'impl< S > core::fmt::Debug for Ring< S >' ring_mpsc/src/lib.rs
```

Rendered `Debug` sizes, the surviving payload bytes, and the constructor's
accept/reject behaviour come from a release probe quoted in the instances.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BF46 | `ring_store` | n/a — observation | The unbounded `impl` block's generality is reachable only from downstream generic code, and every consumer in the family names `Buffer< TypedSlot< T > >` concretely, so the split currently buys a flexibility nobody has asked for |
| BF47 | `ring_store` | **latent hazard** | The lint-required `#[ derive( Debug ) ]` renders 3,153,718 characters for a 256×`BytesSlot< 4096 >` buffer and includes payload bytes that survived a `clear`; both rings avoid it manually, only one says why, and neither names the volume or the residue |
| BF48 | `ring_store` | n/a — observation | Of the three crossing types only `Capacity` is validated and only it has a private field, so `get`'s bounds check is all that stands between an arbitrary `SlotIndex` and a slice index |
| BF49 | `ring_store` | n/a — doc gap | `Buffer` derives one trait against `Capacity`'s eight; the absences of `Clone`, `PartialEq` and `Default` are each correct and each unstated, so a reader meets them as omissions rather than as decisions |
