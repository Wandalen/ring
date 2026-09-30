# Pitfall: The Second Fold Nobody Noticed

### Scope

**Purpose:** Record why the duplicate fold in `ring_mpsc` is a hazard rather
than a style complaint — it masks with a capacity read from one field and
indexes storage sized by another, a coupling the one-owner version does not
have.

**Responsibility:** `ring_mpsc::Ring::stamp` as an implementation of this
crate's fold, and what its provenance differs from.

**In Scope:** `ring_mpsc/src/lib.rs:369-382, 426-430, 540-547`;
`ring_store/src/lib.rs:59-63, 87-93`.

**Out of Scope:** the topology fact — which crates depend on which — is
[`integration/001`](../integration/001_two_dependents_and_a_third_that_did_it_again.md)
IX12. The boundary the duplicate violates is
[`pattern/001`](../pattern/001_one_owner_for_one_arithmetic_fact.md).

---

## The Two Folds Side by Side

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -r '( seq.0 as usize ) &' ring_*/src/*.rs
```

Live output:

```
ring_index/src/lib.rs:  SlotIndex( ( seq.0 as usize ) & capacity.mask() )
ring_mpsc/src/lib.rs:    let index = ( seq.0 as usize ) & self.capacity().mask();
```

Identical arithmetic. The difference is entirely in where `capacity` comes from
and what the result is used to index.

---

### IX19 — The Mask and the Storage Read Their Length From Different Fields

`ring_mpsc::Ring` stores the capacity three times, in three places, and the
duplicate fold crosses between two of them:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what new() builds --'
command grep -m1 -A10 -F '    let stamps = ( 0 .. capacity.get() )' ring_mpsc/src/lib.rs
echo '  -- what capacity() returns --'
command grep -m1 -A3 -F '  pub fn capacity( &self ) -> Capacity' ring_mpsc/src/lib.rs
echo '  -- what stamp() does with it --'
command grep -m1 -A6 -F '  fn stamp( &self, seq : Seq ) -> &AtomicSeq' ring_mpsc/src/lib.rs
```

Live output:

```
  -- what new() builds --
    let stamps = ( 0 .. capacity.get() )
      .map( | _ | AtomicSeq::new( UNSTAMPED ) )
      .collect::< Vec< _ > >()
      .into_boxed_slice();

    Self
    {
      slots : Buffer::new( capacity ),
      stamps,
      consumers : GatingSet::new( capacity, 1 ),
    }
  -- what capacity() returns --
  pub fn capacity( &self ) -> Capacity
  {
    self.consumers.capacity()
  }
  -- what stamp() does with it --
  fn stamp( &self, seq : Seq ) -> &AtomicSeq
  {
    let index = ( seq.0 as usize ) & self.capacity().mask();
    // The mask is `capacity - 1` for a power-of-two capacity, which `Capacity`
    // enforces at construction, so the index is always in range.
    &self.stamps[ index ]
  }
```

**Finding.** Follow the two halves of `stamps[ index ]`:

- `index` is masked with `self.capacity()`, which is `self.consumers.capacity()`
  — the `GatingSet`'s copy.
- `stamps` was sized by `capacity.get()`, the constructor's parameter, stored
  nowhere and never consulted again.

They agree because `new` happens to pass the same `Capacity` to
`GatingSet::new`, to `Buffer::new`, and to the `stamps` loop, three lines apart.
Nothing enforces that. The comment above the index asserts in-range-ness from
`Capacity`'s power-of-two guarantee, which is the wrong guarantee: a
power-of-two mask keeps the index below *that* capacity, and says nothing about
`stamps.len()`.

The bound that actually matters — `stamps.len() == consumers.capacity().get()`
— is an invariant of the constructor, held in no type, checked by no assertion,
and stated in no comment. If a future constructor sized `stamps` from a config
field while `GatingSet` took its capacity from another, `stamps[ index ]` would
panic on a slice bound with the comment above it still claiming the index is
always in range.

Compare the version that owns the fold:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^pub struct Buffer< S >$/,/^}$/p;/^  pub fn new( capacity : Capacity ) -> Self$/,/^  }$/p' ring_store/src/lib.rs
```

Live output:

```
pub struct Buffer< S >
{
  slots : Box< [ S ] >,
  capacity : Capacity,
}
  pub fn new( capacity : Capacity ) -> Self
  {
    let mut slots = Vec::with_capacity( capacity.get() );
    slots.resize_with( capacity.get(), S::default );
    Self { slots : slots.into_boxed_slice(), capacity }
  }
```

`Buffer` keeps `slots` and the `capacity` that sized it in one struct, set by
one constructor from one argument. `Buffer::at` folds with `self.capacity` and
indexes `self.slots`. There is one number, so there is nothing to drift.

That is the actual content of the one-owner rule, and it is not "avoid typing
the same expression twice." Calling `ring_index::of` would not by itself fix
`ring_mpsc` — the fold would still mask with `consumers`' capacity and index
`stamps`. What fixes it is holding the mask and the storage together, which is
what `Buffer` does and what `ring_mpsc` opted out of by keeping `stamps`
outside its `Buffer`.

**Disposition:** declined — the finding's own text is explicit that calling
`ring_index::of` would not fix this by itself; the actual fix (holding
`stamps` and its sizing `Capacity` in one struct, the way `Buffer` does) is a
data-layout change to `ring_mpsc/src/lib.rs`, a crate outside this
pass's assigned scope (`ring_gating`, `ring_handle`, `ring_index`). Nothing in
`ring_index`'s own `src/` or `docs/` can restructure a different crate's
struct.

---

### IX20 — The Only Assertion of the Coupling Is Three Literals in Three Tests

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'stamps().len()' ring_mpsc/src/lib.rs ring_mpsc/tests/*.rs
```

Live output:

```
ring_mpsc/src/lib.rs:/// assert_eq!(ring.stamps().len(), 4, "one stamp per slot, not one per lap");
ring_mpsc/tests/mpsc_test.rs:        assert_eq!(ring.stamps().len(), 16);
ring_mpsc/tests/mpsc_test.rs:        assert_eq!(ring.stamps().len(), 8);
```

**Finding.** Three assertions, each comparing `stamps().len()` against a
hard-coded integer that the same test used to build the ring. None of them
compares it against `ring.capacity()`, which is the field the fold actually
masks with — so all three would still pass if `capacity()` began returning
something else.

The doc-test at `:306` is the closest to stating the invariant, and its message
— "one stamp per slot, not one per lap" — names the right property while
asserting it against `4` rather than against the ring's own capacity. Rewriting
that one line as `assert_eq!( ring.stamps().len(), ring.capacity().get() )`
would turn three coincidences into a real check, and would fail immediately if
the two ever came apart.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/001`](../integration/001_two_dependents_and_a_third_that_did_it_again.md) | The dependency edge `ring_mpsc` did not add |
| [`pattern/001`](../pattern/001_one_owner_for_one_arithmetic_fact.md) | The rule this violates, and what the rule actually protects |
| [`invariant/001`](../invariant/001_the_mask_equals_the_modulo.md) | The identity both implementations rely on |
| [`api/readme.md`](../api/readme.md) | Why a types-only surface makes the duplicate invisible to the compiler |

### Sources

| Fact | Where |
|------|-------|
| The duplicate fold | `ring_mpsc/src/lib.rs:543` |
| Three capacities from one argument | `ring_mpsc/src/lib.rs:369-382` |
| `capacity()` reading from `consumers` | `ring_mpsc/src/lib.rs:426-430` |
| `Buffer` holding both in one struct | `ring_store/src/lib.rs:59-63, 87-93` |
| The three length assertions | Census above |

### Tests

| Test | Covers |
|------|--------|
| `ring_mpsc` doc-test at `:306` | `stamps().len()` against a literal |
| *(to create)* | `stamps().len() == capacity().get()` — the coupling the fold depends on and nothing asserts |
| *(to create)* | `ring_mpsc::stamp`'s index against `ring_index::of`, as `ring_batch` already does for its own fold |
