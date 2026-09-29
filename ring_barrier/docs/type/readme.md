# type

One public type, one field, three derives. Both instances are about what is
*not* declared: the width the crate never converts, and the six traits it does
not have — five of which it could not have, one of which it chose not to.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [A `u64` Distance and a `usize` Headroom](001_a_u64_distance_and_a_usize_headroom.md) | What each width measures, and BR21 — the only crate in the family that casts between them zero times |
| 002 | [The Traits Derived and the Traits Absent](002_the_traits_derived_and_the_traits_absent.md) | Three derives, two auto traits, five impossible absences, and one declined |

### The Declaration, In Full

```rust
// ring_barrier/src/lib.rs:80-84
#[ derive( Debug, Clone, Copy ) ]
pub struct Barrier< 'a >
{
  dependencies : &'a [ PaddedCursor ],
}
```

| | Count | |
|--|------:|--|
| Public types | 1 | `Barrier< 'a >` |
| Fields | 1 | private |
| Derives | 3 | `Debug`, `Clone`, `Copy` |
| Auto traits | 2 | `Send`, `Sync` — stated nowhere |
| Hand-written `impl`s | 0 | only the inherent block, at `:86` |
| Integer-width casts | **0** | in 13 other source files, 38 |

### Regenerate Both Censuses

```sh
cd "$(git rev-parse --show-toplevel)"

# widths, by what they measure
grep -vE '^\s*(///|//!|//)' ring_barrier/src/lib.rs | grep -nE 'usize|u64'

# casts, per source file — ring_barrier is absent from the output
for f in ring_*/src/*.rs; do
  n=$( grep -vE '^\s*(///|//!|//)' "$f" | grep -cE 'as usize|as u64' )
  # the trailing `true` keeps a final zero from failing the whole block
  [ "$n" != 0 ] && echo "$n  $f"
  true
done
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BR21 | family | n/a — observation | 13 source files carry 38 casts between `usize` and `u64`; `ring_barrier` carries none — and the family's six *bare* narrowings are all in the crates on the route that does not use a `Barrier` |
| BR48 | `ring_barrier` | n/a — observation | `available` returns `u64` and `len` returns `usize` and the crate casts nowhere, because the two counts never meet in an expression — a future method relating sequences to dependencies would introduce the family's thirty-ninth cast into the one crate whose type story is that it has none |
| BR49 | `ring_barrier` | n/a — observation | There is no `Default`, so the empty barrier — the input class the crate documents most — is written `Barrier::over( &[] )` four times instead; a `Default` would be sound but would advertise a `'static` borrow unlike every other way of building the type |
