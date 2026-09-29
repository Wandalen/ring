# data_structure

`GatingSet` is the crate's only type. Both instances are about what its shape
commits to.

### Overview Table

| ID | Name | Subject |
|----|------|---------|
| 001 | [The Set That Cannot Grow](001_the_set_that_cannot_grow.md) | No `&mut self` anywhere; membership is fixed at construction, and the family's one production call site sets it to 1 |
| 002 | [Owning the Cursors Rather Than Borrowing Them](002_owning_the_cursors_rather_than_borrowing_them.md) | `Vec< PaddedCursor >`, its cache-line stride, and the aliasing the ownership prevents |

### The Two Halves of One Decision

`GatingSet` owns its cursors *and* fixes how many there are. 002 argues the first
half is right — a set that borrowed cursors could outlive them or hold fewer than
it was built for. 001 observes that the second half is a consequence nobody
appears to have chosen deliberately, and that the family currently never needs
more than one.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| GT11 | `GatingSet` | n/a — observation | It has no `&mut self` method; consumers can be neither added nor removed after construction |
| GT12 | The `Vec` | n/a — observation | `Vec::with_capacity` then `resize_with`, and nothing resizes it afterwards. A `Box< [ PaddedCursor ] >` would put the fixed length in the type and drop the capacity word; the `Vec` is a growable collection used for its allocation and never once for its growth |
| GT13 | `GatingSet::new` | n/a — coverage | Exactly one call site exists in the production source of all 33 crates, and it passes `consumers = 1`; every multi-consumer construction in the workspace is in a test |
| GT14 | The `Vec`'s stride | n/a — observation | The 64-byte stride is asserted, and it comes from `PaddedCursor`'s size rather than from anything this crate does — so the property that keeps consumers off each other's cache lines is inherited, and this crate's test of it is a test of `ring_cursor` |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two fields, and the allocation that fills the first --'
command grep -E '^  cursors :|^  capacity :|Vec::with_capacity|resize_with' ring_gating/src/lib.rs
echo '  -- methods taking &mut self --'
command grep -c '&mut self' ring_gating/src/lib.rs || true
echo '  -- control: a crate in the family that does take one --'
command grep -c '&mut self' ring_registry/src/lib.rs || true
```

Live output:

```
  -- the two fields, and the allocation that fills the first --
  cursors : Vec< PaddedCursor >,
  capacity : Capacity,
    let mut cursors = Vec::with_capacity( consumers );
    cursors.resize_with( consumers, PaddedCursor::default );
  -- methods taking &mut self --
0
  -- control: a crate in the family that does take one --
4
```

**A heap `Vec` fixed at construction, and no method that can change it.** The
zero is paired with a control using the identical expression, because a zero from
a mistyped pattern and a zero from a genuinely immutable type read the same.
