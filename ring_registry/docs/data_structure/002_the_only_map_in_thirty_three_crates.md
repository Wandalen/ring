# Data Structure: The Only Map in Thirty-Three Crates

### Scope

**Purpose:** Place this crate's `HashMap` against the rest of the family — the
only associative container in thirty-three library crates — and test the two
claims that follow from it: which crate's `std` use is structural, and what bound
the wrapper carries.

**Responsibility:** The map census across `ring_*/src/lib.rs`; the `std::`
census across the same tree; whether `HashMap` and `BTreeMap` exist in `alloc`;
and the `T` bounds on `Registry` against those on `Split`.

**In Scope:** `ring_registry/src/lib.rs:43`, `:90`, `:93`, `:101`;
`ring_handle/src/lib.rs:69`;
`ring_trace/docs/non_functional_requirement/002_the_one_crate_that_genuinely_needs_std.md:102`.

**Out of Scope:** The widths are
[`data_structure/001`](001_forty_eight_bytes_that_do_not_move.md). What the map
costs against the ordered alternative is
[`algorithm/002`](../algorithm/002_four_reads_and_the_order_they_do_not_promise.md).

---

## One Map, Eight Callers of `std`

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every map in the thirty-three library crates --'
command grep -r 'HashMap\|BTreeMap' --include=lib.rs ring_*/src/ | sed 's|ring/||' | sed 's/^/    /'
echo '  -- and every crate that reaches for std at all --'
for c in ring_*/; do
  n=$( command grep -rc 'std::' --include=*.rs "$c"src/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )
  if [ "$n" -gt 0 ]; then printf '    %-18s std:: mentions in src/: %s\n' "$( basename "$c" )" "$n"; fi
done
echo '  -- the bound the wrapper does not carry --'
command grep '^impl< T' ring_registry/src/lib.rs | sed 's/^/    /'
command grep '^impl< T' ring_handle/src/lib.rs | sed 's/^/    /'
echo '  -- and whether this crate states its own removability --'
command grep "^\*\*This crate's .std. requirement is real today" \
  ring_registry/docs/data_structure/002_the_only_map_in_thirty_three_crates.md | sed 's/^/    /'
```

Live output:

```
  -- every map in the thirty-three library crates --
    ring_registry/src/lib.rs:use std::collections::HashMap;
    ring_registry/src/lib.rs:  rings : HashMap< String, Split< T > >,
    ring_registry/src/lib.rs:    Self { rings : HashMap::new() }
    ring_registry/src/lib.rs:  /// `HashMap` iteration order is unspecified and varies between one map and
  -- and every crate that reaches for std at all --
    ring_bench         std:: mentions in src/: 7
    ring_mpsc          std:: mentions in src/: 1
    ring_overflow      std:: mentions in src/: 1
    ring_registry      std:: mentions in src/: 2
    ring_spsc          std:: mentions in src/: 2
    ring_testkit       std:: mentions in src/: 2
    ring_tls           std:: mentions in src/: 1
    ring_trace         std:: mentions in src/: 3
    ring_types         std:: mentions in src/: 1
    ring_wait          std:: mentions in src/: 2
  -- the bound the wrapper does not carry --
    impl< T > Default for Registry< T >
    impl< T > Registry< T >
    impl< T : Send > Split< T >
    impl< T : Send > Producer< '_, T >
    impl< T : Send > Iterator for Drain< '_, '_, T >
  -- and whether this crate states its own removability --
    **This crate's `std` requirement is real today, and removable.** `HashMap` has
```

## Which of the Two Maps `alloc` Has

Three one-line library crates, each naming a type in `alloc` rather than `std`,
compiled with `--emit=metadata`:

```rust
// compile/-hashmap_from_alloc.rs
#![ no_std ]
extern crate alloc;
pub fn f() -> alloc::collections::HashMap< alloc::string::String, u32 > { todo!() }

// compile/-btreemap_from_alloc.rs
pub fn f() -> alloc::collections::BTreeMap< alloc::string::String, u32 >
{ alloc::collections::BTreeMap::new() }

// compile/-mutex_from_alloc.rs
pub fn f() -> alloc::sync::Mutex< u32 > { todo!() }
```

```
=== -btreemap_from_alloc ===
(no diagnostics)
=== -hashmap_from_alloc ===
error[E0425]: cannot find type `HashMap` in module `alloc::collections`
error: aborting due to 1 previous error
=== -mutex_from_alloc ===
error[E0425]: cannot find type `Mutex` in module `alloc::sync`
error: aborting due to 1 previous error
```

**This crate's `std` requirement is real today, and removable.** `HashMap` has
no `alloc` substitute; `BTreeMap` does, and is the family's own faster choice
at the population this crate holds. That substitute is what stands between
this crate and the family's `no_std` eligibility — not its absence, the way
`ring_trace`'s lock is absent one.

## A Registry Over a Non-`Send` Record Type

```rust
// compile/-nonsend.rs
use ring_registry::Registry;
use core::cell::Cell;

// A registry over a non-Send record type constructs fine.
pub fn f() -> Registry< Cell< u32 > > { Registry::new() }
```

```
=== -nonsend ===
(no diagnostics)
```

---

### RG11 — This Is the Family's Other Structural `std` Dependency, and It Has a Substitute

Across all thirty-three library crates, `HashMap` and `BTreeMap` appear in
exactly one `src/lib.rs`: this one, four times, of which one is the import, one
the field, one the constructor and one a doc comment. Eight crates name `std::`
at all, and most of those are threads or timing in test-support code;
`ring_registry`'s two mentions are the map and its `Entry`, both load-bearing.

[`ring_trace`'s `non_functional_requirement/002`](../../../ring_trace/docs/non_functional_requirement/002_the_one_crate_that_genuinely_needs_std.md)
states, of its `Mutex`, "That makes this the family's only library crate whose
`std` use is structural" — two sentences after naming `ring_registry`'s `HashMap`
as the other candidate. On the test that sentence implies, the two crates are
symmetric: `alloc::sync::Mutex` does not exist and `alloc::collections::HashMap`
does not exist, both giving the same `error[E0425]`, so both crates are held to
`std` by a type they cannot get from `alloc`.

Where they part is the substitute. `alloc::collections::BTreeMap` compiles, so
this crate's dependency on `std` is removable by changing one type — and, as
[`algorithm/002`](../algorithm/002_four_reads_and_the_order_they_do_not_promise.md)
measures, changing it to the faster type at the population a registry actually
holds. `ring_trace` has no such move; a lock is not in `alloc` in any shape.

**Finding.** Recorded as a family claim that is wrong in the direction that
matters, and a fact about this crate that nothing states. Two repairs, in
different crates and both one sentence: `ring_trace`'s claim should say its lock
has no `alloc` substitute, rather than that it is alone in needing `std`; and
this crate should say the same thing about its map, which is that the `std`
requirement is real today and is the only thing standing between the crate and
the family's `no_std` eligibility.

**Disposition:** applied — this crate's own half. A new sentence after the
`alloc` compile probe states plainly that the `std` requirement is real today
and that `BTreeMap` is the substitute standing between this crate and
`no_std` eligibility. `ring_trace`'s half of the repair is a sibling crate's
doc, out of this crate's scope.
Now prints: `This crate's `

---

### RG12 — The Wrapper Is Looser Than What It Wraps

`impl< T > Registry< T >` carries no bound at all — not `Send`, not `Debug`, not
`Sized` beyond the implicit one — across all eight methods, and `Default` is
likewise unbounded. `Split< T >`, the only thing a `Registry` can hold, exposes
both of its methods behind `impl< T : Send > Split< T >`, and every type
downstream of it — `Producer`, `Drain` — repeats that bound.

The compile probe confirms the consequence: `Registry< Cell< u32 > >` constructs
with no diagnostic. It is also permanently empty, because the only way to obtain
a `Split< Cell< u32 > >` to put in it is `Split::new`, which requires `Send`. So
the type admits a whole family of instantiations in which `register` can never be
called, `remove` can never return anything, and `len` is a constant zero.

Nothing here is unsound and nothing misleads at the point of use — a caller who
tries reaches the real error at `Split::new`, in `ring_handle`, with the bound
named. What it costs is one crate away and one step later than it needs to be,
and it costs the reader of `Registry`'s own documentation, which describes a
container for rings without saying that the rings it can contain are `Send` ones.

**Finding.** Recorded as an unstated narrowing rather than a defect. Adding
`T : Send` to the inherent impl would move the error to the declaration site and
make `Registry`'s signature say what it holds; leaving it off keeps `Registry<T>`
constructible for any `T`, which is only useful if something wants an empty one.
The family's own precedent is unambiguous — `ring_handle` writes the bound three
times in one file — and this crate is the one place the chain drops it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/001`](001_forty_eight_bytes_that_do_not_move.md) | The 48 bytes this map entirely accounts for |
| [`algorithm/002`](../algorithm/002_four_reads_and_the_order_they_do_not_promise.md) | What the `alloc` substitute would cost, measured |
| [`non_functional_requirement/001`](../non_functional_requirement/001_a_setup_time_call_and_what_that_licenses.md) | The crate's other premise, stated once and never measured |
| [`integration/002`](../integration/002_what_ring_handle_requires_and_this_crate_does_not.md) | The `Send` bound tracked across the edge |
| [`item/001`](../item/001_two_lints_one_allow_and_the_reason_beside_it.md) | The declaration-level attributes on the same type |

### Sources

| Fact | Where |
|------|-------|
| The import, the field and the constructor | `ring_registry/src/lib.rs:43`, `:90`, `:107` |
| The unbounded inherent impl | `ring_registry/src/lib.rs:93`, `:101` |
| `Split`'s `Send` bound, three times | `ring_handle/src/lib.rs:69`, `:125`, `:257` |
| "the family's only library crate whose `std` use is structural" | `ring_trace/docs/non_functional_requirement/002_the_one_crate_that_genuinely_needs_std.md` |
| `HashMap` and `Mutex` absent from `alloc`, `BTreeMap` present | Compile probes above |
| `Registry< Cell< u32 > >` constructs | Compile probe above |

### Tests

| Test | Covers |
|------|--------|
| `two_names_hold_two_distinct_rings` | The map holding more than one entry |
| `unusual_names_are_ordinary_names` | `String` keys with no constraint on content |
| `names_lists_every_live_name` | Iteration over the map's keys |
