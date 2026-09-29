# API Doc Definition

### Scope

- **Purpose**: Document the crate's public surface as a caller meets it — three names, one of which is a number.
- **Responsibility**: For each surface, state what a caller can do with it, what it deliberately withholds, and the obligation it leaves with the caller.
- **In Scope**: The reading surface ([`CACHE_LINE`](../type/001_cache_line.md), `on_distinct_lines`); the wrapping surface ([`CacheAligned`](../type/002_cache_aligned.md) and its four associated functions).
- **Out of Scope**: The cursor API built on top, which is [`ring_cursor`](../../../ring_cursor/readme.md)'s; the layout rule behind the wrapper, which is [`algorithm/002`](../algorithm/002_rounding_a_payload_up_to_whole_lines.md).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Reading Surface](001_the_reading_surface.md) | A constant and a predicate — what a caller consults to *check* a layout it already has, acquiring no capability | 🔄 |
| 002 | [The Wrapper Surface](002_the_wrapper_surface.md) | Four associated functions, three of them `const`, and the one obligation the type cannot discharge for its caller | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every callable entry point, and its const-ness --'
command grep -E '^  pub (const )?fn|^pub const fn' ring_align/src/lib.rs
echo '  -- how many of them carry #[ must_use ] --'
command grep -c '^#\[ must_use \]\|^  #\[ must_use \]' ring_align/src/lib.rs
echo '  -- control: the family annotates freely elsewhere --'
command grep -rc 'must_use' ring_types/src/capacity.rs ring_seqno/src/lib.rs
echo '  -- the one function that consumes self and returns the payload --'
command grep 'pub fn into_inner' ring_align/src/lib.rs
```

Live output:

```
  -- every callable entry point, and its const-ness --
  pub const fn new( value : T ) -> Self
  pub const fn get( &self ) -> &T
  pub const fn get_mut( &mut self ) -> &mut T
  pub fn into_inner( self ) -> T
pub const fn on_distinct_lines( a : usize, b : usize ) -> bool
  -- how many of them carry #[ must_use ] --
1
  -- control: the family annotates freely elsewhere --
ring_types/src/capacity.rs:2
ring_seqno/src/lib.rs:5
  -- the one function that consumes self and returns the payload --
  pub fn into_inner( self ) -> T
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AL5 | `into_inner` | n/a — unenforced | Of five entry points, only `on_distinct_lines` carries `#[ must_use ]` — and the one that most needs it is `into_inner`, which consumes `self` and returns the payload, so discarding its result destroys the only copy while compiling clean. `ring_types` and `ring_seqno` annotate freely, so the omission is local rather than a house style |
| AL6 | The wrapper surface | n/a — observation | Three of four associated functions are `const`; `into_inner` is not, and the cause is `E0493` rather than a choice — recorded in [`workaround/002`](../workaround/002_into_inner_cannot_be_const.md) with a measured caller impact of zero |
| AL7 | The reading surface | n/a — observation | `CACHE_LINE` and `on_distinct_lines` grant a caller no capability it did not already have: both let it *check* a layout it holds, and neither can produce one, which is why the crate's whole effect on a program is what the wrapper's attribute does at compile time |
| AL8 | `CacheAligned< T >` | n/a — observation | The type cannot discharge the obligation its own guarantee is about — one wrapper says nothing about two, so the property lives in whatever struct holds a pair, and the API has no way to require that struct exist |
