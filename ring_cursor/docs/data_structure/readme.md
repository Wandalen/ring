# data_structure

The two layouts this crate declares, as bytes rather than as fields.

### Overview Table

| ID | Name | Size | Payload | Spent on separation |
|----|------|-----:|--------:|--------------------:|
| 001 | [The Padded Cursor](001_the_padded_cursor.md) | 64 | 8 | 56 bytes (87.5%) |
| 002 | [The Cursor Pair](002_the_cursor_pair.md) | 192 | 24 | 168 bytes (87.5%) |

**The ratio is the crate.** Seven bytes of padding for every byte of state, at
both scales, bought to stop two cores from invalidating each other's copy of a
line they are not sharing data through. Whether that trade pays is `ring_bench`'s
question; whether the layout the measurement will be
taken on is the layout claimed is
[`non_functional_requirement/001`](../non_functional_requirement/001_the_layout_claim_is_testable.md)'s.

Neither entry pins a field *offset*. `#[ repr( align ) ]` does not imply
`repr( C )`, so placement within each structure stays the compiler's business —
what is fixed is the size, the alignment, and therefore the stride.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two structures, and the fields of the second --'
command grep -E '^pub struct|^  (producer|consumer|capacity) :' ring_cursor/src/lib.rs
```

Live output:

```
  -- the two structures, and the fields of the second --
pub struct PaddedCursor( CacheAligned< AtomicSeq > );
pub struct CursorPair
  producer : PaddedCursor,
  consumer : PaddedCursor,
  capacity : Capacity,
```

**One field in the first, three in the second.** A structure whose whole content
is a layout claim has nothing to check except measurements of it, which is why
this definition's two instances are both about sizes rather than behaviour.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CU9 | `PaddedCursor` | n/a — observation | A one-field tuple struct over `CacheAligned< AtomicSeq >` with no field, method body, or constant of its own — every one of its 64 bytes and all four of its behaviours come from types in two other crates. It is the smallest structure in the family that still owns a name |
| CU10 | `Default` | n/a — coverage | `PaddedCursor` derives `Default` and `CursorPair` cannot, because a `Capacity` has none. The two structures therefore differ in constructibility in a way no test asserts and no document states, and a caller reaching for `CursorPair::default()` learns it from the compiler |
| CU11 | `tests/cursor_test.rs:419-420` | **misleading doc** | The assertion was `size <= 3 * CACHE_LINE` with message "capacity must not cost a whole extra line". `3 * CACHE_LINE` is 192 and the measured size is 192, so the bound was satisfied at exactly the value the message forbade — the test passed in the state it was written to reject. **Disposition: applied** — message corrected to "capacity may cost up to one additional cache line" |
| CU12 | Field order | n/a — observation | `capacity` is declared after two 64-aligned fields, so its 8 bytes land at the start of a third line and 56 bytes of that line are padding. That is the same 56-in-64 ratio `001` documents for a cursor, paid a second time for a value that is written once and never again |
