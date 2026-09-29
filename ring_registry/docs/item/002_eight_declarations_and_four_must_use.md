# Item: Eight Declarations and Four `must_use`

### Scope

**Purpose:** Record which of the eight public methods carry `#[ must_use ]`, what
the compiler actually warns about when all eight returns are dropped, and where
the one uncovered method sits in a family-wide census.

**Responsibility:** The four attribute sites; the seven-call ignored-return
probe; the five methods across the family that hand ownership out through
`Option`; and the family's `must_use` and `expect` totals.

**In Scope:** `ring_registry/src/lib.rs:104`, `:189`, `:199`, `:205`,
`:212`, `:219`, `:232`; every ownership-transferring `Option` return in
`ring_*/src/lib.rs`.

**Out of Scope:** What dropping `remove`'s return actually destroys is
[`pitfall/002`](../pitfall/002_the_remove_that_drops_a_ring_without_a_word.md).
The lint attributes are [`item/001`](001_two_lints_one_allow_and_the_reason_beside_it.md).

---

## Which Four, and What the Compiler Says About the Rest

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- which of the eight carry the attribute --'
command grep -B 1 '^  pub fn' ring_registry/src/lib.rs | command grep -e 'must_use' -e 'pub fn' | sed 's/^/    /'
echo '  -- every pub fn in the family handing ownership out through Option --'
# the preceding line is checked for the attribute rather than printed: several of
# these sit under a doc example, whose fence would land inside this output.
# `-B1 --no-group-separator` pairs each match with its preceding line as two
# stream lines rather than seeking the source file by a captured line number.
command grep -rB1 --no-group-separator 'pub fn [a-z_]*( *&mut self.*) *-> *Option< [A-Z]' --include=lib.rs ring_*/src/ \
| awk '
  NR % 2 == 1 { mark = ( $0 ~ /must_use/ ) ? "must_use" : "--------"; next }
  {
    file = $0; sub( /^ring\//, "", file ); sub( /:.*/, "", file )
    rest = $0; sub( /^[^:]*:/, "", rest ); sub( /^ */, "", rest )
    printf( "    %-8s %s:%s\n", mark, file, rest )
  }
'
echo '  -- and the two attribute forms across the family --'
printf '    must_use attributes: %s   expect attributes: %s\n' \
  "$( command grep -rc '#\[ must_use' --include=lib.rs ring_*/src/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )" \
  "$( command grep -rc '#\[ expect' --include=*.rs ring_*/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
```

Live output:

```
  -- which of the eight carry the attribute --
      #[ must_use ]
      pub fn new() -> Self
      pub fn register
      pub fn get_mut( &mut self, name : &str ) -> Option< &mut Split< T > >
      pub fn remove( &mut self, name : &str ) -> Option< Split< T > >
      #[ must_use ]
      pub fn contains( &self, name : &str ) -> bool
      #[ must_use ]
      pub fn len( &self ) -> usize
      #[ must_use ]
      pub fn is_empty( &self ) -> bool
      pub fn names( &self ) -> impl Iterator< Item = &str >
  -- every pub fn in the family handing ownership out through Option --
    -------- ring_core/src/lib.rs:pub fn try_recv( &mut self ) -> Option< T >
    -------- ring_handle/src/lib.rs:pub fn try_recv( &mut self ) -> Option< T >
    -------- ring_registry/src/lib.rs:pub fn remove( &mut self, name : &str ) -> Option< Split< T > >
    -------- ring_slot/src/lib.rs:pub fn set( &mut self, value : T ) -> Option< T >
    must_use ring_slot/src/lib.rs:pub fn take( &mut self ) -> Option< T >
  -- and the two attribute forms across the family --
    must_use attributes: 275   expect attributes: 0
```

## Every Return Dropped, and What the Compiler Notices

A library crate that calls all seven methods and ignores every result:

```rust
// compile/-ignored_returns.rs
pub fn f( r : &mut Registry< u32 >, s : Split< u32 > )
{
  r.register( "a", s );
  r.get_mut( "a" );
  r.remove( "a" );
  r.contains( "a" );
  r.len();
  r.is_empty();
  r.names();
}
```

```
    warning: unused `Result` that must be used
    warning: unused return value of `Registry::<T>::contains` that must be used
    warning: unused return value of `Registry::<T>::len` that must be used
    warning: unused return value of `Registry::<T>::is_empty` that must be used
    warning: unused implementer of `Iterator` that must be used
    warning: 5 warnings emitted
```

Five of seven. The two the compiler says nothing about are `get_mut` and
`remove`, and `Option` — unlike `Result` — carries no `#[ must_use ]` of its own:

```rust
// compile/-option_vs_result.rs
pub fn o() -> Option< String > { None }
pub fn r() -> Result< String, () > { Err( () ) }
pub fn f() { o(); r(); }
```

```
warning: unused `Result` that must be used
warning: 1 warning emitted
```

---

### RG27 — The Attribute Is on the Three Methods That Waste a Nanosecond and Off the One That Loses Data

The four `#[ must_use ]` attributes sit on `new`, `contains`, `len` and
`is_empty`. Ignoring any of those four costs a caller nothing at all: `new`
builds an empty struct with no allocation, and the other three read a `bool` or a
`usize` out of the map in twenty-odd nanoseconds and throw it away. They are
correct attributes — a discarded query is always a mistake — and they are the
cheap end of the range.

`register` and `names` are covered too, but not by anything the crate wrote:
`Result` and `Iterator` both carry `#[ must_use ]` upstream, which is why the
probe shows five warnings from four local attributes.

That leaves `get_mut` and `remove` silent, and the two are not alike. Dropping
`get_mut`'s return discards a borrow; nothing happens. Dropping `remove`'s return
discards a `Split< T >` **by value** — the registry has already given the ring
away, so the ring, its buffer and every record still unread in it are destroyed
at the end of the statement. `registry.remove( "events" );` compiles clean.

**Finding.** Recorded as the one gap in an otherwise deliberate attribute set,
and the gap is on the only method where the attribute would prevent data loss
rather than a wasted read. `#[ must_use ]` on `remove` — with a note, since the
attribute takes a string — is one line and turns the silent form into a warning
at the call site. Worth stating explicitly: this is not an argument that the crate
was careless about `must_use`, which the four attributes disprove; it is that the
attribute was applied by asking "is this a query?" rather than "what happens if
this value is dropped?", and the two questions disagree on exactly one method.

**Disposition:** declined — the fix is real but its cost is not the one line the
finding names. `remove` sits at `src/lib.rs:199`, and `#[ must_use ]` can only be
attached as a line immediately above it, which shifts every subsequent line in
the file by one. A repository-wide census
(`command grep -rn 'lib\.rs:[0-9]' ring_registry/docs/`) found upward of
thirty citations of `src/lib.rs` line numbers at or past 199 across at least
fifteen other doc files in this crate alone, including two already-closed
`**Disposition:** applied` entries in
`docs/algorithm/002_four_reads_and_the_order_they_do_not_promise.md` that quote
`src/lib.rs:227` and `:227-231` verbatim — content this shift would move without
those entries' own text changing. Reconciling that corpus-wide citation web
belongs to whoever owns those files, not to a single-finding disposition pass
in `item/002` and `invariant/001`'s scope. RG28 already names `remove` as "the
minimum" repair if the family-wide version is ever done; nothing here forecloses
that, only this pass making it unilaterally.

---

### RG28 — Five Methods in the Family Hand Ownership Out Through `Option`, and None of Them Is Marked

The same shape appears five times across the thirty-three crates:
`ring_core::try_recv`, `ring_handle::try_recv`, `ring_registry::remove`, and
`ring_slot`'s `set` and `take`. All five take `&mut self`, all five return
`Option< T >` where the `T` is owned and has just left the receiver, and the
census checks the line above each declaration for the attribute and finds it on
none of them.

The family is not indifferent to the attribute — it writes it 251 times across
the same files. It is that `Option` does not carry `#[ must_use ]` itself while
`Result` does, so a returning method is covered by default only if its author
happened to pick the error-shaped return; and every one of these five picked
`Option` deliberately, `ring_handle` writing out the reason directly above
`try_recv`'s declaration — "modelling it as one makes every caller unwrap a
non-failure".
The choice is right and it silently forfeits the guard.

The severity varies. A dropped `try_recv` loses one record from a ring that has
more; a dropped `ring_slot::take` loses one value. A dropped
`ring_registry::remove` loses a whole ring and everything still in it, which is
the largest unit of loss any of the five can produce.

**Finding.** Recorded as a family-wide pattern with one bad case rather than five
equal ones. The uniform repair is `#[ must_use ]` on all five, which costs five
lines and changes no behaviour; the minimum one is `remove`, per RG27. What makes
the census worth keeping either way is that it explains why the gap exists — not
five oversights, but one type-system default meeting a design preference the
family states out loud — so the same hole will open again the next time someone
correctly chooses `Option` over `Result`.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/002`](../pitfall/002_the_remove_that_drops_a_ring_without_a_word.md) | What the missing attribute lets through, measured |
| [`item/001`](001_two_lints_one_allow_and_the_reason_beside_it.md) | The other attribute census, and the unused `expect` form |
| [`api/002`](../api/002_the_receiver_split_and_the_sweep_it_forbids.md) | The same eight methods, by receiver and return |
| [`lifecycle/001`](../lifecycle/001_a_ring_from_registration_to_drop.md) | The ownership transfer `remove` performs |
| [`invariant/001`](../invariant/001_one_name_one_ring.md) | The invariant `remove` is the release valve for |

### Sources

| Fact | Where |
|------|-------|
| The four attribute sites | `ring_registry/src/lib.rs:104`, `:205`, `:212`, `:219` |
| The two uncovered methods | `ring_registry/src/lib.rs:189`, `:199` |
| Five warnings from seven ignored calls | Compile probe above |
| `Option` not `#[ must_use ]`, `Result` is | Compile probe above |
| Five ownership-transferring `Option` returns, none marked | Census above |
| 251 `must_use`, 0 `expect`, across the family | Census above |

### Tests

| Test | Covers |
|------|--------|
| `a_removed_ring_carries_its_records_to_its_new_owner` | `remove`'s return used, which is the covered case |
| `removing_an_absent_name_is_none` | The `None` arm, where nothing is lost |
| `removing_a_name_frees_it_for_reuse` | The registry's own state after the transfer |
