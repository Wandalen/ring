# Data Structure Doc Definition

### Scope

- **Purpose**: Document the crate's two shapes — a wrapper whose only content is its own layout, and the two-field arrangement that is what the family actually builds out of it.
- **Responsibility**: For each, give the memory layout, the sizes it produces, and the property that depends on the shape rather than on any code.
- **In Scope**: `CacheAligned<T>`'s in-memory form; a padded pair inside one struct.
- **Out of Scope**: The accessor surface, which is [`api/002`](../api/002_the_wrapper_surface.md); the cursor payloads, which are [`ring_cursor`](../../../ring_cursor/readme.md)'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The `CacheAligned` Wrapper](001_the_cache_aligned_wrapper.md) | A structure with one field and no fields — the padding is not stored, it is the difference between size and payload | 🔄 |
| 002 | [A Padded Pair in One Struct](002_a_padded_pair_in_one_struct.md) | The two-field arrangement the guarantee is about, and how holding both halves in one type discharges the obligation the wrapper cannot | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the declared shape --'
command grep -E '^#\[ derive|^#\[ repr|^pub struct CacheAligned' ring_align/src/lib.rs
echo '  -- sizes the attribute produces, measured --'
cat > ./-ds_probe.rs <<'EOF'
#[ repr( align( 64 ) ) ]
struct CacheAligned< T >( #[ allow( dead_code ) ] T );
#[ repr( align( 128 ) ) ]
struct Wide( #[ allow( dead_code ) ] u64 );
fn main()
{
  use core::mem::{ align_of, size_of };
  for ( name, a, s ) in [
    ( "()      ", align_of::< CacheAligned< () > >(), size_of::< CacheAligned< () > >() ),
    ( "u8      ", align_of::< CacheAligned< u8 > >(), size_of::< CacheAligned< u8 > >() ),
    ( "[u8; 63]", align_of::< CacheAligned< [ u8; 63 ] > >(), size_of::< CacheAligned< [ u8; 63 ] > >() ),
    ( "[u8; 65]", align_of::< CacheAligned< [ u8; 65 ] > >(), size_of::< CacheAligned< [ u8; 65 ] > >() ),
    ( "Wide    ", align_of::< CacheAligned< Wide > >(), size_of::< CacheAligned< Wide > >() ),
  ] { println!( "    {name}  align {a:>3}  size {s:>4}" ); }
}
EOF
rustc --crate-name ds_probe --edition 2021 -o ./-ds_probe ./-ds_probe.rs 2>/dev/null && ./-ds_probe
rm -f ./-ds_probe ./-ds_probe.rs
```

Live output:

```
  -- the declared shape --
#[ derive( Debug, Clone, Copy, Default, PartialEq, Eq ) ]
#[ repr( align( 64 ) ) ]
pub struct CacheAligned< T >( T );
  -- sizes the attribute produces, measured --
    ()        align  64  size    0
    u8        align  64  size   64
    [u8; 63]  align  64  size   64
    [u8; 65]  align  64  size  128
    Wide      align 128  size  128
```

The probe replicates the declaration rather than importing it, for the reason
[`workaround/001`](../workaround/001_the_alignment_literal_cannot_be_the_constant.md)
compiles its own three lines: what is under test is what the *attribute* does,
and a probe that imports the crate would be testing the crate's re-export path
as well.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AL9 | `CacheAligned< () >` | **misleading doc** | `src/lib.rs:39-41` says the size is a multiple of `CACHE_LINE` "for any `T` that fits, **so** two of them in one struct are guaranteed to land on different lines" — and for a zero-sized payload the premise holds while the conclusion fails: size is 0, which is a multiple of 64, and two such fields sit at the same address. The stated implication is not the theorem the type actually has |
| AL10 | `CacheAligned< T >` | n/a — observation | Alignment is `max( 64, align_of::< T > () )`, not 64 — a payload declared `#[ repr( align( 128 ) ) ]` produces a 128-aligned wrapper. Harmless here, since every value above 64 preserves the separation property, but it means `align_of` is not a constant of this type the way the doctest's `u64` case suggests |
| AL11 | The wrapper | n/a — observation | The padding is not a field and is not stored: the struct has one member and the difference between `size_of` and `size_of::< T >()` is what the attribute instructed the compiler to leave empty, which is why `Debug` prints only the payload and `PartialEq` compares only the payload |
| AL12 | `ring_cursor::CursorPair` | n/a — observation | Exactly one struct in the family holds two padded fields — `CursorPair { producer : PaddedCursor, consumer : PaddedCursor, capacity : Capacity }` — and it carries a third, unpadded one. `capacity` needs no line of its own for a reason this crate supplies rather than `ring_cursor`: the two 64-aligned fields ahead of it push it past both their lines whatever order the compiler picks, so its separation is a consequence of the wrapper and not a decision anybody made |
