# Algorithm: Two Executable Statements, and One Branch

### Scope

**Purpose:** Record what the crate computes, which turns out to be almost
nothing, and where the work it appears to do actually happens.

**Responsibility:** The three free functions' bodies, the four trait impls they
resolve to, the crate's entire executable content, and the one function whose
bound belongs to another crate.

**In Scope:** `ring_event/src/lib.rs:66-86`, `:127-145`, `:173-178`,
`:207-212`, `:229-234`; `ring_slot/src/lib.rs:41-67`.

**Out of Scope:** Where the dispatch is resolved, and what it costs, is
[`algorithm/002`](002_the_dispatch_happens_before_the_program_runs.md). The
`Option`/`Result` shapes the bodies return are
[`api/002`](../api/002_a_result_one_impl_can_never_return.md).

---

## The Whole Crate, Executable Content First

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the three free functions, whole --'
awk '/^pub fn publish_into< S, P >\( slot : &mut S, payload : P \) -> Result< \(\), RingError >$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 5 { print } /^\/\/\/ let slot = BytesSlot::< 4 >::empty\(\);$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 8 { print } /^pub fn recycle< S >\( slot : &mut S \)$/{ n3 = NR } n3 && NR >= n3 && NR <= n3 + 5 { print }' ring_event/src/lib.rs
echo '  -- the four impl bodies they resolve to --'
sed -n '/^impl< T > Fill< TypedSlot< T > > for T$/,/^  }$/p;/^impl< const N : usize > Fill< BytesSlot< N > > for &\[ u8 ]$/,/^  }$/p;/^impl< T > Peek for TypedSlot< T >$/,/^  }$/p;/^impl< const N : usize > Peek for BytesSlot< N >$/,/^  }$/p' ring_event/src/lib.rs | command grep -v '^$'
echo '  -- executable statements in the crate, doc comments stripped --'
sed 's|//.*||' ring_event/src/lib.rs | command grep -n ';' | command grep -v ':use \|: *fn \|: *type ' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- control flow, impl headers excluded --'
sed 's|//.*||' ring_event/src/lib.rs | command grep -nw 'if\|match\|for\|while\|loop' | command grep -v ':impl' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- and the trait recycle bounds on, which is not this crates --'
command grep -m1 -A7 -F 'pub trait Slot' ring_slot/src/lib.rs
```

Live output:

```
  -- the three free functions, whole --
pub fn publish_into< S, P >( slot : &mut S, payload : P ) -> Result< (), RingError >
where
  P : Fill< S >,
{
  payload.fill( slot )
}
/// let mut slot = BytesSlot::< 4 >::empty();
/// publish_into( &mut slot, &b"ab"[ .. ] ).unwrap();
/// assert_eq!( drain_from( &slot ), Some( &b"ab"[ .. ] ) );
/// assert!( !slot.is_empty(), "reading it did not free it" );
///
/// recycle( &mut slot );
pub fn recycle< S >( slot : &mut S )
where
  S : Slot,
{
  slot.clear();
}
  -- the four impl bodies they resolve to --
impl< T > Fill< TypedSlot< T > > for T
{
  fn fill( self, slot : &mut TypedSlot< T > ) -> Result< (), RingError >
  {
    // Discarding the displaced value is `fill`'s documented contract — "write
    // `self` into `slot`, replacing whatever it held" — not an oversight. A
    // caller that needs the old record calls `TypedSlot::set` directly and
    // binds it; this trait exists to give both slot shapes one signature, and
    // `BytesSlot` has nothing to hand back.
    slot.set( self );
    Ok( () )
  }
impl< const N : usize > Fill< BytesSlot< N > > for &[ u8 ]
{
  fn fill( self, slot : &mut BytesSlot< N > ) -> Result< (), RingError >
  {
    slot.write( self )
  }
impl< T > Peek for TypedSlot< T >
{
  type Out< 'a > = &'a T where T : 'a;
  fn peek( &self ) -> Option< &T >
  {
    self.get()
  }
impl< const N : usize > Peek for BytesSlot< N >
{
  type Out< 'a > = &'a [ u8 ];
  fn peek( &self ) -> Option< &[ u8 ] >
  {
    if self.is_empty() { None } else { Some( self.read() ) }
  }
  -- executable statements in the crate, doc comments stripped --
    slot.set( self );
  slot.clear();
  -- control flow, impl headers excluded --
    if self.is_empty() { None } else { Some( self.read() ) }
  -- and the trait recycle bounds on, which is not this crates --
pub trait Slot
{
  /// Whether this slot currently holds nothing.
  fn is_empty( &self ) -> bool;

  /// Return the slot to its empty state.
  ///
  /// **Not a promise to overwrite.** For a shape that owns what it stores
```

---

### EV1 — The Crate's Entire Computation Is Two Statements and One Branch

Seven bodies — three free functions and four trait methods — and with doc
comments stripped the crate contains exactly two semicolon-terminated executable
statements (`slot.set( self );` at `:75`, `slot.clear();` at `:233`) and exactly
one branch (`:143`). Everything else is a tail expression forwarding to a call in
`ring_slot`, or a signature.

`publish_into` is `payload.fill( slot )`. `drain_from` is `slot.peek()`. `recycle`
is `slot.clear()`. Of the four impls, three forward to a single `ring_slot`
method and the fourth adds the one branch — the `is_empty` test that turns a
zero-length read into `None`.

**Finding.** This is not a criticism, and the module documentation says so
outright at `:149-150`: "Deliberately trivial. Its value is not what it does but
that there is only one of it." It is worth recording as a measurement because it
fixes what the rest of this corpus can be about. A crate with two statements has
essentially no behaviour to get wrong; every finding here is necessarily about
structure — who can call what, what the type system permits, and whether the
single path the crate exists to create is actually the path anything takes.

The one branch is the crate's only place where the two shapes are treated
differently, and it is the origin of the crate's only documented limitation
([`pitfall/001`](../pitfall/001_a_zero_length_payload_reads_as_nothing.md)).

---

### EV2 — Two of the Three Functions Create a Shared Path; the Third Renames One `ring_slot` Already Had

`publish_into` and `drain_from` bound on `Fill` and `Peek`, both declared in this
crate. Those two traits are the mechanism: without them there is no single
signature that both slot shapes satisfy, and the family would need one write call
per shape.

`recycle` is different. Its bound is `S : Slot` — `ring_slot`'s trait, declared at
`ring_slot/src/lib.rs:41`, whose `clear` is already a trait method with one
implementation per shape. Any type that can be passed to `recycle` can be passed
to `Slot::clear` directly, with the same monomorphisation and the same result.

**Finding.** `recycle`'s stated reason — "a shape-specific reset would be a fourth
path the identical-path claim does not cover" (`:217-218`) — names a risk that
`Slot::clear` has already foreclosed. A type reachable through `recycle` is a type
that implements `Slot`, and `Slot` has exactly one `clear`. The function cannot
prevent a shape from also offering an inherent reset (`TypedSlot::take` at
`ring_slot:154` is exactly that), because a caller can reach the inherent method
without going near either crate's trait.

So the three functions are not the same kind of thing. Two of them are the crate's
whole point; the third is a re-export under a different name, and the reason given
for it does not distinguish it from the trait method it forwards to. Worth
recording rather than deleting: a caller reading `publish_into`, `drain_from`,
`recycle` as a set has no way to tell that only the first two are load-bearing,
and the rationale comment actively suggests all three are.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](002_the_dispatch_happens_before_the_program_runs.md) | Where the calls these bodies forward to are chosen |
| [`api/001`](../api/001_two_traits_three_functions_one_associated_type.md) | The surface these seven bodies sit behind |
| [`integration/002`](../integration/002_the_ring_writes_slots_without_this_crate.md) | Whether the single path is the path anything takes |
| [`pitfall/001`](../pitfall/001_a_zero_length_payload_reads_as_nothing.md) | What the one branch cannot distinguish |

### Sources

| Fact | Where |
|------|-------|
| The three free functions' bodies | `ring_event/src/lib.rs:173-178`, `:207-212`, `:229-234` |
| The four impl bodies | `ring_event/src/lib.rs:66-86`, `:127-145` |
| Two executable statements, one branch | Census above |
| `recycle`'s stated rationale | `ring_event/src/lib.rs:216-218` |
| `Slot::clear` as a trait method with two impls | `ring_slot/src/lib.rs:41-67`, `:183`, `:392` |
| An inherent reset that bypasses both traits | `ring_slot/src/lib.rs:154` |

### Tests

| Test | Covers |
|------|--------|
| `one_generic_body_round_trips_a_typed_slot` | The two load-bearing functions over one shape |
| `the_same_generic_body_round_trips_a_bytes_slot` | The same body over the other |
| `recycling_empties_either_shape_through_the_same_call` | The third function, over both shapes |
