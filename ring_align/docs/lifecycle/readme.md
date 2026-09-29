# Lifecycle Doc Definition

### Scope

- **Purpose**: Track the two things in this crate that have a life longer than a function call — a wrapped value, and the constant itself across a platform port.
- **Responsibility**: For each, give the states, the transitions, and the point at which the crate's guarantee is created or lost.
- **In Scope**: The arc of a `CacheAligned` value; the edit sequence a port requires.
- **Out of Scope**: The layout at any single moment, which is [`data_structure/`](../data_structure/readme.md); why the constant is unconditional, which is [`decisions/001`](../decisions/001_the_constant_is_not_conditional.md).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Wrapped Value's Arc](001_the_wrapped_values_arc.md) | Wrap, hold, move, unwrap — where the padding property comes into existence, what preserves it through a move, and the one transition that ends it | 🔄 |
| 002 | [The Constant Across a Platform Port](002_the_constant_across_a_platform_port.md) | The edit sequence raising `CACHE_LINE` to 128, both places it has to change, and everything that will keep passing either way | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every site in the crate that writes the line size --'
command grep -E '\b64\b' ring_align/src/lib.rs
echo '  -- of those, the ones the compiler reads --'
sed 's://.*::' ring_align/src/lib.rs | command grep -cE '\b64\b' || true
echo '  -- the transition that ends the guarantee --'
command grep 'pub fn into_inner' ring_align/src/lib.rs
echo '  -- what fails if a port raises one compiler-read site and not the other --'
command grep 'CACHE_LINE' ring_align/tests/align_test.rs
```

Live output:

```
  -- every site in the crate that writes the line size --
//! No `unsafe` is needed for any of it — `#[ repr( align( 64 ) ) ]` is a safe
/// 64 on x86-64 and on AArch64's common configuration. Apple Silicon uses 128,
/// and a value too small is the failure that matters — two cursors 64 bytes
/// assert_eq!( ring_align::CACHE_LINE, 64 );
pub const CACHE_LINE : usize = 64;
// family has used (64, 128) and unchecked until now (-> docs/algorithm/001
#[ repr( align( 64 ) ) ]
/// assert!( on_distinct_lines( 63, 64 ) );    // straddling the boundary
  -- of those, the ones the compiler reads --
2
  -- the transition that ends the guarantee --
  pub fn into_inner( self ) -> T
  -- what fails if a port raises one compiler-read site and not the other --
use ring_align::{ on_distinct_lines, CacheAligned, CACHE_LINE };
  assert_eq!( CACHE_LINE, 64 );
/// The direction of a future change to `CACHE_LINE` matters and nothing but
  // `CACHE_LINE >= 64` (both operands are literals today), which otherwise
    core::hint::black_box( CACHE_LINE ) >= 64,
    "CACHE_LINE dropped below 64 - every assertion in this suite would still \
  assert_eq!( core::mem::align_of::< CacheAligned< u8 > >(), CACHE_LINE );
  assert_eq!( core::mem::size_of::< CacheAligned< u8 > >(), CACHE_LINE );
  assert_eq!( core::mem::align_of::< CacheAligned< u64 > >(), CACHE_LINE );
  assert_eq!( core::mem::size_of::< CacheAligned< u64 > >(), CACHE_LINE );
  assert_eq!( core::mem::size_of::< CacheAligned< [ u8; 63 ] > >(), CACHE_LINE );
  assert_eq!( size % CACHE_LINE, 0, "size {size} is not a whole number of lines" );
  assert!( a.abs_diff( b ) >= CACHE_LINE, "fields are {} bytes apart", a.abs_diff( b ) );
  assert!( a.abs_diff( b ) < CACHE_LINE );
```

The first two commands are one measurement in two halves: eight sites written,
two of them compiled. The gap between those numbers is the whole subject of
[`002`](002_the_constant_across_a_platform_port.md), and neither half means
anything without the other.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AL33 | The port edit list | n/a — observation | Eight sites in `src/lib.rs` write the number and two are compiled — the port's real work is the `CACHE_LINE` declaration and the `#[ repr( align( 64 ) ) ]` attribute — and the six remaining are prose: the module doc's mention of the attribute, the two sentences of `CACHE_LINE`'s own doc comment naming the platform sizes, the power-of-two precondition comment above the compile-time assertion, a doctest assertion on `CACHE_LINE` itself that fails loudly on a port, and an `on_distinct_lines` doctest example that stays true at any line size and so drifts without complaint |
| AL34 | The `#[ repr( align( 64 ) ) ]` attribute | n/a — observation | The duplicated literal is guarded, which [`workaround/001`](../workaround/001_the_alignment_literal_cannot_be_the_constant.md) does not say: `tests/align_test.rs` compares `size_of` against `CACHE_LINE` for several wrapped types, so raising the constant and forgetting the attribute fails the suite rather than shipping. The dent is in the invariant's cleanliness, not in the port's safety |
| AL35 | `into_inner` | n/a — observation | The one transition that ends the guarantee is also the one entry point with no `#[ must_use ]` (recorded at [`api`](../api/readme.md) AL5) — discarding a `CacheAligned` by ignoring the value it unwraps is the single move the crate makes both irreversible and silent |
| AL36 | The value's arc | n/a — observation | Wrap, hold, move and unwrap are all `Copy` moves over a type with no state of its own, so nothing at runtime can observe which stage a value is in. The lifecycle documented here is the reader's model of the guarantee, not a machine's — and it is worth saying, because a lifecycle document usually implies the opposite |
