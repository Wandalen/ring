# Workaround: The Mask That Lives One Crate Up

### Scope

**Purpose:** Record that this entire crate exists because `ring_types` publishes
`Capacity::mask` instead of publishing the fold itself, name what that costs, and
state the condition under which the crate deletes.

**Responsibility:** The external constraint — a public `mask()` on a type in
another crate — and what `ring_index` absorbs on its consumers' behalf as a
result.

**In Scope:** `ring_types/src/capacity.rs:40-78`;
`ring_index/Cargo.toml`; `ring_index/src/lib.rs:48-52`.

**Out of Scope:** the one-owner pattern this constraint undermines is
[`pattern/001`](../pattern/001_one_owner_for_one_arithmetic_fact.md). The
violation it permitted is
[`pitfall/002`](../pitfall/002_the_second_fold_nobody_noticed.md).

---

## The Constraint

`Capacity` has three methods. One constructs, one reads the slot count, and one
hands out the bitmask:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -E '^\s*pub (const )?fn ' ring_types/src/capacity.rs
echo '  -- and everything in the ring family that calls the third one --'
# scoped to ring_*: spatial_mask has an unrelated LayerMask::mask()
command grep -r '\.mask()' --include=*.rs ring_*/ | command grep -v '^ring_types/'
```

Live output:

```
  pub const fn new( slots : usize ) -> Result< Self, RingError >
  pub const fn get( self ) -> usize
  pub const fn mask( self ) -> usize
  -- and everything in the ring family that calls the third one --
ring_index/tests/index_test.rs:      assert_eq!( of( Seq( seq ), c ), SlotIndex( seq as usize & c.mask() ) );
ring_index/src/lib.rs:  SlotIndex( ( seq.0 as usize ) & capacity.mask() )
ring_mpsc/src/lib.rs:    let index = ( seq.0 as usize ) & self.capacity().mask();
```

Three call sites across 33 crates. One is `of`'s body, one is the test that
asserts `of` is a mask rather than a division, and one is `ring_mpsc`'s
hand-written fold.

---

### IX45 — The Crate Exists Because the Method Was Not Written

**Finding.** `ring_index` depends on exactly one crate, and every item its body
touches comes from that crate:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/\[dependencies\]/,/^\[lints\]/p' ring_index/Cargo.toml
command grep 'pub use' ring_types/src/lib.rs
command grep -m1 -B1 -A3 -F 'pub fn of( seq : Seq, capacity : Capacity ) -> SlotIndex' ring_index/src/lib.rs
```

Live output:

```
[dependencies]
ring_types = { path = "../ring_types" }

[lints]
pub use capacity::Capacity;
pub use error::RingError;
pub use id::{ Seq, SlotIndex };
pub use policy::{ OverflowPolicy, WaitKind };
#[ must_use ]
pub fn of( seq : Seq, capacity : Capacity ) -> SlotIndex
{
  SlotIndex( ( seq.0 as usize ) & capacity.mask() )
}
```

`Capacity`, `Seq` and `SlotIndex` are all re-exported from the same crate, two
lines apart. `of`'s body references nothing else. So the function could have been
written as `Capacity::slot_of( self, seq : Seq ) -> SlotIndex` inside
`ring_types` with no new dependency, no new type, and no change to its body — and
then `ring_index` would have nothing left to contain.

That is the constraint this crate absorbs: the fold's natural owner is the type
that carries the precondition the fold relies on, and that type is in another
crate that stopped at handing out the raw bitmask. `ring_index` is what fills the
gap.

The absorption is genuine and it works. Consumers get a named function with a
doctest, a `#[ must_use ]`, and 8,184 assertions behind it, rather than an
open-coded `&` on a number they fetched from an accessor.

---

### IX46 — What the Workaround Costs, and What Deletes It

**Finding.** The costs are all structural rather than runtime — `of` inlines to
one `and`, so the crate boundary is free at execution time. What it is not free
of:

- **A public `mask()` that must stay public.** `of` lives in a different crate,
  so the bitmask has to cross a crate boundary as public API. That is precisely
  the door `ring_mpsc/src/lib.rs:543` walked through. Had the fold lived on
  `Capacity`, `mask` could be private and that line would not compile.
- **Twelve lines of manifest and a dependency edge per consumer.** Every crate
  that folds must add `ring_index` to `Cargo.toml` and a `use` to its source.
  Three crates did; the fourth wrote the mask instead, and one of the reasons
  available to it was that adding a dependency is a visible cost and writing `&`
  is not.
- **A pattern that has to be stated in prose.** `ring_store`'s comment, this
  corpus, and `ring_batch`'s test all exist to assert something the type system
  would have asserted for free
  ([`pattern/001`](../pattern/001_one_owner_for_one_arithmetic_fact.md) IX41).

**Deletion condition.** Move `of` onto `Capacity` as `pub const fn slot_of`, make
`mask` private, and `ring_index` deletes: 122 lines of source, 188 lines of test,
one manifest, one workspace member. The 188 lines of test move with the function.
Every current consumer changes one `use` line and one call site, and
`ring_mpsc:543` stops compiling — which is the point.

Two things stand in the way and neither is technical. The family's tier scheme
gives `ring_index` a slot at Tier 1 that the workstream plan enumerates, and
this family's own feature record names the
crate rather than the behaviour
([`integration/002`](../integration/002_the_feature_it_implements_half_of.md)
IX13). The workaround is load-bearing for the documentation more than for the
code.

**Disposition:** declined — this file's own Tests table marks "(to create)" a
census assertion that `mask()` has no caller outside this crate, but the
census two sections above already shows one: `ring_mpsc/src/lib.rs:543`
calls `.mask()` today. Adding that assertion as stated would fail immediately
against real, current code in a crate outside this pass's assigned scope
(`ring_gating`, `ring_handle`, `ring_index`) — fixing it requires the same
`ring_mpsc` change IX12 and IX19 already name, not a change inside
`ring_index`. The Deletion Condition itself is the crate's own accepted
trade-off ("load-bearing for the documentation more than for the code"), not
a defect this pass fixes.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/002`](002_the_iterator_ring_batch_built_instead.md) | The other absorbed constraint, this one imposed by `ring_index` on a consumer |
| [`pattern/001`](../pattern/001_one_owner_for_one_arithmetic_fact.md) | The one-owner rule a public `mask()` makes unenforceable |
| [`pitfall/002`](../pitfall/002_the_second_fold_nobody_noticed.md) | What walked through the door |
| [`type/001`](../type/001_three_types_borrowed_none_owned.md) | Why `Capacity` is the fold's natural owner |

### Sources

| Fact | Where |
|------|-------|
| `Capacity`'s three methods | `ring_types/src/capacity.rs:40, 60, 75` |
| Three `mask()` call sites family-wide | Census above |
| One dependency, all items from it | `ring_index/Cargo.toml`; `ring_types/src/lib.rs:41-44` |
| The crate's total size | 122 source lines, 188 test lines |

### Tests

| Test | Covers |
|------|--------|
| `derivation_is_a_mask` | The one legitimate `mask()` call outside `of`, asserting the method |
| `non_power_of_two_capacity_is_rejected_upstream` | The precondition that would travel with the method if it moved |
| *(to create)* | A census assertion that `mask()` has no caller outside this crate |
