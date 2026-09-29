# Type Doc Definition

### Scope

- **Purpose**: Document the two names this crate exports as design decisions — a number, and a wrapper that is nothing but layout.
- **Responsibility**: For each, state what it is, what shape it takes and why, and what a consumer acquires by naming it.
- **In Scope**: [`CACHE_LINE`](001_cache_line.md); [`CacheAligned<T>`](002_cache_aligned.md).
- **Out of Scope**: The free function `on_distinct_lines`, which is a predicate over addresses rather than a Domain Type — cataloged in [`item/002`](../item/002_on_distinct_lines.md); the cursor types that wear the wrapper, which are [`ring_cursor`](../../../ring_cursor/readme.md)'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [`CACHE_LINE`](001_cache_line.md) | One number, owned in one place, whose wrongness is silent — the crate's whole reason to exist as a crate rather than a line of code | 🔄 |
| 002 | [`CacheAligned<T>`](002_cache_aligned.md) | A newtype carrying no data of its own: the padding *is* the type, and transparency to the payload is the rest of the contract | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two exported names, as declared --'
command grep -E '^pub const CACHE_LINE|^pub struct CacheAligned' ring_align/src/lib.rs
echo '  -- the wrapper accepts any payload: no bound on the impl --'
command grep '^impl' ring_align/src/lib.rs
echo '  -- the family does newtype its other numbers --'
command grep -r '^pub struct Capacity\|^pub struct Budget\|^pub struct SlotIndex' ring_*/src/*.rs
echo '  -- everything this crate exports at top level --'
command grep -E '^pub (const|struct|fn) ' ring_align/src/lib.rs
```

Live output:

```
  -- the two exported names, as declared --
pub const CACHE_LINE : usize = 64;
pub struct CacheAligned< T >( T );
  -- the wrapper accepts any payload: no bound on the impl --
impl< T > CacheAligned< T >
  -- the family does newtype its other numbers --
ring_poll/src/lib.rs:pub struct Budget( usize );
ring_types/src/capacity.rs:pub struct Capacity( usize );
ring_types/src/id.rs:pub struct SlotIndex( pub usize );
  -- everything this crate exports at top level --
pub const CACHE_LINE : usize = 64;
pub struct CacheAligned< T >( T );
pub const fn on_distinct_lines( a : usize, b : usize ) -> bool
```

The last command is the definition's own boundary check: this crate exports
three names, two of which are Domain Types documented here and one of which is
a predicate documented in [`item/002`](../item/002_on_distinct_lines.md). If a
fourth ever appears, this block prints it before anybody notices it is
undocumented.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AL45 | `CACHE_LINE` | n/a — inconsistency | It is a bare `usize`, while the family newtypes `Capacity`, `Budget` and `SlotIndex` — all three of them plain counts with less at stake. The one number whose misuse is silent is the one left as a primitive, which is what lets `claim.abs_diff( consume ) >= 64` typecheck at `ring_mpsc/src/lib.rs:865` |
| AL46 | `CacheAligned< T >` | n/a — observation | One `impl` block, no bound on `T` — so every payload is accepted including the zero-sized one whose behaviour contradicts the module doc (recorded at [`data_structure`](../data_structure/readme.md) AL9). A `T` that could not be zero-sized is not expressible as a bound, so the gap is the language's rather than the design's, and it is the reason AL9 is a doc finding and not a type finding |
| AL47 | The export surface | n/a — observation | Three names at top level and no module tree — the crate's entire published vocabulary fits in one grep, which is the property that makes it a tier-1 crate rather than a utility drawer. Two are Domain Types and one is a predicate, and the split is why this definition and [`item/`](../item/readme.md) do not overlap |
| AL48 | `CACHE_LINE` as `usize` | n/a — observation | The primitive form is what makes the constant usable in `#[ repr( align( … ) ) ]`'s neighbourhood at all — a newtype could not be substituted there either (see [`workaround/001`](../workaround/001_the_alignment_literal_cannot_be_the_constant.md)), so wrapping it would buy call-site safety and cost nothing at the attribute, which already cannot use it |
