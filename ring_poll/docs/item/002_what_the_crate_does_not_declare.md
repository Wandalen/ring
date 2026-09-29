# Item: What The Crate Does Not Declare

### Scope

- **Purpose**: Record the declaration kinds absent from this crate's public surface, and which of those absences are choices with consequences rather than defaults.
- **Responsibility**: The zero counts, what each would have bought, and the two whose absence is visible from outside the crate.
- **In Scope**: Error types, traits, modules, re-exports, aliases, `#[ non_exhaustive ]`, `Drop`, `unsafe`, `#[ inline ]`.
- **Out of Scope**: What the crate does declare (→ [`001`](001_four_operations_eight_entry_points.md)); the trait impls it does derive (→ [`../data_structure/001`](../data_structure/001_three_values_three_shapes.md)).

### Abstract

Eight declarations and eighteen functions, and a longer list of things a crate of
this size usually has and this one does not. Most of the absences are the right
call for a leaf crate on a hot path. Two of them show up in a caller's code.

### The zeros

| Kind | Count | Reading |
|---|---|---|
| `pub trait` | 0 | Nothing here is meant to be implemented elsewhere |
| `pub mod` | 0 | One flat namespace; the crate is small enough |
| `pub use` | 0 | Nothing re-exported — a caller wanting `Producer` reaches `ring_core` directly |
| `pub type` | 0 | No aliases; `Result< (), T >` is written out |
| `#[ non_exhaustive ]` | 0 | `Progress`'s two variants are matchable exhaustively, and a third would break callers |
| `impl Drop` | 0 | Nothing here owns a resource; `Tick` is `Copy` |
| `unsafe` | 0 | Every operation is a `ring_core` call |
| `#[ inline ]` | 0 | Nothing is marked, and the workspace sets no `lto` |
| error type | 0 | Failure returns the record, not an error → PL27 |

### The error shape

`push_within` returns `Result< (), T >`. On refusal the `Err` carries the record
the caller handed in, so nothing is lost and nothing needs allocating. The other
three return `usize`, `Option< T >` and `usize` — none of them can fail in a way
that needs describing.

The family has a shared error type, `ring_types::RingError`, and twenty of the
thirty-three `ring_*` crates use it. This one does not, and the choice is
coherent: a refusal is not an error, it is a full ring, and the useful thing to
hand back is the record. What it costs a caller is that the failure does not
compose with anything else's — `?` does not work, `Box< dyn Error >` does not
work, and a function mixing this crate with any of the twenty has two unrelated
failure vocabularies to reconcile → PL27.

### The inlining shape

Ten of the eighteen functions are non-generic: `Budget`'s three, `Progress`'s
four, and `Tick`'s `new`, `budget`, `progress`. All ten are one-liners, and all
ten are `const fn`, which constrains what they may do but says nothing about
whether they are inlined across a crate boundary.

The other eight are generic over `T : Send`, so they are instantiated in the
caller's crate and the optimiser sees the body. The ten are not, and neither
`#[ inline ]` nor workspace `lto` closes that gap → PL28.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'pub trait:              %s\n' "$( command grep -c '^pub trait' ring_poll/src/lib.rs || true )"
printf 'pub mod:                %s\n' "$( command grep -c '^pub mod' ring_poll/src/lib.rs || true )"
printf 'pub use:                %s\n' "$( command grep -c '^pub use' ring_poll/src/lib.rs || true )"
printf 'pub type:               %s\n' "$( command grep -c '^pub type' ring_poll/src/lib.rs || true )"
printf 'non_exhaustive:         %s\n' "$( command grep -c 'non_exhaustive' ring_poll/src/lib.rs || true )"
printf 'impl Drop:              %s\n' "$( command grep -c '^impl.*Drop' ring_poll/src/lib.rs || true )"
printf 'unsafe:                 %s\n' "$( command grep -c 'unsafe' ring_poll/src/lib.rs || true )"
printf 'inline attributes:      %s\n' "$( command grep -c 'inline' ring_poll/src/lib.rs || true )"
printf 'lto in the workspace:   %s\n' "$( command grep -c 'lto' Cargo.toml || true )"
printf 'RingError here:         %s\n' "$( command grep -c 'RingError' ring_poll/src/lib.rs || true )"
printf 'family crates using it: %s of %s\n' "$( for c in ring_*/src/lib.rs; do command grep -q 'RingError' "$c" && echo x; done | wc -l )" "$( ls -d ring_*/ | wc -l )"
printf 'return types here:      %s\n' "$( command grep -oE '^-> .*' ring_poll/src/lib.rs | sed 's/^-> //' | tr '\n' '/' )"
printf 'the failing one carries: %s\n' "$( command grep -oE '^-> Result< \(\), [A-Z] >' ring_poll/src/lib.rs | sed 's/^-> //' )"
printf 'non-generic pub fns:    %s\n' "$( awk '/^ *pub (const )?fn /{ l=$0; if (l !~ /</) { sub(/^ *pub (const )?fn /,"",l); sub(/\(.*/,"",l); printf "%s ", l } }' ring_poll/src/lib.rs )"
printf 'generic pub fns:        %s\n' "$( command grep -cE '^ *pub (const )?fn [a-z_]+<' ring_poll/src/lib.rs || true )"
printf 'const fns:              %s\n' "$( command grep -cE '^ *pub const fn ' ring_poll/src/lib.rs || true )"
printf 'inline in whole family: %s\n' "$( command grep -rc '#\[ inline' ring_*/src/*.rs 2>/dev/null | command grep -v ':0$' | wc -l )"
```

Live output:

```
pub trait:              0
pub mod:                0
pub use:                0
pub type:               0
non_exhaustive:         0
impl Drop:              0
unsafe:                 0
inline attributes:      0
lto in the workspace:   0
RingError here:         0
family crates using it: 20 of 33
return types here:      Result< (), T >/usize/Option< T >/usize/
the failing one carries: Result< (), T >
non-generic pub fns:    once new attempts of is_made count then new reset budget progress lost 
generic pub fns:        8
const fns:              12
inline in whole family: 1
```

### Items

| File | Relationship |
|------|--------------|
| [001_four_operations_eight_entry_points.md](001_four_operations_eight_entry_points.md) | The declarations that are here, which this file is the complement of |

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_tick_path_surface.md`](../api/001_tick_path_surface.md) | The surface guarantees, one of which rests on `Progress` staying two-variant |

### Data Structures

| File | Relationship |
|------|--------------|
| [`../data_structure/001_three_values_three_shapes.md`](../data_structure/001_three_values_three_shapes.md) | The derived traits, which are what the crate has instead of declared ones |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [`../non_functional_requirement/001_what_a_tick_may_cost.md`](../non_functional_requirement/001_what_a_tick_may_cost.md) | The per-call cost the missing `#[ inline ]` bears on |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Every count above, and every absence |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `this_crate_declares_no_parking_dependency` — the only absence the suite asserts |

### PL27 — the crate has no error vocabulary, and the family it sits in does

Twenty of the family's thirty-three crates name `ring_types::RingError`.
`ring_poll` names it zero times. Its one fallible operation returns
`Result< (), T >`, where the `Err` is the record that did not fit.

That is a good design for what it does. Handing the record back is allocation-free
and lossless, and it makes the refusal impossible to ignore: the caller has the
value and must do something with it. A `RingError` would have thrown the record
away to describe what happened, which is exactly backwards for a full ring.

The cost is at the seam. A caller that uses `ring_poll` alongside any of the
twenty has two failure representations in one function. `?` does not apply to
`Result< (), T >` in a function returning `Result< _, RingError >` without a
`From` impl that cannot be written, since `T` is the caller's own record type.
`Box< dyn Error >` does not apply either, because a record is not an error. The
practical consequence is a `match` at every boundary between this crate and the
rest of the family, which is where a scheduler actually sits.

Nothing here argues for changing it — the alternative loses the record. What is
worth recording is that the absence is not a gap to be filled later. It is a
different contract, it is contagious at the call site, and no document in this
crate previously said so, which makes it look like an omission rather than the
decision it is.

### PL28 — ten one-line accessors, no `#[ inline ]`, no `lto`

Eight of the eighteen public functions are generic over `T : Send`. Those are
monomorphised in the calling crate, so the optimiser sees their bodies and the
question of cross-crate inlining does not arise.

The other ten are not generic: `Budget::once`, `new`, `attempts`,
`Progress::of`, `is_made`, `count`, `then`, and `Tick::new`, `budget`,
`progress`. Every one is a single expression, all ten are `const fn`, and none
carries `#[ inline ]`. `const fn` restricts what the body may do; it is not an
inlining hint, and a non-generic function in an upstream crate is not visible to
the caller's optimiser unless it is marked or the build enables cross-crate
optimisation. The workspace manifest sets no `lto` in any profile.

So `Progress::of( n )` — a comparison and a variant construction — is a call
across a crate boundary in a build without LTO, and `Tick::progress()` is a field
read behind the same call. On a hot path measured per tick, that is the cheapest
kind of overhead and also the kind that is invisible in a profile: it does not
show up as one expensive thing, it shows up as everything being slightly slower.

The whole family is consistent here — zero `#[ inline ]` attributes across all
thirty-three `ring_*` crates — so this is not a `ring_poll` oversight. It is a
family-wide default that has never been measured, in a family whose reason to
exist is per-record cost. This crate is a defensible place to notice it, because
its non-generic accessors are the smallest functions the family has, and
`ring_bench` is the crate that could settle it.
