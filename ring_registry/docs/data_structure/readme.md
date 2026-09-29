# data_structure

One field. `Registry< T >` is a `HashMap< String, Split< T > >` and nothing
else — 48 bytes on the stack for every `T` from `u8` to `[ u8; 4096 ]`, because
`Split< T >` is a handle rather than a buffer and the records live behind a
pointer. Every property the type has, it has because `HashMap` has it: the
lookup cost, the unspecified iteration order, the teardown order, the `std`
dependency, and the one allocation that does not happen until the first
`register` succeeds.

The two instances here take the two halves of that. The first is about width —
the 448-byte `Result` the crate argues about for twenty lines, hedged as a floor
when it is a constant, alongside 1810 heap bytes on the same call that nothing
mentions. The second is about the map itself: the only one in thirty-three
crates, the family's other structural `std` dependency, and a wrapper whose
`impl` block is looser than the type it wraps.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_forty_eight_bytes_that_do_not_move.md) | Forty-Eight Bytes That Do Not Move | The width of `Registry`, `Split` and the `Result`, measured across five record types |
| [002](002_the_only_map_in_thirty_three_crates.md) | The Only Map in Thirty-Three Crates | The `HashMap` choice, the `std` it pulls in, and the bound the wrapper drops |

## A Constant Written as a Floor

`src/lib.rs:126` prices the error `Result` at "**at least 448 bytes**, because it
carries a whole `Split< T >`". The provenance is honest — clippy printed that
number — but "at least … because it carries a whole `Split< T >`" reads as a
floor that rises with the record type, and `api/001` restates it as *the* number
with no range. Measured for `u8`, `u32`, `u64`, `[ u8; 64 ]` and `[ u8; 4096 ]`,
it is 448 every time, because `Ring< T >` holds its records behind a pointer.

Ten lines below the hedge, the same doc argues against boxing on the grounds
that "`register` takes the same `Split< T >` by value, so the 448 bytes cross
this boundary either way" — a sentence that is only true if 448 is a constant,
which is what the hedge denies.

## The Map Is the Crate's Only Dependency on `std`

Across all thirty-three library crates, `HashMap` and `BTreeMap` appear in
exactly one `src/lib.rs`: this one, four times — the import, the field, the
constructor, and one doc comment. `alloc::collections::HashMap` does not exist,
so the map holds the crate to `std` the same way `ring_trace`'s `Mutex` holds
that one. Where they part is the substitute: `alloc::collections::BTreeMap`
compiles, and at the population a registry actually runs at it is the faster of
the two, so this crate's `std` requirement is removable by changing one type and
`ring_trace`'s is not.

The same file drops a bound its dependency keeps. `impl< T > Registry< T >`
carries none across all eight methods; `ring_handle` writes `T : Send` three
times in one file, on `Split`, `Producer` and `Drain`. `Registry< Cell< u32 > >`
therefore constructs with no diagnostic and is permanently empty, because the
only way to obtain a `Split< Cell< u32 > >` to put in it is `Split::new`, which
requires `Send`.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the struct, and the one field it holds --'
command grep -m1 -A4 -F '#[ derive( Debug ) ]' ring_registry/src/lib.rs
echo '  -- every map in the thirty-three library crates --'
command grep -rn 'HashMap\|BTreeMap' --include=lib.rs ring_*/src/ |
  sed 's|||' | cut -c1-86 | sed 's/^/    /'
echo '  -- the bound on the wrapper, and the bound on what it wraps --'
command grep -n '^impl< T' ring_registry/src/lib.rs | sed 's/^/    /'
command grep -n '^impl< T' ring_handle/src/lib.rs | sed 's/^/    /'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RG9 | `ring_registry` | **misleading doc** | `src/lib.rs:126` hedges the error `Result`'s width as "**at least 448 bytes**, because it carries a whole `Split< T >`", which reads as a floor rising with the record type and is restated as *the* number by `api/001:62` and its table at `:82`; measured for `T` in `u8`, `u32`, `u64`, `[ u8; 64 ]` and `[ u8; 4096 ]` the `Result` is **448 bytes every time**, `Split< T >` 384 every time and `Registry< T >` 48 every time, because `Ring< T >` holds its records behind a pointer so a four-kilobyte record moves the heap allocation rather than the struct; the crate already depends on the constancy ten lines later, where `:136` refuses boxing because "`register` takes the same `Split< T >` by value, so the 448 bytes cross this boundary either way" — true only if 448 does not vary — so the repair is one word plus the reason the hedge omits, which is that the lint fires once for the whole family of instantiations |
| RG10 | `ring_registry` | n/a — doc gap | Twenty lines of doc comment price the `Result`'s 448 stack bytes — which lint fires, why shrinking is refused, why boxing is refused, that the width is paid on `Ok` too — and the same call makes **two heap allocations totalling 1810 bytes** the first time it succeeds, mentioned nowhere in the crate: `Registry::new` allocates nothing because `HashMap::new` builds no table, so the whole cost of the first table, sized for 384-byte values, lands on the first `register`; neither number is a problem on a setup-time call, but the doc's framing — "what the lint does cost, honestly stated" — invites reading the 448 as *the* cost when it is the smaller and more constant part, and the refused remedy at `:134` would add one allocation to a path already measured at two |
| RG11 | `ring_registry` | **wrong doc** | `ring_trace`'s `non_functional_requirement/002` states of its `Mutex` "That makes this the family's only library crate whose `std` use is structural", two sentences after naming this crate's `HashMap` as the other candidate, and on the test that sentence implies the two are symmetric: `alloc::sync::Mutex` and `alloc::collections::HashMap` both fail with the same `error[E0425]`, so both crates are held to `std` by a type `alloc` does not have — `HashMap`/`BTreeMap` appear in exactly one of the thirty-three `src/lib.rs` files, this one, four times, two of them load-bearing; where they part is the substitute, since `alloc::collections::BTreeMap` compiles and is the faster type at registry population (RG4, RG16) while a lock has no `alloc` shape at all, so the repairs are one sentence each in two crates — `ring_trace` should claim no `alloc` substitute rather than sole structural use, and this crate should record that the map is the only thing standing between it and family-wide `no_std` eligibility |
| RG12 | `ring_registry` | n/a — observation | `impl< T > Registry< T >` carries no bound at all across all eight methods and `Default` is likewise unbounded, while `Split< T >` — the only thing a registry can hold — exposes both its methods behind `impl< T : Send > Split< T >` and every type downstream of it repeats the bound, `ring_handle` writing it three times in one file; the compile probe confirms `Registry< Cell< u32 > >` constructs with no diagnostic and is permanently empty, since the only source of a `Split< Cell< u32 > >` is `Split::new`, which requires `Send`, so the type admits a family of instantiations where `register` can never be called and `len` is a constant zero; nothing is unsound and a caller who tries reaches the real error at `Split::new` with the bound named, one crate away and one step later than necessary — adding `T : Send` to the inherent impl would move the error to the declaration site and make `Registry`'s signature say what it holds |
