# Invariant: The Two Emptiness Paths Agree

### Scope

**Purpose:** Record that `BytesSlot` answers "am I empty?" through two separate
functions, that they can never disagree because one delegates to the other, and
that the corresponding asymmetry on `TypedSlot` is undocumented.

**Responsibility:** The relationship between `BytesSlot::is_empty` (inherent) and
`< BytesSlot as Slot >::is_empty` (trait), and the absence of the inherent half
on `TypedSlot`.

**In Scope:** `ring_slot/src/lib.rs:309-326, 385-396, 176-187`;
`ring_slot/tests/slot_test.rs:339-360`.

**Out of Scope:** What emptiness *means* for a `BytesSlot` — the zero-length
conflation — is [`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md)
SL8. This instance is about the two paths agreeing, not about what they agree on.

---

## Two Functions, One Answer

```sh
cd "$(git rev-parse --show-toplevel)"
# the inherent one, with the reason it exists
command grep -m1 -A17 -F '  /// Whether nothing has been written.' ring_slot/src/lib.rs
echo '--- and the trait impl that delegates to it ---'
command grep -m1 -A11 -F 'impl< const N : usize > Slot for BytesSlot< N >' ring_slot/src/lib.rs
```

Live output:

```
  /// Whether nothing has been written.
  ///
  /// Duplicates [`Slot::is_empty`] deliberately: a caller holding a concrete
  /// `BytesSlot` should not need the trait in scope to ask, and the trait impl
  /// below delegates here so the two can never disagree.
  ///
  /// ```
  /// use ring_slot::BytesSlot;
  /// let mut s = BytesSlot::< 8 >::empty();
  /// assert!( s.is_empty() );
  /// s.write( b"a" ).unwrap();
  /// assert!( !s.is_empty() );
  /// ```
  #[ must_use ]
  pub const fn is_empty( &self ) -> bool
  {
    self.len == 0
  }
--- and the trait impl that delegates to it ---
impl< const N : usize > Slot for BytesSlot< N >
{
  fn is_empty( &self ) -> bool
  {
    Self::is_empty( self )
  }

  fn clear( &mut self )
  {
    self.len = 0;
  }
}
```

---

### SL11 — The Agreement Is Structural, and the Test Knows Why That Still Needs Asserting

`Self::is_empty( self )` is not a re-implementation — it is a call. There is one
comparison in the crate that decides `BytesSlot` emptiness, and both paths reach
it. Disagreement is not merely untested; it is unrepresentable without editing
the delegation.

The suite asserts it anyway, and says why:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A21 -F '/// `BytesSlot` carries an inherent `is_empty` alongside the trait'"'"'s, so a caller' ring_slot/tests/slot_test.rs
```

Live output:

```
/// `BytesSlot` carries an inherent `is_empty` alongside the trait's, so a caller
/// holding the concrete type need not import `Slot`. The two must never
/// disagree — this is the only test that can catch it, since every other call
/// site resolves to whichever one is in scope.
#[ test ]
fn the_inherent_and_trait_emptiness_agree()
{
  let mut slot = BytesSlot::< 4 >::empty();
  for payload in [ &b""[ .. ], b"a", b"abcd", b"" ]
  {
    slot.write( payload ).unwrap();
    assert_eq!
    (
      BytesSlot::is_empty( &slot ), Slot::is_empty( &slot ),
      "payload {payload:?} reads differently through the two paths"
    );
  }

  slot.write( b"ab" ).unwrap();
  slot.clear();
  assert_eq!( BytesSlot::is_empty( &slot ), Slot::is_empty( &slot ) );
}
```

**Finding.** The test's own doc comment states the reason it exists: every
ordinary call site resolves to whichever `is_empty` is in scope, so no ordinary
test can distinguish them. The only way to compare the two is to name both
explicitly with fully-qualified syntax, which this test does at five payloads
plus the post-`clear` state.

That is the correct treatment of a delegation invariant: the delegation makes
disagreement structurally impossible *today*, and the test is what keeps a future
edit from replacing the delegation with a duplicated comparison. It costs one
test and it is the only guard against a class of bug that would otherwise be
invisible.

---

### SL12 — `TypedSlot` Has No Inherent `is_empty`, and Nothing Says So

`BytesSlot`'s doc gives a reason for the duplication: "a caller holding a
concrete `BytesSlot` should not need the trait in scope to ask." That reason
applies verbatim to `TypedSlot`, which does not have one:

```sh
cd "$(git rev-parse --show-toplevel)"
echo 'TypedSlot inherent methods:'
command grep -m1 -A72 -F 'impl< T > TypedSlot< T >' ring_slot/src/lib.rs | grep -E '^\s*pub (const )?fn '
echo 'TypedSlot trait impl:'
command grep -m1 -A11 -F 'impl< T > Slot for TypedSlot< T >' ring_slot/src/lib.rs
```

Live output:

```
TypedSlot inherent methods:
  pub const fn empty() -> Self
  pub fn set( &mut self, value : T ) -> Option< T >
  pub const fn get( &self ) -> Option< &T >
  pub fn take( &mut self ) -> Option< T >
TypedSlot trait impl:
impl< T > Slot for TypedSlot< T >
{
  fn is_empty( &self ) -> bool
  {
    self.0.is_none()
  }

  fn clear( &mut self )
  {
    self.0 = None;
  }
}
```

Four inherent methods, none of them `is_empty`. Emptiness on a `TypedSlot` is
reachable only through the trait — so `use ring_slot::Slot;` is mandatory for a
caller who holds a concrete `TypedSlot` and wants to ask, and optional for one who
holds a concrete `BytesSlot`.

The suite's very first test relies on the trait being in scope for exactly this
reason:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -B1 -A6 -F 'fn a_fresh_typed_slot_is_empty()' ring_slot/tests/slot_test.rs
```

Live output:

```
#[ test ]
fn a_fresh_typed_slot_is_empty()
{
  let slot = TypedSlot::< u32 >::empty();
  assert!( slot.is_empty() );
  assert_eq!( slot.get(), None );
  assert!( TypedSlot::< String >::default().is_empty() );
}
```

`slot.is_empty()` there resolves to `Slot::is_empty` — the file imports the
trait at line 17.

**Finding.** The two shapes present different import requirements for the same
question, and the crate documents the requirement for one of them only.
`BytesSlot::is_empty` explains at length why it duplicates the trait method;
`TypedSlot` offers no counterpart and no note saying the trait is required.

Neither behaviour is wrong — `TypedSlot`'s emptiness is one `Option::is_none`
call and a caller can always write `slot.get().is_none()` instead. What is
missing is the sentence: a reader who learns from `BytesSlot`'s doc that the
crate deliberately duplicates emptiness for concrete-type ergonomics will
reasonably expect the same on `TypedSlot`, and finds a trait-import error
instead. Adding an inherent `is_empty` to `TypedSlot` would make the two shapes
symmetric and would extend SL11's delegation invariant to cover it; declining to
add one is equally fine and wants one line saying so.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/001`](001_a_read_returns_what_was_written.md) | The crate's other property, and the exhaustive test that establishes it |
| [`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md) | What emptiness means for a `BytesSlot`, and the distinction it cannot make |
| [`api/001`](../api/001_ten_functions_six_const_seven_must_use.md) | The full surface, where the asymmetry is visible as a count |
| [`item/001`](../item/001_the_four_of_a_typed_slot.md) | `TypedSlot`'s four inherent methods, one at a time |

### Sources

| Fact | Where |
|------|-------|
| The inherent `is_empty` and its justification | `ring_slot/src/lib.rs:309-326` |
| The delegating trait impl | `ring_slot/src/lib.rs:385-396` |
| `TypedSlot`'s four inherent methods | `ring_slot/src/lib.rs:86-158` |
| `TypedSlot`'s trait impl | `ring_slot/src/lib.rs:176-187` |
| The trait import the suite relies on | `ring_slot/tests/slot_test.rs:17` |

### Tests

| Test | Covers |
|------|--------|
| `the_inherent_and_trait_emptiness_agree` | Both paths at five payloads plus post-`clear`, fully qualified |
| `a_fresh_typed_slot_is_empty` | `TypedSlot` emptiness — necessarily through the trait |
| `the_trait_reports_the_same_cycle_for_both_shapes` | Both shapes' emptiness across empty/filled/cleared |
| *(to create)* | An inherent `TypedSlot::is_empty`, or a doc line stating the trait is required |
