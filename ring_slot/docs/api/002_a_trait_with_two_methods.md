# API: A Trait With Two Methods

### Scope

**Purpose:** Record what the `Slot` trait declares, what it deliberately does
not, where the generic path it enables stops, and the second bound three of its
four users must add to make it usable at all.

**Responsibility:** The `Slot` trait's two methods, the four bound sites in the
family, and the `Default` impls that fill the gap the trait leaves.

**In Scope:** `ring_slot/src/lib.rs:38-45, 168-174, 377-383`;
`ring_store/src/lib.rs:96`; `ring_event/src/lib.rs:231`;
`ring_mpsc/src/lib.rs:355, 876`; `ring_spsc/src/lib.rs:284`.

**Out of Scope:** The ten inherent `pub fn` and their attributes are
[`api/001`](001_ten_functions_six_const_seven_must_use.md) — none of the counts
there include these two. That the two `is_empty` paths agree is
[`invariant/002`](../invariant/002_the_two_emptiness_paths_agree.md).

---

## The Whole Trait

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A7 -F 'pub trait Slot' ring_slot/src/lib.rs
```

Live output:

```
pub trait Slot
{
  /// Whether this slot currently holds nothing.
  fn is_empty( &self ) -> bool;

  /// Return the slot to its empty state.
  ///
  /// **Not a promise to overwrite.** For a shape that owns what it stores
```

Two methods. No constructor, no `set`, no `get`, no `write`, no `read`, no
associated type. Everything that touches a payload stayed inherent on the two
concrete shapes.

---

### SL19 — Neither Method Moves a Value, So the Generic Path Ends at the Slot Boundary

`is_empty` reads a `bool` out; `clear` throws a payload away. Neither puts one
in and neither hands one back. That is the whole expressive budget generic code
over `Slot` has, and it is enough for exactly one function in the family:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rE ': Slot( |,|$)' ring_*/src/*.rs | grep -vE ':[[:space:]]*///?' | sort
```

Live output:

```
ring_store/src/lib.rs:impl< S : Slot + Default > Buffer< S >
ring_event/src/lib.rs:  S : Slot,
ring_mpsc/src/lib.rs:impl< S : Slot + Default > Ring< S >
ring_spsc/src/lib.rs:impl< S : Slot + Default > Ring< S >
```

Four bound sites; one of them takes `Slot` alone. That one is:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F 'pub fn recycle< S >( slot : &mut S )' ring_event/src/lib.rs
```

Live output:

```
pub fn recycle< S >( slot : &mut S )
where
  S : Slot,
{
  slot.clear();
}
```

A one-line delegation to one of the trait's two methods. That is the entire
family's use of the unadorned bound.

Where a value actually has to move, the generic path is abandoned:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'impl< .a, T > Producer' ring_mpsc/src/lib.rs
command grep -m1 -A4 -F '  pub fn push( &self, value : T ) -> Result< Seq, RingError >' ring_mpsc/src/lib.rs
```

Live output:

```
impl< 'a, T > Producer< 'a, TypedSlot< T > >
  pub fn push( &self, value : T ) -> Result< Seq, RingError >
  {
    let mut reserved = self.claim()?;
    let seq = reserved.sequence();
    // The displaced value is dropped, deliberately, and this is not the same
```

`ring_mpsc::Ring< S >` is generic over `Slot` for its whole lifecycle — claim,
publish, drain, gate — and then `push`, the one method that carries a payload,
lives in a second impl block nailed to `TypedSlot< T >`. The `reserved.set( value )`
on the last line is `TypedSlot`'s inherent method, not a trait call — there is no
trait method it could be.

**Finding.** The trait draws the genericity boundary exactly at the point where a
payload would have to be described, and that is the right place to draw it. A
`Slot` trait with `set`/`get` would need an associated payload type, and the two
shapes disagree about what that is — `TypedSlot< T >` moves a `T` by value,
`BytesSlot< N >` copies a `&[ u8 ]` and returns a borrow with a lifetime. Unifying
them means a GAT and a lifetime parameter on the trait, and every downstream
`impl< S : Slot >` pays for it.

Instead the ring's *machinery* is generic and the ring's *payload transfer* is
concrete, and the split is one impl block wide. The cost is visible and bounded:
one extra impl block per crate that offers a convenience push, which today is
`ring_mpsc` alone. The observation worth recording is that this is a deliberate
ceiling, not an omission — but nothing in the trait's documentation says where
the ceiling is, so a reader adding a third slot shape learns it from a compile
error in `ring_mpsc` rather than from `Slot`.

---

### SL20 — Three of Four Bounds Add `Default` Because the Trait Has No Constructor, and That Costs `const`

Both shapes have an `empty()`, both are `const`, and neither is on the trait. So
generic code cannot construct a slot, and the three bounds that need to
construct one add `Default`:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- what generic construction looks like ---'
command grep -m1 -A4 -F '  pub fn new( capacity : Capacity ) -> Self' ring_store/src/lib.rs
echo '--- and the impls it reaches ---'
sed -n '/^impl< T > Default for TypedSlot< T >$/,/^}$/p;/^impl< const N : usize > Default for BytesSlot< N >$/,/^}$/p' ring_slot/src/lib.rs
```

Live output:

```
--- what generic construction looks like ---
  pub fn new( capacity : Capacity ) -> Self
  {
    let mut slots = Vec::with_capacity( capacity.get() );
    slots.resize_with( capacity.get(), S::default );
    Self { slots : slots.into_boxed_slice(), capacity }
--- and the impls it reaches ---
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

Both impls are hand-written, and both are one line. That they are hand-written
rather than derived is the load-bearing detail: `#[ derive( Default ) ]` on
`TypedSlot< T >( Option< T > )` would generate `impl< T : Default > Default`,
and the bound would propagate to every `Buffer< TypedSlot< T > >` in the family.
The hand-written impl carries no bound on `T` at all.

Measured — a payload type with no `Default` of its own, and both constructors in
`const` position:

```
TypedSlot< NoDefault >::default() is empty: true
TypedSlot::empty() in const position: ok, 1 bytes
BytesSlot::empty() in const position: ok, 16 bytes
```

And the price, from the same probe with `default()` substituted:

```
error[E0015]: cannot call non-const associated function `<TypedSlot<u32> as Default>::default` in constants
4 |   const EMPTY : TypedSlot< u32 > = TypedSlot::default();
  = note: calls in constants are limited to constant functions, tuple structs and tuple variants
```

**Finding.** The trait's missing constructor is paid for twice over, and the two
payments pull in opposite directions.

The first payment is deliberate and well made: hand-writing `Default` as
`Self::empty()` buys unbounded `T`, so a `Buffer< TypedSlot< T > >` works for any
payload rather than only for defaultable ones. A derive would have been shorter
and strictly worse, and nothing in either impl says so — the two four-line
bodies look like boilerplate a derive would have written, and they are precisely
what a derive could not.

The second is a genuine loss. `Default::default` is not and cannot be `const`, so
every generic construction path gives up the compile-time initialisation that
`empty()` offers. `ring_store::new` allocates at runtime regardless, so nothing
is lost there today; a future `[ S; K ]` in static position would find the door
shut, and the fix would be an associated `const EMPTY : Self` on the trait rather
than a method.

The narrower observation: the trait declares `clear`, which returns a slot to the
state `empty()` produces, but not `empty()` itself. It can restore the initial
state and cannot create it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/001`](001_ten_functions_six_const_seven_must_use.md) | The ten inherent functions, which these two are not among |
| [`pattern/001`](../pattern/001_one_trait_two_shapes.md) | Why one trait over two shapes is the family's shape |
| [`type/001`](../type/001_two_shapes_one_trait_no_copy.md) | What the trait's absence of `Copy` costs downstream |
| [`invariant/002`](../invariant/002_the_two_emptiness_paths_agree.md) | The trait's `is_empty` against `BytesSlot`'s inherent one |
| [`integration/001`](../integration/001_seven_dependents_and_four_that_stay_generic.md) | The four crates that stay generic, and where each stops |

### Sources

| Fact | Where |
|------|-------|
| The two-method trait | `ring_slot/src/lib.rs:38-45` |
| The four bound sites | `ring_store/src/lib.rs:96`; `ring_event/src/lib.rs:231`; `ring_mpsc/src/lib.rs:355`; `ring_spsc/src/lib.rs:284` |
| The only bare-`Slot` function | `ring_event/src/lib.rs:229-234` |
| Where the generic path stops | `ring_mpsc/src/lib.rs:876, 903-907` |
| Generic construction via `S::default` | `ring_store/src/lib.rs:88-93` |
| The two hand-written `Default` impls | `ring_slot/src/lib.rs:168-174, 377-383` |
| Unbounded `T`, and `const` vs `default()` | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `both_shapes_drive_through_the_trait_alone` | That the two methods suffice for a shape-agnostic caller |
| `the_trait_reports_the_same_cycle_for_both_shapes` | `is_empty` and `clear` composing to the same three-state cycle |
| `a_typed_slot_holds_non_copy_payloads` | A `T` the unbounded `Default` impl admits |
| `ring_store` — `the_same_buffer_type_serves_both_slot_shapes` | The `S : Slot + Default` bound satisfied by both shapes |
| *(to create)* | A `TypedSlot< T >` for a `T` with no `Default`, asserting the bound really is absent |
