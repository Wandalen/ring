# api

Eight declarations, and they sort themselves by receiver. `new` takes none.
Three take `&mut self` — `register`, `get_mut`, `remove` — and every one of them
names `Split< T >`. Four take `&self` — `contains`, `len`, `is_empty`, `names` —
and not one of them mentions `T` anywhere, returning `bool`, `usize`, and
`impl Iterator< Item = &str >`. Immutably, `Registry< T >` is not a map; it is a
set of names wearing `HashSet`'s query surface under one rename.

The two findings on the surface's shape are both about that line, from opposite
sides. It is the argument the crate wants for "there is no immutable `get`" and
does not make, reaching instead for a fact about `ring_handle` that nothing here
checks. And it is why nothing can visit the rings as a collection: `names()`
borrows `&self`, `get_mut` needs `&mut self`, the two do not compose, and the
workaround costs thirty-fold. The other two findings are about what `register`'s
signature charges — a `Debug` bound with three sources where the file names one,
and a refusal that hands back one of its two by-value arguments without saying
which.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_registry_surface.md) | The Registry Surface | The eight operations, the `Debug` bound's three sources, and what a refusal returns |
| [002](002_the_receiver_split_and_the_sweep_it_forbids.md) | The Receiver Split, and the Sweep It Forbids | `&mut self` versus `&self`, and the collection accessor the surface does not offer |

## A Bound With Three Sources and One Sentence

The file charges the `Debug` bound to `.unwrap()`/`.expect()` sites, and the
mechanism is right: `Result::expect` requires `E : Debug`, and `E` is
`( RegistryError, Split< T > )`. But `#[ derive( Debug ) ]` on `Registry< T >`
generates `impl< T : Debug > Debug for Registry< T >`, so a non-`Debug` record
type produces three compile errors, not one — the documented `.expect()`, a
`println!( "{r:?}" )` on the registry, and a `#[ derive( Debug ) ]` struct that
holds one. The last two are paid by callers who never call `register`.

The asymmetry is what makes it worth writing down: `Default` on the same struct
was hand-written specifically to dodge a spurious `T : Default` bound, two lines
away. One derive was reasoned about and replaced; the other was left, and its
cost was then attributed to a different part of the API.

## The Registry Cannot Be Swept, and Nobody Has Tried

`for n in r.names() { r.get_mut( n ); }` is `error[E0502]`, and collecting into a
`Vec< &str >` does not help — the keys are the map's own and hold the immutable
borrow open. There is no `values`, `values_mut`, `iter`, `drain` or `retain`.
What compiles is materializing owned `String`s and looking each name up again:
against the `values_mut()` sweep the wrapped `HashMap` already supports, four
rings cost five allocations and 272 ns against zero and 9 ns; sixteen cost
seventeen allocations and 1.09 µs against zero and 31 ns.

All three `.names()` call sites in the workspace are immutable, so nobody pays
this today. But flush every ring, drain every ring at shutdown, report depth per
ring — the operations a named collection exists to make possible — are each this
loop, and the question was never added to the three Pending ones that already
defer to a consumer.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every declaration, its receiver, and whether it names T --'
awk '
  /^  pub fn/ { n = $3; sub( /\(.*/, "", n ); buf = $0; inbuf = 1; next }
  inbuf && /^  \{/ {
    r = index( buf, "&mut self" ) ? "&mut self" : index( buf, "&self" ) ? "&self    " : "—        "
    printf "    %-10s %s  names T: %s\n", n, r, ( index( buf, "Split< T >" ) ? "yes" : "no" ); inbuf = 0; next }
  inbuf { buf = buf " " $0 }
' ring_registry/src/lib.rs
echo '  -- collection accessors on the surface --'
printf '    values / values_mut / iter / drain / retain: %s\n' \
  "$( command grep -c 'pub fn values\|pub fn iter\|pub fn drain\|pub fn retain' ring_registry/src/lib.rs || true )"
echo '  -- and every .names() call site in the workspace --'
command grep -rn '\.names()' --include=*.rs */ | cut -c1-84
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RG5 | `ring_registry` | **misleading doc** | `api/001` states the `Debug` bound's cost precisely and attributes it to one place — "a `Debug` bound at the `.unwrap()`/`.expect()` sites" — while `#[ derive( Debug ) ]` on `Registry< T >` at `src/lib.rs:87` generates `impl< T : Debug > Debug for Registry< T >` and imposes it twice more: compiled, a non-`Debug` record type yields **three** `E0277`s, the documented `.expect()` site plus `println!( "{r:?}" )` on a registry and a `#[ derive( Debug ) ]` struct that embeds one, the last two reached by callers who never call `register` and never see a `Result`; the inconsistency behind it is that `Default` on the same struct was hand-written two lines away specifically to avoid the identical spurious-bound hazard (RG47), so one derive was reasoned about and replaced while the other was left and its cost reattributed |
| RG6 | `ring_registry` | **misleading doc** | The file's longest argument is about what a refusal returns — the ring comes back, because swallowing it "would make 'you chose a name that was taken' destroy data" — and `register`'s *other* by-value argument is never mentioned: `self.rings.entry( name )` consumes the name and the error is built from `occupied.key().clone()`, measured with a caller's `String` of capacity 64 as **one allocation, one deallocation, and a reported name of capacity 6**; nothing observable differs today because no normalization exists and every `register` call in the suite passes a literal identical to the registered one, so `tests/registry_test.rs:125` passes whichever copy the error reports; the same measurement prices an undocumented lever — an owned `String` into a free name is **zero** allocations, into a taken name one, and a `&str` two — and the cheapest shape the API offers is the one nothing demonstrates |
| RG7 | `ring_registry` | n/a — observation | The receiver split is exact: three `&mut self` methods all name `Split< T >`, four `&self` methods name `T` nowhere and return `bool`/`usize`/`impl Iterator< Item = &str >`, so immutably a `Registry< T >` is `HashSet`'s query surface under one rename and an immutable `get` would be the first `&self` method ever to mention `T`; the crate reaches the same conclusion three times — `api/001`, the module doc, and Closed 3 — always through `Split`'s lack of a `&self` method, a true premise living in `ring_handle` that nothing here guards (RG20), where this file's own eight signatures give the identical conclusion without leaving the crate; one sentence in the operations table makes the line explicit and makes it survive a change in the dependency |
| RG8 | `ring_registry` | **measured cost** | Nothing on the surface reaches the rings as a collection: `names()` borrows `&self` and `get_mut` needs `&mut self`, so `for n in r.names() { r.get_mut( n ) }` is `error[E0502]` and a `Vec< &str >` does not help because the keys hold the borrow open, and the census finds zero `values`/`values_mut`/`iter`/`drain`/`retain`; the compiling workaround materializes owned `String`s and re-hashes each, costing at four rings five allocations, 124 bytes and 272 ns against the wrapped `HashMap`'s zero-allocation 9 ns `values_mut()` sweep, and at sixteen rings seventeen allocations, 496 bytes and 1.09 µs against 31 ns — roughly 29–35× — with all three workspace `.names()` sites immutable so nobody bears it today; recorded as a missing fourth Pending question rather than a missing method, since the three that exist are all "would a consumer want X?" and this one arrives with its price already measured |
