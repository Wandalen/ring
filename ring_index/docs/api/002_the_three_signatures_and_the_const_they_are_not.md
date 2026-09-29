# API: The Three Signatures and the `const` They Are Not

### Scope

**Purpose:** Record that `of` compiles unchanged as a `const fn` and is not
declared one, that `aliases` is kept from `const` by a single comparison it did
not have to write that way, and that this crate has the lowest `const` ratio of
any Tier 0/1 crate in the family.

**Responsibility:** What the three signatures commit to and what they give up —
`const`, the missing inverse, and the argument order they share.

**In Scope:** `ring_index/src/lib.rs:39-119`; every Tier 0/1 crate's
public function list.

**Out of Scope:** The surface as an inventory is
[`api/001`](001_three_functions_three_must_use_one_reached.md). What `run`'s
return type costs is
[`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md).

---

## Nothing Here Is `const`, and the Tier Is

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
for c in ring_types ring_config ring_slot ring_align ring_overflow ring_stats ring_seqno ring_index; do
  t=$( command grep -h '^\s*pub \(const \)\?fn' ring/$c/src/*.rs | wc -l )
  k=$( command grep -h '^\s*pub const fn' ring/$c/src/*.rs | wc -l )
  printf '  %-14s %2s of %2s public fns are const\n' "$c" "$k" "$t"
done
```

Live output:

```
  ring_types     12 of 12 public fns are const
  ring_config    11 of 12 public fns are const
  ring_slot       6 of 10 public fns are const
  ring_align      4 of  5 public fns are const
  ring_overflow   3 of  4 public fns are const
  ring_stats      2 of 16 public fns are const
  ring_seqno        0 of  5 public fns are const
  ring_index      0 of  3 public fns are const
```

Both operations this crate calls are `const fn` — `Capacity::mask` and
`Seq::advanced_by` — as is the accessor it declines to call, `SlotIndex::get`.
The crate that composes them is the one that is not.

---

### IX7 — `of` Compiles as a `const fn` Unchanged

Not "could be made const with work" — the same body, with the keyword added:

```
error-free; the following compiled under rustc --edition 2024:

  pub const fn of( seq : Seq, capacity : Capacity ) -> SlotIndex
  {
    SlotIndex( ( seq.0 as usize ) & capacity.mask() )
  }

  pub const CAP  : Capacity  = Capacity( 1024 );
  pub const SLOT : SlotIndex = of( Seq( 5000 ), CAP );
  const _ : () = assert!( SLOT.0 == 5000 % 1024 );
```

**Finding.** The `const _ : () = assert!( … )` is the part that matters: it is
evaluated at compile time, so the fold ran in the compiler and agreed with the
modulo it replaces. Nothing in `of`'s body needs the runtime — it is a mask over
two `Copy` values.

What that would buy is small but not nothing. A capacity fixed at a call site —
which is how every ring in the family is actually built, from a
`ring_config::Config` constructed with a literal — could have its slot indices
folded once by the compiler rather than on every call. More usefully, a caller
could write `const FIRST_SLOT : SlotIndex = of( Seq::ZERO, CAP );` and use it in
an array length or a `static`, which today is impossible.

The cost of the missing keyword is not performance. `of` optimizes to one `and`
regardless
([`non_functional_requirement/001`](../non_functional_requirement/001_what_the_fold_costs.md)
IX21 measures it at 0.9 cycles). The cost is that a `const` context cannot call
it at all, in a family where `ring_types` made all twelve of its functions
callable from one.

---

### IX8 — `aliases` Is Kept From `const` by an `==` That Had a `const` Alternative Waiting

`aliases` is two `of` calls and an `==`. The `==` is the obstruction:

```
the crate's own form, as a const fn:

  pub const fn aliases( a : Seq, b : Seq, capacity : Capacity ) -> bool
  { of( a, capacity ) == of( b, capacity ) }

  error[E0015]: cannot call non-const operator in constant functions
    note: impl defined here, but it is not `const`
          #[ derive( ..., PartialEq, Eq ) ] pub struct SlotIndex( pub usize );

through the accessor ring_types already provides, it compiles and const-evaluates:

  pub const fn aliases( a : Seq, b : Seq, capacity : Capacity ) -> bool
  { of( a, capacity ).get() == of( b, capacity ).get() }

  pub const ALIASED : bool = aliases( Seq( 5000 ), Seq( 6024 ), CAP );
  const _ : () = assert!( ALIASED );
  const _ : () = assert!( !aliases( Seq( 5000 ), Seq( 5001 ), CAP ) );
```

**Finding.** A `#[ derive( PartialEq ) ]` impl is not a `const` impl, so
`SlotIndex == SlotIndex` cannot appear in a `const fn`. Comparing the wrapped
`usize` values can — and `ring_types` already exports the `const` accessor that
does it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A10 -F '  /// The underlying offset.' ring_types/src/id.rs
```

Live output:

```
  /// The underlying offset.
  ///
  /// ```
  /// use ring_types::SlotIndex;
  /// assert_eq!( SlotIndex( 7 ).get(), 7 );
  /// ```
  #[ must_use ]
  pub const fn get( self ) -> usize
  {
    self.0
  }
```

So the obstruction is not the derive, and not `SlotIndex`, and not a missing
upstream affordance. `SlotIndex::get` exists, is `pub`, is `#[ must_use ]`, and
is `const` — every property this call site would need. The obstruction is that
`aliases` compares the wrapper instead, in a function whose entire body is that
one comparison.

This is the smaller half of IX7. `of` gives up `const` for no reason at all;
`aliases` gives it up for one call it did not make to a function written to be
called. `run` is the only one of the three with a real reason — it allocates.

---

### IX9 — There Is No Inverse, and That Is the Point

Every signature runs one way: `Seq` in, `SlotIndex` or `bool` out. Nothing takes
a `SlotIndex` as an argument.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -n 'SlotIndex' ring_index/src/lib.rs | command grep -v '^\s*[0-9]*://\|///' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
use ring_types::{ Capacity, Seq, SlotIndex };
pub fn of( seq : Seq, capacity : Capacity ) -> SlotIndex
  SlotIndex( ( seq.0 as usize ) & capacity.mask() )
pub fn run( start : Seq, count : usize, capacity : Capacity ) -> Vec< SlotIndex >
```

**Finding.** `SlotIndex` appears only as a return type and a constructor call.
There is no `fn sequences_addressing( slot : SlotIndex, capacity : Capacity )`,
and there cannot usefully be one: the fold is many-to-one, so the inverse of a
slot is an infinite arithmetic progression rather than a value.

The crate does not say this. `ring_types` does, at `id.rs:90` — a `SlotIndex` is
"Derived from a [`Seq`] by `ring_index`, never constructed by counting" — which
is the closest the family comes to stating that the arrow only points one way.
`aliases` is the readable consequence: the only question you can ask *about* two
sequences' slots is whether they are the same one, and the answer is a `bool`
rather than a distance.

The absence is correct and undocumented here, which is the whole finding.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/001`](001_three_functions_three_must_use_one_reached.md) | The same three functions, as an inventory and a reach census |
| [`type/001`](../type/001_three_types_borrowed_none_owned.md) | The types these signatures are written in |
| [`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md) | The one signature with a real reason not to be `const` |
| [`non_functional_requirement/001`](../non_functional_requirement/001_what_the_fold_costs.md) | Why the missing `const` costs nothing at runtime |

### Sources

| Fact | Where |
|------|-------|
| The three signatures | `ring_index/src/lib.rs:39-119` |
| `const` ratios across Tier 0/1 | Census above |
| `const fn of` compiling and const-evaluating | `rustc --edition 2024` probe, quoted above |
| `derive( PartialEq )` not being a `const` impl | Same probe, `E0015` |
| `SlotIndex::get` being `pub const fn` and `#[ must_use ]` | `ring_types/src/id.rs:109-113` |
| `aliases` via `.get()` compiling as `const` | Same probe, both assertions evaluating |
| `SlotIndex` is never an argument | Census above |
| "never constructed by counting" | `ring_types/src/id.rs:90` |

### Tests

| Test | Covers |
|------|--------|
| `derivation_is_a_mask` | That `of` is the mask — the property a `const` evaluation would let the compiler check |
| `aliasing_is_exactly_whole_laps` | `aliases`, at runtime only |
| *(to create)* | A `const` call of `of` — which today does not compile, so the test would be the change |
