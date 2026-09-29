# Data Structure: The One Collection the Crate Builds

### Scope

**Purpose:** Describe the only owned structure `ring_index` can produce — the
`Vec< SlotIndex >` `run` returns — as a layout: what it costs at each size, why
it is exactly sized, and what the header/payload ratio is at the sizes a caller
would plausibly ask for.

**Responsibility:** `run`'s return value as a data structure rather than as a
signature.

**In Scope:** `ring_index/src/lib.rs:118-122`.

**Out of Scope:** whether it should be a `Vec` at all is
[`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md). The
allocation as a cost is
[`non_functional_requirement/002`](../non_functional_requirement/002_the_one_allocation_and_the_zero_callers.md).
The rest of the family stores no slot indices at all —
[`data_structure/001`](001_the_ring_is_a_computation_not_a_layout.md).

---

## The Only Owned Thing in the Crate

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A6 -F '/// assert_eq!( run( Seq( 0 ), 0, cap ), vec![] );' ring_index/src/lib.rs | tail -n 5
echo '  -- and every other heap allocation in the crate --'
command grep -E 'Vec|Box|String|to_vec|collect|to_owned' ring_index/src/lib.rs
```

Live output:

```
#[ must_use ]
pub fn run( start : Seq, count : usize, capacity : Capacity ) -> Vec< SlotIndex >
{
  ( 0..count as u64 ).map( | n | of( start.advanced_by( n ), capacity ) ).collect()
}
  -- and every other heap allocation in the crate --
/// and the family's own doctests run 2 to 8 — the returned `Vec`'s 24-byte
pub fn run( start : Seq, count : usize, capacity : Capacity ) -> Vec< SlotIndex >
  ( 0..count as u64 ).map( | n | of( start.advanced_by( n ), capacity ) ).collect()
```

Two lines match across the whole file, and both are inside `run`: the `Vec` in
its signature and the `collect` in its body. Nothing else in `ring_index` owns
anything.

---

### IX51 — The Collection Is Exactly Sized, Which Is Measured and Undocumented

```
  size_of Seq            :   8
  size_of SlotIndex      :   8
  size_of Capacity       :   8
  size_of Vec<SlotIndex> :  24  (ptr + len + cap, on the stack)
  run-of-1: len 1 capacity 1  heap bytes 8
  run-of-1024: len 1024 capacity 1024  heap bytes 8192
  exactly sized in both cases: true
```

**Finding.** `run`'s `Vec` never over-allocates. `len` equals `capacity` at every
size measured, including the growth-sensitive ones, and the earlier per-call
probe agrees:

```
--- (3) what one `run` costs, by count ---
  count 0      0 alloc       0 bytes  len 0  capacity 0
  count 1      1 alloc       8 bytes  len 1  capacity 1
  count 8      1 alloc      64 bytes  len 8  capacity 8
  count 1024   1 alloc    8192 bytes  len 1024  capacity 1024
  count 4096   1 alloc   32768 bytes  len 4096  capacity 4096
```

One allocation per call, never two — no doubling, no reallocation. That is a
structural property rather than luck: the body collects a `Map` over a `Range`,
which is `ExactSizeIterator`, so `collect` reads a precise size hint and
allocates once at the final size.

The finding is that this depends entirely on the body's shape and is written down
nowhere. Rewriting the body as a `filter_map`, a `flat_map`, or a loop with
`push` would silently turn one exact allocation into a doubling sequence — up to
eleven allocations for `count = 1024` — with no signature change, no doc change,
and no test that would notice. The crate's four `run` tests assert element
values; none asserts an allocation count or a capacity.

`count = 0` allocating nothing is the other half of the same property and is
asserted: `an_empty_run_is_empty` covers it, though it checks the length rather
than the allocation.

---

### IX52 — The Header Outweighs the Payload at Every Size a Batch Would Ask For

**Finding.** A `Vec< SlotIndex >` is 24 bytes of header — pointer, length,
capacity — carrying 8 bytes per element. The ratio at the sizes that matter:

| `count` | Header | Heap payload | Header share |
|---------|--------|--------------|--------------|
| 0 | 24 | 0 | all of it |
| 1 | 24 | 8 | 75% |
| 4 | 24 | 32 | 43% |
| 8 | 24 | 64 | 27% |
| 1024 | 24 | 8192 | 0.3% |

The sizes a batch claim actually uses sit at the top of that table.
`ring_config`'s `batch` field defaults to `1`, and the doctests throughout the
family use runs of 2 to 8. At those sizes, more than a quarter of the structure
is bookkeeping about a heap block that holds a handful of eight-byte integers —
each of which could have been recomputed in under a nanosecond
([`non_functional_requirement/001`](../non_functional_requirement/001_what_the_fold_costs.md)
IX22).

The comparison that makes it concrete: `of( seq, capacity )` returns 8 bytes in a
register. `run( seq, 1, capacity )` returns 24 bytes on the stack pointing at an
8-byte heap allocation, to deliver the identical value. The structure is four
times the size of the thing it contains and requires an allocator round-trip to
produce it.

This is the layout-level statement of what
[`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md)
records at the API level: `ring_batch` needed runs of exactly this size, and
what it declined was not the arithmetic but this structure.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_index
command grep -F 'header outweighs its payload' src/lib.rs
```

Live output:

```
/// header outweighs its payload: one slot costs 24 bytes of bookkeeping to
```

**Disposition:** applied — `run`'s own doc comment now discloses the ratio
this finding measured: at the batch sizes the family actually uses (default
1, doctests 2 to 8), the returned `Vec`'s 24-byte header outweighs the 8
bytes per slot it carries.
Now prints: `header outweighs its payload`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/001`](001_the_ring_is_a_computation_not_a_layout.md) | Why this is the only structure — nothing else in the family stores a slot |
| [`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md) | The choice of container, undocumented |
| [`non_functional_requirement/002`](../non_functional_requirement/002_the_one_allocation_and_the_zero_callers.md) | The allocation counted per call |
| [`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md) | The consumer that declined this structure and kept the arithmetic |

### Sources

| Fact | Where |
|------|-------|
| The signature and the `collect` | `ring_index/src/lib.rs:118-122` |
| Exact sizing at 1 and 1024 | Standalone `rustc -O` probe, quoted above |
| One allocation per call at every size | Release probe, section 3, quoted above |
| The default batch size | `ring_config/src/lib.rs:65-77` |

### Tests

| Test | Covers |
|------|--------|
| `an_empty_run_is_empty` | `count = 0`, the one size that allocates nothing |
| `a_full_capacity_run_covers_every_slot_once` | Element values at `count == capacity` |
| `an_oversized_run_repeats_slots` | Element values above capacity, where the payload is largest |
| *(to create)* | An assertion that `len == capacity` after `run`, which is the property the body's shape provides |
