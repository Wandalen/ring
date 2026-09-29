# Workaround: Four Functions That Could Be `const`

### Scope

**Purpose:** Record that all four of the crate's non-`const` functions can be
`const` on the toolchain the workspace builds with — two verbatim, two after
replacing an index operator with `split_at` — and that the crate documents none
of this.

**Responsibility:** The `const`/non-`const` split across the ten public
functions: what forces it, what does not, and what it costs to close.

**In Scope:** `ring_slot/src/lib.rs:120, 154, 347, 367` — the four plain
`pub fn`; the const-evaluation probe on rustc 1.97.1.

**Out of Scope:** The census itself — which six are `const` and which six carry
`#[ must_use ]` — is
[`api/001`](../api/001_ten_functions_six_const_seven_must_use.md). What each
function *does* is [`item/001`](../item/001_the_four_of_a_typed_slot.md) and
[`item/002`](../item/002_the_six_of_a_bytes_slot.md).

---

## The Split

```sh
cd "$(git rev-parse --show-toplevel)"
printf '  const fn : %d\n' "$( grep -c '^[[:space:]]*pub const fn ' ring_slot/src/lib.rs )"
printf '  plain fn : %d\n' "$( grep -c '^[[:space:]]*pub fn ' ring_slot/src/lib.rs )"
grep '^[[:space:]]*pub fn ' ring_slot/src/lib.rs
```

Live output:

```
  const fn : 6
  plain fn : 4
  pub fn set( &mut self, value : T ) -> Option< T >
  pub fn take( &mut self ) -> Option< T >
  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >
  pub fn read( &self ) -> &[ u8 ]
```

Six to four. The six are the constructors and the readers of scalar state; the
four are the two mutators on each shape and the two accessors that touch a
payload.

The split reads like a capability boundary. It is not one.

---

### SL49 — Two of the Four Are `const`-Capable Verbatim

`set` is `self.0.replace( value )` and `take` is `self.0.take()`. Both
`Option::replace` and `Option::take` are const-stable, so adding the keyword to
either function requires changing nothing else. Compiled and evaluated in a real
`const` block on the workspace's toolchain:

```
rustc 1.97.1 (8bab26f4f 2026-07-14)
  compile-time set+take:    true
```

The block that produced it constructs a slot, calls `set`, calls `take`, and
asserts the payload came back — entirely at compile time, through generic
`const fn` mirrors of the crate's own two bodies.

**Finding.** Two functions are marked non-`const` for no reason the code
contains. That is not a bug and it costs nothing today — `const` on a `&mut self`
method is only reachable from a `const` context, and nothing in the family
constructs slots at compile time
([`item/002`](../item/002_the_six_of_a_bytes_slot.md) SL27 records that even the
`const` accessors have no caller outside the crate's own tests).

What it costs is legibility. A reader seeing six `const fn` and four `pub fn`
side by side will infer that the four do something `const` cannot express, and
for two of them that inference is simply wrong. The inference is the harm: it
makes the split look like a considered boundary rather than the default it
apparently is.

---

### SL50 — The Other Two Are Blocked by an Operator, Not by What They Do

`read` and `write` each use an index expression:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^    self\.bytes\[ \.\.payload\.len() ]\.copy_from_slice( payload );$/p;/^    &self\.bytes\[ \.\.self\.len ]$/p' ring_slot/src/lib.rs
```

Live output:

```
    self.bytes[ ..payload.len() ].copy_from_slice( payload );
    &self.bytes[ ..self.len ]
```

Those two expressions are the whole obstacle. Attempting either inside a
`const fn` on rustc 1.97.1 fails on the operator and nothing else:

```
error: `Index` is not yet stable as a const trait
error: `IndexMut` is not yet stable as a const trait
```

The equivalent slice methods *are* const-stable, and the rewritten forms —
`self.bytes.split_at( self.len ).0` for `read`, and `split_at_mut` before the
same `copy_from_slice` for `write` — both compile as `const fn` and both evaluate
at compile time:

```
  compile-time read length: 5
```

Five bytes written and read back inside a `const` initialiser, through a
`const fn` `write` performing the identical bound check, `copy_from_slice`, and
length store as the shipped function.

**Finding.** All four non-`const` functions could carry the keyword; two need no
change and two need one expression rewritten each. The external constraint is
real — `Index` and `IndexMut` are not yet const-stable — but it constrains the
*spelling*, not the operation, and the crate absorbs it by writing the clearer
form and losing `const` rather than by writing the const-capable form and keeping
it.

That is a defensible trade and an unrecorded one:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^[[:space:]]*//[/!].*\bconst' ring_slot/src/lib.rs | grep -viE 'constraint|construct' \
  || echo '  no doc comment anywhere explains a const choice'
```

Live output:

```
  no doc comment anywhere explains a const choice
```

Not one comment on ten functions explains why six carry the keyword and four do
not. The consequence is that when `Index` does stabilise as a const trait, there
is nothing in the crate to tell a maintainer that two of these four were waiting
on exactly that — the constraint will lift silently and the workaround will
outlive it, which is the ordinary way a workaround becomes permanent.

The cheap fix is a comment, not a rewrite: one line on `read` and `write` naming
the operator as the blocker, and the keyword added to `set` and `take` where it
is free.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/002`](002_batchtoolarge_borrowed_for_a_different_shape.md) | The crate's other absorbed constraint, in the error type |
| [`api/001`](../api/001_ten_functions_six_const_seven_must_use.md) | The census this instance interprets |
| [`item/002`](../item/002_the_six_of_a_bytes_slot.md) | `capacity`, the `const` function with no external caller |
| [`algorithm/001`](../algorithm/001_write_is_a_bound_check_and_a_copy.md) | `write`'s three steps, unchanged by the const rewrite |
| [`non_functional_requirement/001`](../non_functional_requirement/001_what_a_slot_costs.md) | Where `const` construction makes the O(N) zeroing free |

### Sources

| Fact | Where |
|------|-------|
| Six `const fn`, four plain | `ring_slot/src/lib.rs` |
| The four plain functions | `ring_slot/src/lib.rs:120, 154, 347, 367` |
| The two index expressions | `ring_slot/src/lib.rs:353, 369` |
| No doc comment on any `const` choice | `ring_slot/src/lib.rs` — no occurrence |
| Const-evaluation of all four, and the two operator errors | Probe on rustc 1.97.1, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `capacity_is_the_const_parameter` | A `const` accessor, exercised at runtime only |
| `a_bytes_slot_round_trips_its_payload` | `write` and `read`, the two operator-blocked functions |
| `a_typed_slot_round_trips_its_value` | `set` and `take`, the two that are free to mark |
| `a_zero_capacity_slot_accepts_only_nothing` | The degenerate `N`, where const construction is trivially free |
| *(to create)* | A `const` block constructing a slot, pinning which functions are compile-time reachable |
