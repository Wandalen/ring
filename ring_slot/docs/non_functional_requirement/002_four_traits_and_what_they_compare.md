# Non-Functional Requirement: Four Traits and What They Compare

### Scope

**Purpose:** Record what the four traits on each shape cost, that `Clone` on a
`BytesSlot` copies the capacity rather than the payload, and that the only trait
generic code in the family actually requires is the fifth one — the hand-written
`Default`.

**Responsibility:** The trait obligations attached to each slot shape: what they
cost, who requires them, and which ones nobody does.

**In Scope:** the two `#[ derive ]` lines in `ring_slot/src/lib.rs`, the
three hand-written impls on `BytesSlot` that replace what the second one used to
carry, the two hand-written `Default` impls, and the family's bound census.
Addressed by name rather than by line — SL36's own fix moved every line number
this section used to carry.

**Out of Scope:** *What* equality compares — the tail question — is
[`pitfall/001`](../pitfall/001_the_test_that_names_a_property_the_type_lacks.md)
SL41 and SL42. This instance is about the traits as an obligation, not their
semantics. Per-operation cost is
[`non_functional_requirement/001`](001_what_a_slot_costs.md).

---

## The Four Traits, and Where Each One Comes From

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- derived ---'
command grep '#\[ derive' ring_slot/src/lib.rs
echo '--- written by hand for BytesSlot ---'
command grep -E '^impl< const N : usize > (core::fmt::Debug|PartialEq|Eq) for BytesSlot' ring_slot/src/lib.rs
```

Live output:

```
--- derived ---
#[ derive( Debug, Clone, PartialEq, Eq ) ]
// `#[ derive( Default ) ]` on a tuple struct emits `impl< T : Default >`,
#[ derive( Clone ) ]
// a generic `N`, so `#[ derive( Default ) ]` here does not compile at all.
--- written by hand for BytesSlot ---
impl< const N : usize > core::fmt::Debug for BytesSlot< N >
impl< const N : usize > PartialEq for BytesSlot< N >
impl< const N : usize > Eq for BytesSlot< N > {}
```

The same four traits on both shapes — `Debug`, `Clone`, `PartialEq`, `Eq` — with
no `Copy`, no `Hash`, no `Default` and no `PartialOrd` on either. What differs is
how three of them arrive: derived on `TypedSlot`, hand-written on `BytesSlot`,
because a derived version would read the array past `len`
([`pitfall/002`](../pitfall/002_clear_forgets_it_does_not_erase.md) SL44 records
the change and why). `Clone` is the one trait still derived on both, and the one
whose cost this instance prices.

---

### SL35 — `Clone` Copies `N`, Not `len`, and Is Unconditional on Only One Shape

A `BytesSlot`'s payload is inline, so cloning it copies the array whether or not
the array holds anything. Measured, release, on a `BytesSlot< 4096 >` holding one
byte:

```
--- what Clone copies ---
  len written                 = 1
  bytes moved by clone        = 4104
  ratio to the payload        = 4104x
```

Four kilobytes moved to duplicate one byte. That is not a defect in `Clone` — it
is the same inline-array property that
[`data_structure/001`](../data_structure/001_sixteen_bytes_to_carry_eight.md)
prices at rest, showing up in motion. A `TypedSlot< T >` has the opposite
profile: cloning it clones `Option< T >`, which for a heap payload is one
allocation and for `None` is nothing.

The two shapes also differ in *when* the derive applies:

```
--- what the derives are conditional on ---
  BytesSlot< 8 >  : Slot + Clone = true
  TypedSlot< u32 >: Slot + Clone = true
  TypedSlot< Opaque >: Slot      = true
  TypedSlot< Opaque >: Clone     = no  (the derive is conditional on T)
```

`derive` on `BytesSlot< N >` produces an unconditional impl — `N` is a const
parameter, so there is no type to bound. On `TypedSlot< T >` it produces
`impl< T : Clone > Clone`, so the shape carries the trait only for payloads that
carry it themselves.

**Finding.** A generic bound of `S : Slot + Clone` therefore admits every
`BytesSlot< N >` at a cost proportional to `N`, and only *some* `TypedSlot< T >`
at a cost proportional to the payload. A caller who adds the bound because a
`TypedSlot< u32 >` clone is cheap has, in the same stroke, accepted a
`BytesSlot< 4096 >` clone at four thousand times its content.

Nothing in the crate says so, and no test measures a clone at all:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c '\.clone()' ring_slot/tests/slot_test.rs
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
0
```

Zero callers in the crate's own suite. The derive is present, is free to write,
costs nothing when unused, and has never been exercised — so the asymmetry above
is a property of the types that no test would notice changing.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_slot
command grep -F 'actually written, so its cost is proportional to' src/lib.rs
```

Live output:

```
/// actually written, so its cost is proportional to `N`, not to `len`. A
```

**Disposition:** applied — `BytesSlot`'s own doc comment now says what this
finding found nothing saying: `Clone`'s cost is proportional to `N`, not to
`len`, and a `Slot + Clone` bound accepts this shape at that flat cost beside
`TypedSlot< T >`, whose clone tracks the payload and exists only when `T` is
itself `Clone`.
Now prints: `actually written, so its cost is proportional to`

---

### SL36 — Generic Code Requires Only the One Trait That Is Not Derived

The four traits on each shape look like the crate's interface obligation — the
more so now that three of them on `BytesSlot` are hand-written impls somebody
sat down and typed. They are not: nothing in the family requires any of them of
a slot, derived or written.

```sh
cd "$(git rev-parse --show-toplevel)"
for t in Clone PartialEq Eq Debug; do
  printf '  Slot + %-10s %d\n' "$t" \
    "$( grep -rcE "Slot \+ $t|$t \+ Slot" ring_*/src/*.rs 2>/dev/null | awk -F: '{s+=$2} END{print s+0}' )"
done
```

Live output:

```
  Slot + Clone      1
  Slot + PartialEq  0
  Slot + Eq         0
  Slot + Debug      0
```

What generic code does require is a fifth trait, and only that one:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rhoE 'Slot \+ [A-Za-z]+' ring_*/src/*.rs | sort | uniq -c
```

Live output:

```
      1 Slot + Clone
      4 Slot + Default
```

`ring_store`, `ring_mpsc`, and `ring_spsc` each need to fill a pre-allocated
array with empty slots, so each bounds on `Slot + Default`
([`integration/001`](../integration/001_seven_dependents_and_four_that_stay_generic.md)
records the same three crates as the generic tier). And `Default` is the one
trait the crate writes by hand:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^impl< T > Default for TypedSlot< T >$/,/^}$/p;/^impl< const N : usize > Default for BytesSlot< N >$/,/^}$/p' ring_slot/src/lib.rs
```

Live output:

```
impl< T > Default for TypedSlot< T >
{
  fn default() -> Self
  {
    Self::empty()
  }
}
impl< const N : usize > Default for BytesSlot< N >
{
  fn default() -> Self
  {
    Self::empty()
  }
}
```

`impl< T >`, not `impl< T : Default >`. That distinction is the whole reason the
impl is hand-written, and it is load-bearing: `#[ derive( Default ) ]` on a
`TypedSlot`-shaped tuple struct emits a `T : Default` bound even though
`Option< T >` needs none, so the derived version refuses a payload that has no
default of its own. The crate's own suite holds that against a payload that has
no `Default` — a test whose only job is to keep compiling:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A6 -F 'fn a_slot_is_default_for_a_payload_that_is_not()' ring_slot/tests/slot_test.rs
```

Live output:

```
fn a_slot_is_default_for_a_payload_that_is_not() {
    struct NotDefault(#[allow(dead_code)] u32);

    let slot: TypedSlot<NotDefault> = TypedSlot::default();
    assert!(slot.is_empty());
}
```

Substituting `#[ derive( Default ) ]` for the hand-written impl and re-running
`cargo nextest run -p ring_slot` produces `error[E0277]: the trait bound
NotDefault: Default is not satisfied` — a compile failure, not a test failure,
which is the stronger signal because no assertion can be adjusted to satisfy it.

**Finding.** The crate's real trait obligation is the invisible one. Four traits
are attached to each type where a reader sees them — at the top for `TypedSlot`,
and for `BytesSlot` one at the top and three written out below it — and no
consumer requires any of them; the trait three consumers *do* require appears a
hundred lines later as an ordinary impl, written that way to dodge a bound
`derive` would have added silently.

The consequence is a maintenance trap in one direction only. Replacing the
hand-written `Default` with `#[ derive( Default ) ]` — a tidying edit that looks
like it removes duplication — narrows `TypedSlot< T >` to payloads that
implement `Default`, and every ring in the family instantiates with a payload
that happens to have one, so nothing in the current tree would fail. The
regression would surface at the first consumer with a payload that does not, in
a crate that does not yet exist.

`BytesSlot< N >` is safe from the same edit for an unrelated reason: `[ u8; N ]`
has no `Default` for a generic `N` at all, so the derived version does not
compile rather than compiling narrower. One shape fails loudly and the other
fails later — and the comment explaining why neither is derived is absent from
both.

**Disposition:** applied — both, and the silent half is now noisy. Each
hand-written `Default` carries a comment stating its own reason: `TypedSlot`'s
says the derive "emits `impl< T : Default >`, because it defaults every field —
including the `Option< T >`, whose own `Default` is `None` and needs nothing from
`T`", names the three consumers the bound would propagate to (`ring_store`,
`ring_mpsc`, `ring_spsc`), and names the test that catches the edit;
`BytesSlot`'s records that its own derive "does not compile at all" and why, so a
reader is not left inferring that the two impls exist for one shared reason. The
test the first comment names is
`a_slot_is_default_for_a_payload_that_is_not`, above: `NotDefault` deliberately
has no `Default` impl, so the tidying edit this finding predicts stops the file
compiling instead of narrowing the type in silence. That closes the asymmetry the
finding's last sentence identifies — both shapes now fail loudly, one at its own
definition and one at its consumer's test. Falsified by substituting the derive
and re-running, which produced `the trait bound NotDefault: Default is not satisfied`.
Now prints: `let slot: TypedSlot<NotDefault> = TypedSlot::default();`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/001`](001_what_a_slot_costs.md) | The per-operation budget these obligations sit beside |
| [`pitfall/001`](../pitfall/001_the_test_that_names_a_property_the_type_lacks.md) | What `PartialEq` compares, which this instance defers |
| [`data_structure/001`](../data_structure/001_sixteen_bytes_to_carry_eight.md) | The inline array that makes a clone cost `N` |
| [`integration/001`](../integration/001_seven_dependents_and_four_that_stay_generic.md) | The three generic consumers, and what they bound on |
| [`api/002`](../api/002_a_trait_with_two_methods.md) | Why `Slot` declares two methods and no supertraits |

### Sources

Addressed by content rather than by line number — the census in this document is
its own subject, and acting on it moves every address it could otherwise cite.

| Fact | Where |
|------|-------|
| The two derive lines | `ring_slot/src/lib.rs` — `command grep '#\[ derive'` |
| `BytesSlot`'s three hand-written impls | `ring_slot/src/lib.rs` — `impl< const N : usize > … for BytesSlot< N >` for `core::fmt::Debug`, `PartialEq`, `Eq` |
| The two hand-written `Default` impls | `ring_slot/src/lib.rs` — `impl< T > Default for TypedSlot< T >`, `impl< const N : usize > Default for BytesSlot< N >` |
| No bound on any of the four traits | `ring_*/src/*.rs` — all four counts zero |
| The three `Slot + Default` bounds | `ring_store/src/lib.rs`, `ring_mpsc/src/lib.rs`, `ring_spsc/src/lib.rs` — `command grep -rn 'Slot + Default'` |
| No clone in the crate's suite | `ring_slot/tests/slot_test.rs` — no occurrence |
| Clone cost and conditionality | Release probe, quoted above |
| Derived-`Default` refusal | `a_slot_is_default_for_a_payload_that_is_not`, which stops compiling under the derive |

### Tests

| Test | Covers |
|------|--------|
| `a_fresh_typed_slot_is_empty` | `TypedSlot::< String >::default()` |
| `a_fresh_bytes_slot_is_empty` | `BytesSlot::< 8 >::default()` |
| `slots_compare_by_payload_not_by_tail` | `PartialEq` on `BytesSlot`, now hand-written — that two slots with different tails and the same payload compare equal |
| `a_cleared_slot_is_indistinguishable_from_a_fresh_one` | `Debug` and `PartialEq` together, over a slot whose tail holds a former payload |
| `a_typed_slot_holds_non_copy_payloads` | A payload with no `Copy`, exercising the conditional derives |
| `a_slot_is_default_for_a_payload_that_is_not` | A `TypedSlot< T >` where `T` has no `Default`, pinning the unconditional impl — a compile-time pin, so it asserts almost nothing at runtime by design |
| *(to create)* | A clone of a `BytesSlot`, asserting the tail travels with it — `Clone` is still derived and still copies all `N`, and nothing tests it |
