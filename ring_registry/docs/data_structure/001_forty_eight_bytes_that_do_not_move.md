# Data Structure: Forty-Eight Bytes That Do Not Move

### Scope

**Purpose:** Measure every width the crate reasons about — the struct, the value
it stores, the error, and the `Result` that carries both — across a range of `T`
spanning four kilobytes, against a doc that states one of them as a lower bound.

**Responsibility:** `Registry< T >`'s single field; `size_of` for `Registry`,
`Split`, `RegistryError` and both `Result` shapes; and the heap the first
`register` claims.

**In Scope:** `ring_registry/src/lib.rs:87-91`, `:123-142`;
`ring_registry/docs/api/001_the_registry_surface.md:62`, `:82`.

**Out of Scope:** Why the error carries a payload at all is
[`pattern/001`](../pattern/001_an_error_that_hands_the_payload_back.md). The
choice of map is
[`data_structure/002`](002_the_only_map_in_thirty_three_crates.md). The time each
call takes is [`algorithm/001`](../algorithm/001_two_branches_and_what_the_refusal_costs.md).

---

## One Field, and the Three Places the Crate Prices It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the whole data structure --'
command grep -m1 -A4 -F '#[ derive( Debug ) ]' ring_registry/src/lib.rs
echo '  -- the two sentences the crate writes about its width --'
command grep 'exactly 448 bytes\|cross this boundary either way\|every caller.s .Result. is that' ring_registry/src/lib.rs | sed 's/^/    /'
echo '  -- and what api/001 tells a reader the number is --'
# Anchored at column 0: api/001 quotes its own table inside indented regenerate
# output further down, and an unanchored search finds that copy too.
command grep '^\*\*It also costs a .Result. 448 bytes\|^| \*\*Current\*\* |' ring_registry/docs/api/001_the_registry_surface.md | sed 's/^/    /'
```

Live output:

```
  -- the whole data structure --
#[ derive( Debug ) ]
pub struct Registry< T >
{
  rings : HashMap< String, Split< T > >,
}
  -- the two sentences the crate writes about its width --
      /// **exactly 448 bytes**, fixed across every `T` because `Split< T >` is a
      ///   `Split< T >` by value, so the 448 bytes cross this boundary either way.
      /// What the lint does cost, honestly stated: every caller's `Result` is that
  -- and what api/001 tells a reader the number is --
    **It also costs a `Result` 448 bytes wide, and a suppressed lint.** That number
    | **Current** | One call; the ring comes back whole, the name is consumed and reported by copy; a `Debug` bound at `.expect()` sites, a 448-byte `Result`, and one scoped `allow` |
```

## Every Width, Measured

*These widths were captured once, from a scratch binary under `-rg_probe/`
that no longer exists — swept, like every hyphen-prefixed directory in this
project, once the session that produced it ended. Reading `Registry`,
`Split< T >` and `RegistryError`'s current field layouts turns up nothing
that would move any of these numbers, but a static read is not `size_of`:
treat the figures below as a historical measurement this doc cannot
re-verify, rather than a live one.*

```rust
// -rg_probe/src/bin/sizes.rs — size_of and align_of over the four types
println!( "  Registry< u32 >              {:3} bytes  align {}", size_of::< Registry< u32 > >(), align_of::< Registry< u32 > >() );
```

```
  Registry< u32 >               48 bytes  align 8
  Registry< [ u8; 4096 ] >      48 bytes  align 8
  Split< u32 >                 384 bytes
  RegistryError                 24 bytes
  ( RegistryError, Split<u32> )448 bytes
  Result< (), ( RegistryError, Split<u32> ) > 448 bytes
  Result< (), RegistryError >   24 bytes
  an empty Registry< u32 > holds 0 names; after one register, 1
```

And the same two widths across a range of record types spanning four kilobytes:

```
    T = u8               Split< T >    384   Result< (), ( RegistryError, Split< T > ) >    448
    T = u32              Split< T >    384   Result< (), ( RegistryError, Split< T > ) >    448
    T = u64              Split< T >    384   Result< (), ( RegistryError, Split< T > ) >    448
    T = [ u8; 64 ]       Split< T >    384   Result< (), ( RegistryError, Split< T > ) >    448
    T = [ u8; 4096 ]     Split< T >    384   Result< (), ( RegistryError, Split< T > ) >    448
    the error alone         RegistryError     24   Result< (), RegistryError >                  24
```

## What the Same Call Puts on the Heap

The counting allocator of
[`algorithm/001`](../algorithm/001_two_branches_and_what_the_refusal_costs.md),
with the ring built outside the measured window so the numbers are `register`'s
own:

```
    Registry::new()                      0 allocations, 0 bytes
    register into a free name            2 allocations, 1810 bytes
    register into a taken name (refused) 2 allocations, 12 bytes
    contains + get_mut + len + names(1) 0 allocations, 0 bytes
```

---

### RG9 — "At Least 448 Bytes" Is a Constant, and the Doc Says So Ten Lines Later

`src/lib.rs:126` introduces the width as "**at least 448 bytes**, because it
carries a whole `Split< T >` — measured, not estimated, by the lint itself". The
provenance is honest: clippy reported that figure for the instantiation it
compiled. The words around it are not. "At least … because it carries a whole
`Split< T >`" reads as a floor that rises with the record type, and that is how
[`api/001:62`](../api/001_the_registry_surface.md) and its comparison table at
`:82` both restate it — as *the* number, with no range.

Measured for `T` in `u8`, `u32`, `u64`, `[ u8; 64 ]` and `[ u8; 4096 ]`, the
`Result` is 448 bytes every time and `Split< T >` is 384 bytes every time. The
type parameter never appears in the width, because `Ring< T >` holds its records
behind a pointer rather than inline; a four-kilobyte record type moves the heap
allocation, not the struct. `Registry< T >` itself is 48 bytes — a `HashMap` and
nothing else — for `u32` and for `[ u8; 4096 ]` alike.

The crate already knows this. Ten lines below the "at least", `:136` argues
against boxing on the grounds that "`register` takes the same `Split< T >` by
value, so the 448 bytes cross this boundary either way" — a statement that is
only true if 448 is a constant, which is exactly what the "at least" denies.

**Finding.** Recorded as a hedge that inverts its own conclusion. A reader
budgeting for `Registry< [ u8; 4096 ] >` reads "at least 448 because it carries a
whole `Split< T >`" and reserves four kilobytes; the real answer is 448, the same
as for `u8`. The repair is one word — "exactly 448 bytes" — plus the reason,
which is more interesting than the hedge: the width is fixed because
`Split< T >` is a handle, so the lint fires once for the whole family of
instantiations rather than differently for each.

**Disposition:** applied — `src/lib.rs:126` now reads "exactly 448 bytes,
fixed across every `T` because `Split< T >` is a handle, not the record",
replacing the "at least... because it carries a whole `Split< T >`" hedge; the
comparison table's own three-line-later restatement already treated 448 as a
constant, so this removes the contradiction rather than creating a new claim.
Now prints: `exactly 448 bytes**, fixed across every`

---

### RG10 — The Width Discussion Prices the 448 Stack Bytes and Never Names the 1810 Heap Bytes

Twenty lines of doc comment argue about the `Result`'s 448 bytes: which lint
fires, why shrinking is refused, why boxing is refused, and that the width is
paid on the `Ok` path too. It is a careful accounting of one number.

The same call makes two heap allocations totalling **1810 bytes** the first time
it succeeds, and none of them is mentioned anywhere in the crate. `Registry::new`
allocates nothing — `HashMap::new` builds no table — so the whole cost of the
map's first table lands on the first `register`, sized for 384-byte values. That
is four times the stack width the doc spends twenty lines on, on the same call,
in the same operation.

Neither number is a problem: `register` is a setup-time call and 1810 bytes once
is nothing. What the asymmetry costs is a reader's model of where the cost is.
The doc's own framing — "what the lint does cost, honestly stated" — invites the
reading that the 448 bytes are the cost, when they are the smaller and more
constant part of it, and the part a caller cannot avoid by any means the doc
discusses. Boxing the error, the remedy `:134` refuses because "that allocates on
the failure path", would add one allocation to a path measured at two.

**Finding.** Recorded as an incomplete cost statement rather than a wrong one.
One line beside the existing accounting — that the first successful `register`
allocates the map's table, measured at 1810 bytes, and that `Registry::new`
allocates nothing — completes the picture and costs nothing to keep true, since
it does not vary with `T` any more than the 448 does.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/002`](002_the_only_map_in_thirty_three_crates.md) | The `HashMap` that is the whole 48 bytes |
| [`algorithm/001`](../algorithm/001_two_branches_and_what_the_refusal_costs.md) | The two allocations on the refusal path |
| [`pattern/001`](../pattern/001_an_error_that_hands_the_payload_back.md) | Why the `Result` carries a `Split< T >` at all |
| [`api/001`](../api/001_the_registry_surface.md) | Where the number is restated without its range |
| [`decisions/001`](../decisions/001_four_closed_questions_and_the_one_measurement_none_took.md) | Closed 2, the decision this width records |

### Sources

| Fact | Where |
|------|-------|
| The single field | `ring_registry/src/lib.rs:87-91` |
| "at least 448 bytes" | `ring_registry/src/lib.rs:126` |
| "the 448 bytes cross this boundary either way" | `ring_registry/src/lib.rs:136` |
| The number restated in the surface doc | `ring_registry/docs/api/001_the_registry_surface.md:62`, `:82` |
| 384 and 448 constant across five record types | Probe above |
| 48 bytes for `Registry< T >`, both extremes | Probe above |
| 0 allocations at `new`, 1810 bytes at first `register` | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `a_refused_registration_hands_the_ring_back` | The 448-byte `Err` variant, destructured |
| `two_names_hold_two_distinct_rings` | The map holding more than one 384-byte value |
| `an_empty_registry_is_empty` | The state before the table is allocated |
