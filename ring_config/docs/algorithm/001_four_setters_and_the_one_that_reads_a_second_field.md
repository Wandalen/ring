# Algorithm: Four Setters, and the One That Reads a Second Field

### Scope

**Purpose:** Record the whole of the crate's computation — four field assignments,
two of them clamped — and the single cross-field read that makes the fourth
different from the other three.

**Responsibility:** Each setter's body, the two clamps, and the structural reason
the cross-field read is safe.

**In Scope:** `ring_config/src/lib.rs:89-93`, `:104-108`, `:122-126`,
`:140-145`, `:71`.

**Out of Scope:** Why clamping was chosen over an error is
[`decisions/002`](../decisions/002_clamping_instead_of_a_question_mark_mid_chain.md).
What happens if a `with_capacity` is ever added is
[`pitfall/002`](../pitfall/002_the_setter_that_would_break_commutation.md).

---

## Every Setter, and the One Cross-Field Read

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every setter body, in declaration order --'
sed -n '/^  pub const fn with_wait( mut self, wait : WaitKind ) -> Self$/,/^    self$/p;/^  pub const fn with_overflow( mut self, overflow : OverflowPolicy ) -> Self$/,/^    self$/p;/^  pub const fn with_producers( mut self, producers : usize ) -> Self$/,/^    self$/p;/^  pub const fn with_batch( mut self, batch : usize ) -> Self$/,/^    self$/p' ring_config/src/lib.rs
echo '  -- the only setter that reads a field it does not write --'
command grep 'self\.capacity\.get()' ring_config/src/lib.rs
echo '  -- the capacity field, its one assignment, and any setter for it --'
command grep 'capacity :\|fn with_capacity' ring_config/src/lib.rs
echo '  -- comparisons, then loop keywords, in the whole crate with comments stripped --'
command grep -v '^ *//' ring_config/src/lib.rs | command grep -c -e ' > \| == \| < \| != \| >= \| <= ' || true
command grep -v '^ *//' ring_config/src/lib.rs | command grep -c '\bfor\b\|\bwhile\b\|\bloop\b' || true
```

Live output:

```
  -- every setter body, in declaration order --
  pub const fn with_wait( mut self, wait : WaitKind ) -> Self
  {
    self.wait = wait;
    self
  pub const fn with_overflow( mut self, overflow : OverflowPolicy ) -> Self
  {
    self.overflow = overflow;
    self
  pub const fn with_producers( mut self, producers : usize ) -> Self
  {
    self.producers = if producers == 0 { 1 } else { producers };
    self
  pub const fn with_batch( mut self, batch : usize ) -> Self
  {
    let capped = if batch > self.capacity.get() { self.capacity.get() } else { batch };
    self.batch = if capped == 0 { 1 } else { capped };
    self
  -- the only setter that reads a field it does not write --
    let capped = if batch > self.capacity.get() { self.capacity.get() } else { batch };
  -- the capacity field, its one assignment, and any setter for it --
  capacity : Capacity,
        capacity : Capacity::new( slots )?,
  -- comparisons, then loop keywords, in the whole crate with comments stripped --
4
0
```

---

### RC1 — The Whole Computation Is Four Assignments, Four Comparisons and a Constructor

With comments stripped, the crate contains zero loop keywords and no arithmetic
operator at all — four comparisons are its entire computation, and there is no
allocation. Each setter takes `mut self` by value, writes one field, and returns
`Self`.

Three of the four comparisons sit in the setters. Two setters write the value they
were given unchanged; the other two guard it first — `with_producers` maps `0` to
`1` (one `==`), and `with_batch` caps at the capacity and then floors at `1` (one
`>`, one `==`). The fourth comparison is not in a setter at all: it is
`is_multi_producer`'s `self.producers > 1`, which computes a reading rather than
storing one. Every guard is an `if` expression in a `const fn`, so all of them are
evaluable at compile time along with everything else on the type.

**Finding.** The shape worth naming is that the crate's entire behaviour is a
record plus two clamps, and the two clamps are the only places where what a caller
asked for and what it gets can differ. Everything else on the type — five getters,
two derived readings, four derives — is a faithful echo of what was stored.

That concentrates the whole of the crate's risk surface into two `if` expressions
occupying three lines, and it is why the test suite spends two of its ten tests on
clamping alone. It also means the type has no state machine, no ordering
constraint between calls, and nothing to test about sequences beyond what
`setters_commute` already asserts.

---

### RC2 — `with_batch` Is the Only Setter That Reads a Field, and Only One Absence Makes That Safe

Three setters read nothing. `with_batch` reads `self.capacity` to compute its
cap, which makes it the one place in the crate where the result of a call depends
on the record's prior contents rather than only on its argument.

A builder whose setters read each other's fields is normally order-dependent, and
this one is not. The census gives the reason: `capacity` appears exactly twice —
once as a field declaration at `:44` and once as an assignment inside `new` at
`:71`. There is no `with_capacity`, so the field `with_batch` reads is fixed
before any setter can run.

**Finding.** So commutation here is a consequence of an absence, not of the
setters' own construction. `setters_commute` asserts the property and passes; what
holds it up is that the one readable field is immutable after construction.

Nothing states this. `new`'s doc describes capacity as the constructor's argument
and `with_batch`'s doc explains the cap in terms of what a ring can serve, and
neither says that the cap is stable *because* capacity cannot be changed later.
The property is load-bearing for a test the suite already has, and it is
maintained by a setter that does not exist rather than by anything a reader can
see.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/002`](../invariant/002_the_setters_commute_and_one_absence_is_why.md) | The commutation property, and what the test does and does not reach |
| [`pitfall/002`](../pitfall/002_the_setter_that_would_break_commutation.md) | What adding `with_capacity` would cost |
| [`decisions/001`](../decisions/001_capacity_is_the_one_parameter_that_is_not_a_with.md) | Why capacity is a constructor argument |
| [`algorithm/002`](002_one_fallible_path_and_it_is_not_this_crates.md) | The only way construction can fail |

### Sources

| Fact | Where |
|------|-------|
| The four setter bodies | `ring_config/src/lib.rs:89-93`, `:104-108`, `:122-126`, `:140-145` |
| The producer clamp | `ring_config/src/lib.rs:124` |
| The batch cap and floor | `ring_config/src/lib.rs:142-143` |
| The one cross-field read | `ring_config/src/lib.rs:142` |
| Capacity's single assignment, and no setter for it | `ring_config/src/lib.rs:44`, `:71` |
| The fourth comparison, outside the setters | `ring_config/src/lib.rs:223` |
| Four comparisons, zero loops, zero arithmetic | Census above |

### Tests

| Test | Covers |
|------|--------|
| `each_setter_is_independent` | That three setters touch nothing else |
| `setters_commute` | The order-independence the absent setter underwrites |
| `batch_clamps_into_one_through_capacity` | Both ends of the one cross-field clamp |
| `zero_producers_clamps_to_one` | The other clamp |
