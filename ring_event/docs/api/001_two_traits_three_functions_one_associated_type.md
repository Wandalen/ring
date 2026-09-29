# API: Two Traits, Three Functions, One Associated Type

### Scope

**Purpose:** Record the whole public surface, what each item returns, and why the
attribute every neighbouring crate uses is absent from this one.

**Responsibility:** The five public items, the associated type, the three bounds
the free functions carry, and the `#[ must_use ]` census against four neighbours.

**In Scope:** `ring_event/src/lib.rs:53`, `:103`, `:106`, `:173-175`,
`:207-209`, `:229-231`.

**Out of Scope:** The `Result` one impl can never return is
[`api/002`](002_a_result_one_impl_can_never_return.md). What the bodies behind
these signatures do is
[`algorithm/001`](../algorithm/001_two_executable_statements_and_one_branch.md).

---

## The Surface, Whole

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the whole public surface: two traits, one associated type, three functions --'
command grep 'pub trait\|pub fn\|  type Out<' ring_event/src/lib.rs
echo '  -- what every returning item in the crate returns --'
command grep -o 'fn [a-z_]*[^-]*-> *[A-Za-z<:_ ]*' ring_event/src/lib.rs | sed 's/( [^)]*)//'
echo '  -- must_use attributes against returning functions, five crates --'
for c in ring_event ring_publish ring_slot ring_store ring_config; do
  printf '%-14s must_use %-3s returning %s\n' "$c" "$( command grep -c 'must_use' $c/src/lib.rs || true )" "$( command grep -c 'pub \(const \)\?fn .*->' $c/src/lib.rs || true )"
done
echo '  -- doctests, and the bound each free function carries --'
awk '/^(\/\/\/|\/\/!) ```/{ n++ } END{ print n/2, "doctests" }' ring_event/src/lib.rs
awk '/^pub fn publish_into< S, P >\( slot : &mut S, payload : P \) -> Result< \(\), RingError >$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 2 { print } /^\/\/\/ let slot = BytesSlot::< 4 >::empty\(\);$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 5 { print } /^pub fn recycle< S >\( slot : &mut S \)$/{ n3 = NR } n3 && NR >= n3 && NR <= n3 + 2 { print }' ring_event/src/lib.rs | command grep ':'
```

Live output:

```
  -- the whole public surface: two traits, one associated type, three functions --
pub trait Fill< S >
pub trait Peek
  type Out< 'a > where Self : 'a;
  type Out< 'a > = &'a T where T : 'a;
  type Out< 'a > = &'a [ u8 ];
pub fn publish_into< S, P >( slot : &mut S, payload : P ) -> Result< (), RingError >
pub fn drain_from< S >( slot : &S ) -> Option< S::Out< '_ > >
pub fn recycle< S >( slot : &mut S )
  -- what every returning item in the crate returns --
fn fill -> Result< 
fn fill -> Result< 
fn fill -> Result< 
fn peek -> Option< Self::Out< 
fn peek -> Option< 
fn peek -> Option< 
fn publish_into< S, P > -> Result< 
fn drain_from< S > -> Option< S::Out< 
  -- must_use attributes against returning functions, five crates --
ring_event     must_use 0   returning 2
ring_publish   must_use 5   returning 6
ring_slot      must_use 9   returning 10
ring_store    must_use 9   returning 11
ring_config    must_use 11  returning 12
  -- doctests, and the bound each free function carries --
5 doctests
pub fn publish_into< S, P >( slot : &mut S, payload : P ) -> Result< (), RingError >
  P : Fill< S >,
/// let mut slot = BytesSlot::< 4 >::empty();
pub fn recycle< S >( slot : &mut S )
  S : Slot,
```

---

### EV5 — Zero `#[ must_use ]`, and It Is the Right Number

Four neighbouring crates carry the attribute between four and eleven times each,
in every case on most of their returning functions: `ring_config` marks eleven of
twelve, `ring_store` seven of eleven, `ring_slot` six of ten, `ring_publish` four
of six. This crate marks none of two.

The reason is visible in the return census: every returning item in the crate
returns `Result< (), RingError >` or an `Option`, and both are already
`#[ must_use ]` in `core`. There is no function here that hands back a bare value
a caller could drop by accident. `recycle` returns `()` and so cannot.

**Finding.** The absence is a consequence of the surface's shape rather than an
omission, and it is worth recording precisely because a reviewer sweeping the
family for missing attributes would find this crate at zero and have no way to
tell it apart from a crate that forgot. The distinguishing fact — that the two
returning signatures are `Result` and `Option` and nothing else — is not stated
anywhere in the crate.

Five public items over 234 lines is also the smallest surface among the five
crates sampled, which is consistent with what the crate is for: it does not add
capability, it removes the possibility of two capabilities where there should be
one.

---

### EV6 — The Three Functions Do Not Share a Bound, so the One Path Costs Three Traits From Two Crates

`publish_into` requires `P : Fill< S >`. `drain_from` requires `S : Peek`.
`recycle` requires `S : Slot`. No two of the three agree, and `Slot` is
`ring_slot`'s trait rather than one of this crate's.

A caller writing a generic routine that publishes, drains and recycles therefore
has to name all three. The crate's own test suite is the demonstration: its
generic helper `land_and_read` declares `S : Peek + Slot + Default` and
`P : Fill< S >` — four bounds across two crates to express one round trip through
what the module documentation calls "the identical claim/publish/drain path".

**Finding.** Neither trait declares the other, or `Slot`, as a supertrait, and
nothing records why. Both types that implement `Peek` also implement `Slot` —
`TypedSlot` and `BytesSlot` are the only implementors of either, and each
implements both — so `pub trait Peek : Slot` would compile today and would let
`recycle` bound on `Peek` alone, collapsing three traits to two.

That is not obviously the right change: a supertrait is a permanent coupling, and
`Peek` and `Slot` live in different crates for reasons the family documents
elsewhere. But the choice is not recorded as a choice. What a reader sees is three
functions presented as one path, each admitting a different set of types, with no
sentence anywhere saying that the set is the same today or that keeping it the
same is nobody's job.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/002`](002_a_result_one_impl_can_never_return.md) | The one signature that is wider than either impl needs |
| [`algorithm/001`](../algorithm/001_two_executable_statements_and_one_branch.md) | What sits behind these five declarations |
| [`type/001`](../type/001_an_associated_type_with_a_lifetime.md) | The associated type in the surface above |
| [`item/001`](../item/001_five_public_items_and_their_traffic.md) | Declaration-level detail on each item |

### Sources

| Fact | Where |
|------|-------|
| Two traits, one associated type, three functions | `ring_event/src/lib.rs:53`, `:103`, `:106`, `:173`, `:207`, `:229` |
| Every returning item returning `Result` or `Option` | Census above |
| Zero `#[ must_use ]` against four neighbours' four-to-eleven | Census above |
| Five doctests | Census above |
| Three different bounds on three functions | `ring_event/src/lib.rs:175`, `:209`, `:231` |
| The suite needing four bounds to express one round trip | `ring_event/tests/event_test.rs:37-43` |

### Tests

| Test | Covers |
|------|--------|
| `both_shapes_land_in_storage_through_the_same_two_calls` | The four-bound generic helper |
| `fill_is_implemented_on_the_payload_so_a_new_payload_needs_no_slot_change` | `Fill` reached directly rather than through the free function |
| `peek_is_implemented_on_the_slot_so_the_reader_needs_no_payload_type` | `Peek` reached the same way |
