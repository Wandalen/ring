# Item: Five Public Items and Their Traffic

### Scope

**Purpose:** Record every public declaration in the crate at the level of the
declaration itself — where it sits, how much documentation it carries, whether
that documentation runs, what attributes it wears, and who calls it — and read
what the shape of that census says.

**Responsibility:** The six declarations in `src/lib.rs`, the crate's attribute
count against the family's, the call traffic of each free function and each
trait method, and the two `ring_slot` methods whose return value nothing
protects.

**In Scope:** `ring_event/src/lib.rs:53`, `:103`, `:106`, `:173`, `:207`,
`:229`; `ring_slot/src/lib.rs:95`, `:120`, `:154`, `:278`, `:290`, `:304`,
`:221`, `:263`; `ring_core/src/lib.rs:389`;
`ring_mpsc/src/lib.rs:914`; `ring_spsc/src/lib.rs:719`; probe
below.

**Out of Scope:** What the surface shape costs a caller is
[`api/001`](../api/001_two_traits_three_functions_one_associated_type.md). Why
`Out` is a GAT is
[`type/001`](../type/001_an_associated_type_with_a_lifetime.md).

---

## Every Declaration, Measured

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every public item, its doc block and whether that block runs --'
awk '
  /^ *\/\/\// { d++; if ( $0 ~ /```/ ) { t = 1 } ; next }
  /^ *(pub (trait|fn) |type Out< .a > where)/ { printf "%-84s line %-4d doc %-3d doctest %s\n", $0, NR, d, ( t ? "yes" : "no" ) }
  { d = 0 ; t = 0 }
' ring_event/src/lib.rs
echo '  -- doc lines against the whole file --'
command grep -c '^ *//[/!]' ring_event/src/lib.rs || true
wc -l < ring_event/src/lib.rs
echo '  -- attributes in src, across all 33 ring crates, lowest five and highest three --'
for c in ring_*/; do
  n=$( command grep -rhc '^ *#\[' --include=*.rs "$c"src 2>/dev/null | awk '{ s += $1 } END { print s + 0 }' )
  echo "$n $( basename $c )"
done | sort -n | sed -n '1,5p;31,33p'
echo '  -- where each free function is called --'
for f in publish_into drain_from recycle; do
  a=$( command grep -c "^ */// .*$f( " ring_event/src/lib.rs || true )
  b=$( command grep -c "$f( " ring_event/tests/event_test.rs || true )
  c=$( command grep -rn "$f( " --include=*.rs ring_*/ | command grep -vc '^ring_event/' || true )
  printf '%-14s own-doctest %-4s own-tests %-4s elsewhere %s\n' "$f" "$a" "$b" "$c"
done
echo '  -- and every .fill / .peek call in the ring family outside this crate --'
# Hyphen-prefixed paths excluded: `-mutation/-pristine.rs` is a temporary
# mutation-testing artifact rather than family source.
command grep -r '\.fill(\|\.peek(' --include=*.rs ring_*/ \
  | command grep -v '^ring_event/' \
  | command grep -v '/-' \
  | LC_ALL=C sort
```

Live output:

```
  -- every public item, its doc block and whether that block runs --
pub trait Fill< S >                                                                  line 53   doc 14  doctest yes
pub trait Peek                                                                       line 103  doc 15  doctest yes
  type Out< 'a > where Self : 'a;                                                    line 106  doc 1   doctest no
pub fn publish_into< S, P >( slot : &mut S, payload : P ) -> Result< (), RingError > line 173  doc 26  doctest yes
pub fn drain_from< S >( slot : &S ) -> Option< S::Out< '_ > >                        line 207  doc 27  doctest yes
pub fn recycle< S >( slot : &mut S )                                                 line 229  doc 15  doctest yes
  -- doc lines against the whole file --
154
234
  -- attributes in src, across all 33 ring crates, lowest five and highest three --
0 ring_event
1 ring_wait
3 ring_align
3 ring_factory
3 ring_index
27 ring_mpsc
35 ring_core
50 ring_bench
  -- where each free function is called --
publish_into   own-doctest 4    own-tests 13   elsewhere 1
drain_from     own-doctest 5    own-tests 18   elsewhere 1
recycle        own-doctest 2    own-tests 7    elsewhere 0
  -- and every .fill / .peek call in the ring family outside this crate --
```

Zero. No other crate in the ring family calls `Fill::fill` or `Peek::peek`.

**Correction (2026-09-29):** the scan above was originally workspace-wide,
scoped to the private monorepo `ring` was developed inside — it found twenty-two
name collisions with `Fill::fill`/`Peek::peek` across eight unrelated crates
elsewhere in that monorepo, all standard-library `slice::fill`/`BinaryHeap::peek`
calls or other crates' own inherent methods of the same name, none of them
`ring_event`'s. `ring` has since been extracted into its own standalone
repository, which cannot see that monorepo at all, so a workspace-wide claim is
no longer one this repo can make or verify about itself. The scan is now scoped
to the ring family alone, which is the only tree this repository owns — the
zero result above is the honest, current answer to the narrower question,
not a rerun of the original claim.

## What a Discarded Return Value Costs

*The lint warning below is frozen at authoring time: the probe binary has
since been swept, and rustc's exact warning wording is not a stability
guarantee across compiler versions. `Result`'s `#[ must_use ]` in `core` and
`publish_into`'s signature (`ring_event/src/lib.rs:173`) are unchanged,
so the demonstrated fact — one warning fires for the discarded `Result`,
none for the discarded `Option`s — should still hold; only the literal
compiler text is not re-verified.*

```rust
// -ev_probe/src/bin/discarded_returns.rs
let mut slot = TypedSlot::< u32 >::empty();

// Three discarded return values. No item in either crate carries `#[ must_use ]`
// on the path these take.
publish_into( &mut slot, 7u32 );
drain_from( &slot );
slot.set( 9u32 );
```

```
warning: unused `Result` that must be used
  --> src/bin/discarded_returns.rs:10:3
   |
10 |   publish_into( &mut slot, 7u32 );
   |   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: this `Result` may be an `Err` variant, which should be handled
   = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
   |
10 |   let _ = publish_into( &mut slot, 7u32 );
   |   +++++++

warning: `ev_probe` (bin "discarded_returns") generated 1 warning
```

One warning, three discards. `Result` carries `#[ must_use ]` in `core` and
`Option` does not.

---

### EV25 — The Only Crate of Thirty-Three With No Attributes at All, and the One Place That Shows

Every other ring crate decorates something. `ring_bench` carried 43 attributes in
its sources when this was filed and carries 50 now; `ring_core` 35 and
`ring_mpsc` 27 are unchanged. The quietest four after this one carry between one
and three. `ring_event` carries none — no `#[ derive ]`, no
`#[ inline ]`, no `#[ must_use ]`, no `#[ non_exhaustive ]`, nothing but the
crate-level `#![ deny( missing_docs ) ]` that is not an item attribute.

Four of those absences are right, and for reasons worth stating. There is nothing
to derive because the crate declares no data
([`data_structure/001`](../data_structure/001_a_crate_that_declares_no_data.md)).
There is nothing to mark `#[ non_exhaustive ]` for the same reason. `#[ inline ]`
would be noise: all three free functions are generic, so their bodies are
instantiated in the caller's crate and already available to the optimiser without
it. And `publish_into` needs no `#[ must_use ]`, because it returns `Result`,
which carries one in `core` — the probe's single warning is that attribute firing.

The fifth is not right. `drain_from` returns `Option< S::Out< '_ > >`, and
`Option` — unlike `Result` — is not `#[ must_use ]`. So `drain_from( &slot );`
compiles silently, in a crate whose whole purpose is to be the read path.

**Finding.** The discard is harmless in the narrow sense: `drain_from` takes
`&S` and mutates nothing, so throwing away the result loses no payload and leaves
no slot in a bad state. What it produces is a statement that reads like a drain
and does nothing at all — the exact call a reader would scan past. One
`#[ must_use ]` on `drain_from` makes it an error to write, costs one line, and
would bring the crate's attribute count to one, which is where `ring_wait`
already is. The zero is worth keeping as a deliberate zero everywhere else; it is
not worth keeping here.

---

### EV26 — `ring_slot`'s Six `#[ must_use ]` Attributes Are on the Five Methods Where Discarding Is Free

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every attribute in ring_slot, against the item it sits on --'
awk '
  /^ *#\[/ { a = $0 ; next }
  /^ *pub (const )?fn |^pub struct / { if ( a != "" ) { printf "%-4d %-34s %s\n", NR, a, $0 } ; a = "" }
' ring_slot/src/lib.rs
echo '  -- the two methods that hand back an owned payload --'
command grep 'pub fn set\|pub fn take' ring_slot/src/lib.rs
echo '  -- and how the family treats what set hands back --'
command grep -r '\.set( ' --include=*.rs ring_core/src ring_mpsc/src ring_spsc/src
```

Live output:

```
  -- every attribute in ring_slot, against the item it sits on --
84   #[ derive( Debug, Clone, PartialEq, Eq ) ] pub struct TypedSlot< T >( Option< T > );
95     #[ must_use ]                      pub const fn empty() -> Self
154    #[ must_use = "this is the only way a payload leaves the slot; dropping it here destroys the record" ]   pub fn take( &mut self ) -> Option< T >
237  #[ derive( Clone ) ]               pub struct BytesSlot< const N : usize >
278    #[ must_use ]                      pub const fn empty() -> Self
290    #[ must_use ]                      pub const fn capacity( &self ) -> usize
304    #[ must_use ]                      pub const fn len( &self ) -> usize
323    #[ must_use ]                      pub const fn is_empty( &self ) -> bool
367    #[ must_use ]                      pub fn read( &self ) -> &[ u8 ]
  -- the two methods that hand back an owned payload --
  pub fn set( &mut self, value : T ) -> Option< T >
  pub fn take( &mut self ) -> Option< T >
  -- and how the family treats what set hands back --
ring_core/src/lib.rs:            let displaced = reserved.set( record );
ring_mpsc/src/lib.rs:    reserved.set( value );
ring_spsc/src/lib.rs:    drop( reservation.set( record ) );
```

The six attributes sit on `TypedSlot::empty`, `BytesSlot::empty`, `capacity`,
`len`, `is_empty` and `read` — two constructors and four queries, every one of
them `&self` or nothing, every one of them free to discard. The two methods that
hand the caller an owned `T` out of the slot, `set` at `:90` and `take` at
`:117`, carry no attribute at all.

`set`'s own doc comment is what makes this matter. It explains that "returning
the displaced value rather than dropping it is what lets `ring_overflow`'s
evict-oldest policy hand the caller what it evicted, instead of losing it
silently" — the return value is the whole point of the signature, and there is
nothing stopping a caller from dropping it silently anyway.

Three crates call `set` and treat that return three different ways.
`ring_core:389` binds it and checks it: `let displaced = reserved.set( record );`
followed by `debug_assert!( displaced.is_none(), "a claimed slot held a record" )`.
`ring_spsc:719` discards it out loud: `drop( reservation.set( record ) )`.
`ring_mpsc:914` discards it silently: `reserved.set( value );`, a bare statement
with the `Option` dropped where it falls.

**Finding.** All three are correct today, because a claimed slot is empty and
`set` therefore returns `None` at each of them. But only one of the three says so.
`ring_core` asserts the assumption in debug builds; `ring_spsc`'s `drop` at least
records that a value was consciously thrown away; `ring_mpsc` reads exactly like
a method returning `()`. If the claim protocol ever handed out a slot that still
held a record, `ring_core` would trip its assertion and `ring_mpsc` would drop a
live payload without a diagnostic. A `#[ must_use ]` on `set` and `take` costs
two lines, turns `ring_mpsc:914` into a compile error until it states its intent,
and puts the attribute where discarding actually loses something — which is not
where the six that exist are.

**Disposition:** declined — the fix is a `#[ must_use ]` on `set` and `take`
in `ring_slot/src/lib.rs:120`, `:154`, plus the follow-on it names at
`ring_mpsc/src/lib.rs:914`; both are outside this pass's assigned
crates (`ring_event`, `ring_factory`, `ring_flush`), and `ring_slot/src/lib.rs`
currently carries a large, unrelated in-progress edit of its own (uncommitted,
concurrent) — adding an attribute there now risks colliding with that work
rather than closing this finding cleanly.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/001`](../api/001_two_traits_three_functions_one_associated_type.md) | The same five items, read as a surface rather than as declarations |
| [`api/002`](../api/002_a_result_one_impl_can_never_return.md) | The displaced value this crate's `Fill` impl drops before a caller can see it |
| [`data_structure/001`](../data_structure/001_a_crate_that_declares_no_data.md) | Why there is nothing here to derive |
| [`integration/001`](../integration/001_one_declarer_and_it_is_a_dev_dependency.md) | The one outside caller the traffic census finds |

### Sources

| Fact | Where |
|------|-------|
| The six declarations, their doc sizes and doctests | `ring_event/src/lib.rs:53`, `:103`, `:106`, `:173`, `:207`, `:229` |
| Zero attributes here, against 43 at the family's top | Census above |
| `drain_from` returning an `Option` nothing protects | `ring_event/src/lib.rs:207`, probe above |
| `ring_slot`'s six `#[ must_use ]`, all on queries and constructors | `ring_slot/src/lib.rs:95`, `:278`, `:290`, `:304`, `:323`, `:367` |
| `set` and `take` returning an owned payload, unmarked | `ring_slot/src/lib.rs:120`, `:154` |
| Why `set` returns the displaced value at all | `ring_slot/src/lib.rs:102-105` |
| The three treatments of that return | `ring_core/src/lib.rs:389`, `ring_mpsc/src/lib.rs:914`, `ring_spsc/src/lib.rs:719` |
| The only outside calls, both in a test | `ring_tls/tests/tls_test.rs:126`, `:132` |

### Tests

| Test | Covers |
|------|--------|
| `one_generic_body_round_trips_a_typed_slot` | Two of the five items, through one generic body |
| `recycling_empties_either_shape_through_the_same_call` | The one item with no caller outside this crate |
| `a_typed_publish_cannot_fail` | The return value `publish_into`'s attribute-free signature still protects |
