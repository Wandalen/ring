# data_structure

One struct with one field, and the field is a borrow. Everything the crate knows
is on the far side of a pointer it did not allocate, which is why the type is
sixteen bytes, `Copy`, and has no destructor.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [One Field and a Sixteen-Byte View](001_one_field_and_a_sixteen_byte_view.md) | The layout, the 8-in-64 ratio on the far side, and the four things not stored |
| 002 | [The Slice's Three Provenances](002_the_slices_three_provenances.md) | Where the borrowed cursors come from — a publisher, a gating set, or a bare array |

### The Type in One Table

| | Value |
|--|------:|
| Fields | 1 |
| `size_of` | 16 |
| `align_of` | 8 |
| Heap bytes owned | 0 |
| `impl Drop` | none |
| Bytes pointed at, per dependency | 64 |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -A4 'pub struct Barrier' ring_barrier/src/lib.rs
grep -hoE 'Barrier::over\([^)]*\)' ring_*/tests/*.rs | wc -l     # 63
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BR27 | `ring_barrier` | n/a — observation | `Barrier` derives `Debug` and so does `PaddedCursor`, so formatting a barrier performs one atomic load per dependency at whatever ordering `std` picked — a second door to shared memory in a crate that otherwise delegates every read so the ordering lives in one place |
| BR28 | family | n/a — observation | Ten of thirty-three crates carry a lifetime-parameterised struct and this is the smallest: one field, sixteen bytes, `Copy`; `GatingSet`, the other half of the same feature, has no lifetime and is not `Copy` because it owns its cursors |
| BR29 | family | n/a — observation | `PaddedCursor::new`, `Barrier::over` and `ring_consume::Consumer::new` are all `const`, so a bounded consumer can be built entirely at compile time; no `const` or `static` of either type exists anywhere, and keeping the chain intact constrains three constructors across three crates |
