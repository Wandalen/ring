# Item: The Fold Itself

### Scope

**Purpose:** Read `of` as a single code item — its doc comment, its doctest, its
attribute, its body — and record that the comment states the aliasing property
one lap narrower than it is.

**Responsibility:** `ring_index/src/lib.rs:32-52` as a unit: what the item
says about itself and whether the item does that.

**In Scope:** `of`'s doc comment, doctest, and body.

**Out of Scope:** the identity as a property is
[`invariant/001`](../invariant/001_the_mask_equals_the_modulo.md). The other two
functions are [`item/002`](002_the_two_that_nothing_calls.md).

---

## The Item, Whole

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A20 -F '/// The slot a sequence addresses.' ring_index/src/lib.rs
```

Live output:

```
/// The slot a sequence addresses.
///
/// Equal to `seq % capacity` for every input, computed as `seq & (capacity - 1)`.
/// Two sequences a whole number of laps apart return the same slot, which is
/// the aliasing the gating machinery exists to prevent from being *observed*.
///
/// ```
/// use ring_types::{ Capacity, Seq, SlotIndex };
/// use ring_index::of;
///
/// let cap = Capacity::new( 8 ).unwrap();
/// assert_eq!( of( Seq( 0 ), cap ), SlotIndex( 0 ) );
/// assert_eq!( of( Seq( 7 ), cap ), SlotIndex( 7 ) );
/// assert_eq!( of( Seq( 8 ), cap ), SlotIndex( 0 ) );   // one lap on
/// assert_eq!( of( Seq( 13 ), cap ), SlotIndex( 5 ) );
/// ```
#[ must_use ]
pub fn of( seq : Seq, capacity : Capacity ) -> SlotIndex
{
  SlotIndex( ( seq.0 as usize ) & capacity.mask() )
}
```

Twenty-one lines, of which one is the function. Sixteen are documentation and
four of those are executable.

---

### IX33 — "Exactly One Lap Apart" Is One Lap Narrower Than the Property

**Finding.** The comment says two sequences "exactly one lap apart return the
same slot." That is true and it is not the property. The property is that two
sequences *any whole number of laps apart* return the same slot — at capacity 8,
`0` and `8` alias, and so do `0` and `16`, and `0` and `800`.

The word "exactly" makes the narrow reading the natural one. A reader who takes
the comment at face value would expect `of( Seq( 16 ), cap 8 )` to differ from
`of( Seq( 0 ), cap 8 )`, and would be wrong. The doctest reinforces the narrow
reading rather than correcting it: its four assertions cover `0`, `7`, `8` and
`13`, so the largest gap demonstrated is exactly one lap, annotated
`// one lap on`.

The correct statement is one crate-local hop away and already written. `aliases`,
forty lines below, says "true exactly when they are a whole number of laps
apart" — the general form, stated precisely, in the function whose entire job is
to report this relation. Two doc comments about the same property, one general
and one narrow, and the narrow one is on the function everybody calls.

What makes this worth recording rather than nitpicking is where the narrow
reading leads. The sentence continues: "which is the aliasing the gating
machinery exists to prevent from being *observed*." A reader who thinks aliasing
means one-lap collisions will think the gate needs to prevent a producer getting
one lap ahead. That is in fact what the gate does — but because *any* lap is a
collision, not because one lap is special, and the difference matters for anyone
reasoning about a gate that has fallen behind by more than one.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_index
command grep -F 'a whole number of laps apart return the same slot' src/lib.rs
```

Live output:

```
/// Two sequences a whole number of laps apart return the same slot, which is
```

**Disposition:** applied — `of`'s doc comment no longer narrows the property to
"exactly one lap apart"; it now states the same general form `aliases` already
used ("a whole number of laps apart"), so the two comments agree and a reader
following either one reaches the same conclusion about a gate that has fallen
behind by more than one lap.
Now prints: `a whole number of laps apart return the same slot`

---

### IX34 — The Body Is Shorter Than Every Other Part of the Item

`of` is one expression. Its doc comment is fifteen lines, its doctest four
assertions, its attribute one line. The ratio holds across the crate:

```sh
cd "$(git rev-parse --show-toplevel)"
t=$( wc -l < ring_index/src/lib.rs )
d=$( command grep -c '^\s*//[/!]' ring_index/src/lib.rs )
b=$( command grep -c '^\s*$' ring_index/src/lib.rs )
printf '  total %s / doc comment %s / blank %s / code %s\n' "$t" "$d" "$b" "$(( t - d - b ))"
printf '  tests: %s lines, %s test fns\n' "$( wc -l < ring_index/tests/index_test.rs )" \
  "$( command grep -c '^#\[ test \]' ring_index/tests/index_test.rs )"
```

Live output:

```
  total 122 / doc comment 100 / blank 5 / code 17
  tests: 188 lines, 12 test fns
```

**Finding.** Seventeen lines of code carry 100 lines of doc comment and 188 lines
of test — a ~1:6 doc ratio and a ~1:11 test ratio. That is the right shape for a
crate whose entire value is a correctness claim about one arithmetic identity:
there is nothing to read in the implementation, so everything a maintainer needs
has to be in the prose and the assertions.

It also means the doc comments carry unusual weight. Every finding in this
corpus that concerns `ring_index` itself — IX33 above, IX31's totality scope,
IX26's justification, IX27's silent return-type decision — is a finding about
those 100 lines rather than about the 17. In a crate with this ratio, a wrong
sentence is a defect of the same order as a wrong expression, because the
sentence is what a caller acts on.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/002`](002_the_two_that_nothing_calls.md) | The other two items, including the one whose comment gets this property right |
| [`algorithm/001`](../algorithm/001_one_and_of_a_mask.md) | The body, and why it is total |
| [`invariant/001`](../invariant/001_the_mask_equals_the_modulo.md) | The identity the comment's first line states correctly |
| [`api/001`](../api/001_three_functions_three_must_use_one_reached.md) | The item as part of a surface |

### Sources

| Fact | Where |
|------|-------|
| The whole item | `ring_index/src/lib.rs:32-52` |
| The general form of the property | `ring_index/src/lib.rs:54-55` |
| Line census | Census above |

### Tests

| Test | Covers |
|------|--------|
| `of`'s own doctest | Four inputs, largest gap one lap |
| `aliasing_is_exactly_whole_laps` | The general property the comment narrows |
| `mask_equals_modulo_over_four_laps_of_every_capacity` | Four laps, which is where the narrow reading would fail |
