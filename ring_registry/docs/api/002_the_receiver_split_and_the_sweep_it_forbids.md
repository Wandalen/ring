# API: The Receiver Split and the Sweep It Forbids

### Scope

**Purpose:** Read the eight declarations as one table — receiver against stored
type — and record the two things that table shows and the crate's prose does not:
where `T` enters the surface, and what the surface cannot express at all.

**Responsibility:** The receiver of each method and whether its signature names
`Split< T >`; what every `&self` method returns; the absence of any collection
accessor; and the measured cost of the sweep a caller must write instead.

**In Scope:** `ring_registry/src/lib.rs:105`, `:157-163`, `:189`, `:199`,
`:206`, `:213`, `:220`, `:232`; every `.names()` call site in the workspace.

**Out of Scope:** The attributes on these declarations are
[`item/002`](../item/002_eight_declarations_and_four_must_use.md). The signature
decisions themselves are [`api/001`](001_the_registry_surface.md). The
`ring_handle` premise behind "no immutable `get`" is
[`integration/002`](../integration/002_what_ring_handle_requires_and_this_crate_does_not.md).

---

## Eight Declarations as One Table

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every declaration, its receiver, and whether it names the stored type --'
# `register` spans seven lines, so each declaration is accumulated from its
# `pub fn` up to the opening brace rather than read off a single line
awk '
  /^  pub fn/ { buf = $0; name = $3; sub( /\(.*/, "", name ); open = 1 }
  open && !/^  pub fn/ { buf = buf " " $0 }
  open && /^  \{/ {
    recv = ( buf ~ /& *mut self/ ) ? "&mut self" : ( buf ~ /&self/ ? "&self" : "none" )
    ring = ( buf ~ /Split/ ) ? "Split< T >" : "--"
    printf "    %-10s %-10s %s\n", name, recv, ring
    open = 0
  }
' ring_registry/src/lib.rs
echo '  -- what every &self method returns --'
command grep '^  pub fn.*( &self' ring_registry/src/lib.rs | sed 's/.*-> //' | sort -u | sed 's/^/    /'
echo '  -- and whether the rings are reachable as a collection --'
printf '    methods named values, values_mut, iter, iter_mut, drain or retain: %s\n' \
  "$( command grep -c -e 'pub fn values\|pub fn iter\|pub fn drain\|pub fn retain' ring_registry/src/lib.rs || true )"
printf '    names() call sites in the whole workspace: %s\n' \
  "$( command grep -rc '\.names()' --include=*.rs . 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
echo '  -- and whether the missing question got written down --'
command grep '^### Pending 4' ring_registry/docs/decisions/readme.md | sed 's/^/    /'
```

Live output:

```
  -- every declaration, its receiver, and whether it names the stored type --
    new        none       --
    register   &mut self  Split< T >
    get_mut    &mut self  Split< T >
    remove     &mut self  Split< T >
    contains   &self      --
    len        &self      --
    is_empty   &self      --
    names      &self      --
  -- what every &self method returns --
    bool
    impl Iterator< Item = &str >
    usize
  -- and whether the rings are reachable as a collection --
    methods named values, values_mut, iter, iter_mut, drain or retain: 0
    names() call sites in the whole workspace: 3
  -- and whether the missing question got written down --
    ### Pending 4 — Should the registry expose a way to visit every ring?
```

## Visiting Every Ring

```rust
// compile/-sweep.rs
// The obvious form: iterate the names, borrow each ring.
pub fn sweep( r : &mut Registry< u32 > )
{
  for n in r.names() { let _ = r.get_mut( n ); }
}

// Collecting first does not help: the names are borrowed from the map.
pub fn sweep_via_borrowed_names( r : &mut Registry< u32 > )
{
  let names : Vec< &str > = r.names().collect();
  for n in &names { let _ = r.get_mut( n ); }
}

// The form that compiles: pay for a String per ring first.
pub fn sweep_via_owned_names( r : &mut Registry< u32 > )
{
  let names : Vec< String > = r.names().map( String::from ).collect();
  for n in &names { let _ : Option< &mut Split< u32 > > = r.get_mut( n ); }
}
```

```
error[E0502]: cannot borrow `*r` as mutable because it is also borrowed as immutable
  --> compile/-sweep.rs:11:13
   |
 9 |   for n in r.names()
   |            ---------
   |            |
   |            immutable borrow occurs here
   |            immutable borrow later used here
10 |   {
11 |     let _ = r.get_mut( n );
   |             ^^^^^^^^^^^^^^ mutable borrow occurs here

error[E0502]: cannot borrow `*r` as mutable because it is also borrowed as immutable
  --> compile/-sweep.rs:21:13
   |
18 |   let names : Vec< &str > = r.names().collect();
   |                             - immutable borrow occurs here
19 |   for n in &names
   |            ------ immutable borrow later used here
20 |   {
21 |     let _ = r.get_mut( n );
   |             ^^^^^^^^^^^^^^ mutable borrow occurs here

error: aborting due to 2 previous errors
```

The third form compiles. Priced against the `values_mut()` sweep the underlying
`HashMap` already supports, over the same populations, median of nine:

```
     4 rings  values_mut sweep   0 allocations, 0 bytes   median 9.2 ns/sweep  min 9.1  max 9.3
     4 rings  names-first sweep  5 allocations, 124 bytes   median 272.5 ns/sweep  min 270.3  max 277.8
     4 rings  the missing accessor costs  +263.3 ns/sweep, +2861%
    16 rings  values_mut sweep   0 allocations, 0 bytes   median 30.6 ns/sweep  min 30.5  max 31.8
    16 rings  names-first sweep  17 allocations, 496 bytes   median 1095.8 ns/sweep  min 1079.3  max 1107.2
    16 rings  the missing accessor costs  +1065.2 ns/sweep, +3483%
```

---

### RG7 — `T` Enters the Surface Exactly at `&mut self`, and the Crate Argues That Point From Another Crate Instead

Eight declarations, and the table sorts itself. `new` takes no receiver. Three
methods take `&mut self` — `register`, `get_mut`, `remove` — and all three name
`Split< T >`. Four take `&self` — `contains`, `len`, `is_empty`, `names` — and
not one of them mentions `T` anywhere, in an argument or a return. What they
return is `bool`, `usize`, and `impl Iterator< Item = &str >`.

So immutably, `Registry< T >` is not a map at all. It is a set of names, with
`HashSet`'s query surface under one rename — `contains`, `len`, `is_empty`, and
`names` where a set would say `iter`. The generic parameter is invisible from
`&self`; a caller holding `&Registry< T >` can learn everything about which names
are taken and nothing whatever about what is under them.

That is the same conclusion `api/001` reaches in ten lines of prose and the
module doc restates and `decisions/readme.md` records as Closed 3 — "there is no
immutable `get`" — and all three reach it the same way, through `Split`'s lack
of a `&self` method. That premise is true and it lives in `ring_handle`, where
nothing here checks it
([RG20](../integration/002_what_ring_handle_requires_and_this_crate_does_not.md)).
The signature table is a second route to the same place that never leaves this
file: an immutable `get` would be the first and only `&self` method to name `T`,
breaking a line the surface currently draws without exception.

**Finding.** Recorded as an available argument the crate does not make. The two
routes are not equivalent — one depends on a fact about a dependency that no test
guards, the other on a property of this file's own eight signatures, visible to
anyone reading them and checkable by the census above. One sentence in
`api/001`'s operations table — that every `&self` method is `T`-free, and that a
`get` would be the exception — makes the design line explicit and makes it survive
a change in `ring_handle` that
[RG20](../integration/002_what_ring_handle_requires_and_this_crate_does_not.md)
shows nothing else would notice.

---

### RG8 — The Surface Cannot Reach the Rings as a Collection, and the Borrow Checker Prices the Workaround

`names()` is the only way to learn what the registry holds, and it returns
`impl Iterator< Item = &str >` borrowed from `&self`. `get_mut` needs `&mut self`.
The two do not compose:
`for n in r.names() { r.get_mut( n ); }` is `error[E0502]`, and collecting into a
`Vec< &str >` first does not help, because those `&str`s are the map's own keys
and hold the immutable borrow open. The census finds zero methods named `values`,
`values_mut`, `iter`, `iter_mut`, `drain` or `retain`, so there is no accessor
that sidesteps the conflict.

What compiles is materializing the names as owned `String`s and looking each one
up again. Measured against the `values_mut()` sweep the wrapped `HashMap` already
supports: at four rings, five allocations and 124 bytes where there were none,
and 272 ns against 9; at sixteen, seventeen allocations and 496 bytes, and 1.09 µs
against 31. The allocation count is one per ring plus the `Vec`, and the time is
dominated by it — the caller pays a `String` to be told a name it already had, and
then pays a hash to find a ring the iterator had just walked past.

Nobody is paying this today. All three `.names()` call sites in the workspace are
immutable — two in this crate's tests, one in `ring_factory`'s — and each
collects `&str` and sorts. That is the honest severity: this is a shape the
surface forbids rather than a cost anyone is bearing, recorded before a consumer
needs it. But "visit every ring" is not an exotic request of a registry of rings —
flushing all of them, draining all of them at shutdown, reporting depth per ring
are the operations a named collection exists to make possible, and each of them is
this loop.

**Finding.** Recorded as a missing Pending question rather than a missing method.
`decisions/readme.md` carries three, all of the form "would a consumer want X?",
and [RG15](../decisions/002_three_pending_questions_and_the_one_consumer.md)
notes that all three defer to a consumer that already exists. This is a fourth of
exactly that shape and it was never written down — and unlike the other three it
arrives with a price already measured, which is what would let it be settled
rather than deferred. Whether to add `values_mut` is a real question with an
argument on each side; leaving the question unasked while the answer costs
thirty-fold is the part worth changing.

**Disposition:** applied — `decisions/readme.md` now carries "Pending 4 —
Should the registry expose a way to visit every ring?", in the same
What-is-undecided/Why-it-is-not-decided/What-would-settle-it shape as
Pending 1-3, citing this entry's own measured thirty-fold sweep cost as the
price of leaving it unaddressed. `decisions/002`'s own recipe is scoped to
`### Pending [1-3]` so its "three Pending entries" purpose stays accurate;
Pending 4 is this entry's, not that instance's.
Now prints: `Should the registry expose a way to visit every ring`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/001`](001_the_registry_surface.md) | The same eight, by guarantee and precondition |
| [`item/002`](../item/002_eight_declarations_and_four_must_use.md) | The same eight, by attribute |
| [`integration/002`](../integration/002_what_ring_handle_requires_and_this_crate_does_not.md) | The cross-crate premise this table replaces |
| [`decisions/002`](../decisions/002_three_pending_questions_and_the_one_consumer.md) | The three Pending questions this would be the fourth of |
| [`invariant/002`](../invariant/002_what_survives_a_refusal_and_what_does_not.md) | The other thing the surface cannot report |

### Sources

| Fact | Where |
|------|-------|
| The eight declarations and their receivers | `ring_registry/src/lib.rs:105`, `:157-163`, `:189`, `:199`, `:206`, `:213`, `:220`, `:232` |
| Every `&self` return, none naming `T` | Census above |
| No collection accessor of any kind | Census above |
| `names()` and `get_mut` do not compose | Compile probe above |
| The sweep's allocation and time cost | Probe above, two runs |
| Three `.names()` call sites, all immutable | Census above |

### Tests

| Test | Covers |
|------|--------|
| `names_lists_every_live_name` | The one `&self` method that names anything |
| `a_registered_ring_is_retrievable_by_its_name_and_by_no_other` | `get_mut`, one name at a time |
| `two_names_hold_two_distinct_rings` | Two rings reached the only way the surface allows |
