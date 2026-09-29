# Type: Three Variants and Two Questions

### Scope

**Purpose:** Record what each variant claims, what the two predicates ask, and the
fourth answer-pair the type has no variant for.

**Responsibility:** The three doc comments, the two `match` bodies, and the
truth table joining them.

**In Scope:** `pub enum Resolution`'s three variants and their doc comments,
and the bodies of `Resolution::lost_an_item` and `Resolution::accepted_incoming`,
all in `ring_overflow/src/lib.rs`.

**Out of Scope:** The derives are
[`type/001`](001_six_derives_on_a_fieldless_enum.md). The safety property forbidding
the fourth pair is
[`invariant/002`](../invariant/002_no_resolution_overwrites_unread_data_silently.md).

---

## What Each Variant Claims, and What Is Asked of It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the three variants, with what each doc comment claims --'
command grep -m1 -A9 -F 'pub enum Resolution' ring_overflow/src/lib.rs | tail -n 9
echo '  -- and the two questions asked of them --'
sed -n '/^  pub const fn lost_an_item( self ) -> bool$/,/^  }$/p;/^  pub const fn accepted_incoming( self ) -> bool$/,/^  }$/p' ring_overflow/src/lib.rs | command grep -E 'pub const fn |=> true,|=> false,'
```

Live output:

```
  -- the three variants, with what each doc comment claims --
{
  /// The incoming item was discarded; the ring's contents are unchanged.
  DroppedIncoming,
  /// The oldest unread item was discarded to make room; the incoming item was
  /// accepted.
  ///
  /// **No shipping configuration produces this.** It requires
  /// [`OverflowPolicy::DropOldest`], which the default build rejects at
  /// construction with `RingError::PolicyUnsupported`, and which the `crossbeam`
  -- and the two questions asked of them --
  pub const fn lost_an_item( self ) -> bool
      Self::DroppedIncoming | Self::EvictedOldest => true,
      Self::Refused => false,
  pub const fn accepted_incoming( self ) -> bool
      Self::EvictedOldest => true,
      Self::DroppedIncoming | Self::Refused => false,
```

---

### OV35 — The Two Predicates Jointly Identify the Variant, So the Type Is Two Bits With One Forbidden

Two boolean questions admit four answer pairs. Three of them are occupied:

| `lost_an_item` | `accepted_incoming` | variant |
|----------------|---------------------|---------|
| true | false | `DroppedIncoming` |
| true | true | `EvictedOldest` |
| false | false | `Refused` |
| false | true | *(none)* |

The mapping is injective, so the pair of booleans recovers the variant exactly —
no information is lost by reading through the predicates rather than matching.

**Finding.** The empty cell is not an oversight; it is physically impossible. A
full ring cannot take the incoming item without something leaving, so "accepted
without losing anything" describes no state a ring can be in. That impossibility is
precisely what
[`invariant/002`](../invariant/002_no_resolution_overwrites_unread_data_silently.md)
asserts, written as `accepted_incoming() ⟹ lost_an_item()` — the implication whose
only falsifying case is that fourth row.

So the crate's design argument sharpens. `Resolution` is presented as replacing a
boolean with named outcomes; what it actually replaces a boolean with is *two*
booleans and a constraint between them, and the named type is what makes the
constraint statable at all. Three variants is the smallest closed encoding of two
questions minus one forbidden answer, which is why there are three and not two or
four.

None of this is written down. The truth table above is reconstructible from four
lines of source and appears in no doc comment, so the reason the type has exactly
three variants reads as enumeration rather than as the consequence it is.

---

### OV36 — Two Variants Leave the Ring Unchanged and Differ Only in Who Holds the Item

`DroppedIncoming` says "the ring's contents are unchanged." `Refused` says "Nothing
was published and nothing discarded." Both describe a ring in the same state
afterwards: same contents, same occupancy, the publish did not happen.

They differ in one thing the ring cannot see — where the item went. Under
`DropNewest` the ring discarded it and it is gone. Under `Fail` the caller still
holds it and may retry, log it, or block.

**Finding.** `lost_an_item` is the predicate carrying that distinction, and it is
the only thing distinguishing the two outcomes at all. It is therefore the crate's
most load-bearing reading — not `accepted_incoming`, which merely picks out the
variant no production build can reach
([`pitfall/001`](../pitfall/001_the_variant_a_default_build_cannot_reach.md)).

The one production consumer does not call it. `ring_core` names all three
variants across two arms — `DroppedIncoming` alone, then `EvictedOldest` and
`Refused` together
([`algorithm/002`](../algorithm/002_where_the_third_outcome_goes.md)), which reaches
the same distinction by a different route and rebuilds by hand exactly what the
predicate exists to answer.

Worth stating plainly because the two doc comments are the clearest writing in the
crate and still do not say the thing that matters most about the pair: these two
outcomes are indistinguishable from the ring's side and opposite from the caller's.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/001`](001_six_derives_on_a_fieldless_enum.md) | What the type derives |
| [`invariant/002`](../invariant/002_no_resolution_overwrites_unread_data_silently.md) | The constraint the empty cell states |
| [`item/002`](../item/002_the_enum_and_its_two_readings.md) | Both predicates as items |
| [`algorithm/002`](../algorithm/002_where_the_third_outcome_goes.md) | The consumer rebuilding the distinction |

### Sources

| Fact | Where |
|------|-------|
| The three variant doc comments | `ring_overflow/src/lib.rs:58-75` |
| `lost_an_item`'s pattern | `ring_overflow/src/lib.rs:115-116` |
| `accepted_incoming`'s pattern | `ring_overflow/src/lib.rs:142-143` |
| The implication forbidding the fourth pair | `ring_overflow/tests/overflow_test.rs:89-124` |

### Tests

| Test | Covers |
|------|--------|
| `the_two_readings_partition_the_outcomes` | All six cells of the occupied rows |
| `no_resolution_overwrites_unread_data_silently` | That the fourth pair stays empty |
| `a_refusal_loses_nothing` | The `false, false` row |
| *(to create)* | Nothing asserts the pair is injective, though every existing assertion depends on it |
